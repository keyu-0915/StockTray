use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::UpdaterExt;
use tokio::time::sleep;

mod config;
mod models;
mod portfolio;
mod quotes;
mod state;
mod tray;
mod windowing;

use config::*;
use models::*;
use portfolio::compute_daily_pnl;
use quotes::{fetch_quotes, fetch_stock_name, normalize_code};
use state::SharedState;
use tray::{create_tray, update_tray_status};
use windowing::{emit_state_to_windows, hide_popup_window, set_popup_hovered_state};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(SharedState::new(load_config(), config_path()))
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_settings,
            add_stock,
            refresh_quotes,
            hide_popup,
            set_popup_hovered,
            check_and_install_update
        ])
        .on_window_event(|window, event| {
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            create_tray(&handle)?;
            let refresh_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = refresh_quotes_inner(&refresh_handle).await {
                    eprintln!("initial refresh failed: {err}");
                }
            });
            let auto_refresh_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                auto_refresh_loop(auto_refresh_handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run 韭菜托盘");
}

#[tauri::command]
fn get_state(state: State<SharedState>) -> Result<AppStatePayload, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    Ok(AppStatePayload {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        config: guard.config.clone(),
        summary: guard.summary.clone(),
        last_refreshed_at: guard.last_refreshed_at.clone(),
        last_error: guard.last_error.clone(),
    })
}

#[tauri::command]
fn save_settings(
    config: AppConfig,
    app: AppHandle,
    state: State<SharedState>,
) -> Result<AppStatePayload, String> {
    let payload = {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let normalized = normalize_config(config);
        save_config_to(&guard.config_path, &normalized)?;
        guard.config = normalized;
        AppStatePayload {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            config: guard.config.clone(),
            summary: guard.summary.clone(),
            last_refreshed_at: guard.last_refreshed_at.clone(),
            last_error: guard.last_error.clone(),
        }
    };
    if let Some(summary) = payload.summary.as_ref() {
        update_tray_status(&app, summary);
    }
    Ok(payload)
}

#[tauri::command]
async fn add_stock(
    code: String,
    holdings: f32,
    cost_price: Option<f32>,
    state: State<'_, SharedState>,
) -> Result<AppConfig, String> {
    let normalized = normalize_code(&code).ok_or_else(|| "请输入有效的 A 股代码".to_string())?;
    {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        if guard.config.stocks.iter().any(|s| s.code == normalized) {
            return Err("该股票已经存在".into());
        }
    }

    let quote = fetch_quotes(std::slice::from_ref(&normalized))
        .await
        .ok()
        .and_then(|mut quotes| quotes.pop())
        .filter(|quote| quote.error.is_empty());
    let name = if let Some(name) = quote
        .as_ref()
        .filter(|quote| !quote.name.is_empty())
        .map(|quote| quote.name.clone())
    {
        name
    } else {
        fetch_stock_name(&normalized)
            .await
            .unwrap_or_else(|_| normalized.clone())
    };
    let initial_cost = cost_price.unwrap_or_else(|| {
        quote
            .as_ref()
            .map(|quote| quote.price)
            .filter(|price| *price != 0.0)
            .unwrap_or(0.0)
    });
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let show_in_tooltip = !guard.config.stocks.iter().any(|s| s.show_in_tooltip);
    guard.config.stocks.push(StockEntry {
        code: normalized,
        name,
        holdings: normalize_holding(holdings),
        cost_price: normalize_cost_price(initial_cost),
        show_in_popup: true,
        show_in_tooltip,
    });
    save_config_to(&guard.config_path, &guard.config)?;
    Ok(guard.config.clone())
}

#[tauri::command]
async fn refresh_quotes(app: AppHandle) -> Result<DailySummary, String> {
    refresh_quotes_inner(&app).await
}

#[tauri::command]
fn hide_popup(app: AppHandle) -> Result<(), String> {
    hide_popup_window(&app)
}

#[tauri::command]
fn set_popup_hovered(app: AppHandle, hovered: bool) -> Result<(), String> {
    set_popup_hovered_state(&app, hovered)
}

pub(crate) async fn refresh_quotes_inner(app: &AppHandle) -> Result<DailySummary, String> {
    let state = app.state::<SharedState>();
    let config = {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        guard.config.clone()
    };
    let codes = config
        .stocks
        .iter()
        .map(|s| s.code.clone())
        .collect::<Vec<_>>();
    let quotes = match fetch_quotes(&codes).await {
        Ok(quotes) => quotes,
        Err(err) => {
            record_refresh_error(app, &err);
            return Err(err);
        }
    };
    let summary = compute_daily_pnl(&quotes, &config);

    {
        let state = app.state::<SharedState>();
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        guard.summary = Some(summary.clone());
        guard.last_refreshed_at = Some(chrono::Local::now().format("%H:%M:%S").to_string());
        guard.last_error = None;
        let mut changed = false;
        for quote in &quotes {
            if quote.error.is_empty() && !quote.name.is_empty() {
                if let Some(stock) = guard
                    .config
                    .stocks
                    .iter_mut()
                    .find(|s| s.code == quote.code)
                {
                    if stock.name != quote.name {
                        stock.name = quote.name.clone();
                        changed = true;
                    }
                }
            }
        }
        if changed {
            save_config_to(&guard.config_path, &guard.config)?;
        }
    }

    emit_state_to_windows(app);
    update_tray_status(app, &summary);
    Ok(summary)
}

#[derive(Debug, Clone, Serialize)]
struct UpdateCheckResult {
    available: bool,
    current_version: String,
    version: Option<String>,
}

#[tauri::command]
async fn check_and_install_update(app: AppHandle) -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(update) = update {
        let version = update.version.to_string();
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
        Ok(UpdateCheckResult {
            available: true,
            current_version,
            version: Some(version),
        })
    } else {
        Ok(UpdateCheckResult {
            available: false,
            current_version,
            version: None,
        })
    }
}

fn record_refresh_error(app: &AppHandle, err: &str) {
    let state = app.state::<SharedState>();
    let result = state.0.lock();
    if let Ok(mut guard) = result {
        guard.last_error = Some(err.to_string());
    }
    emit_state_to_windows(app);
}

async fn auto_refresh_loop(app: AppHandle) {
    let mut elapsed_ms = 0u32;
    loop {
        let interval_ms = current_refresh_interval_ms(&app);
        if interval_ms == 0 {
            elapsed_ms = 0;
            sleep(Duration::from_secs(1)).await;
            continue;
        }

        let sleep_ms = interval_ms.min(1_000);
        sleep(Duration::from_millis(sleep_ms as u64)).await;

        if current_refresh_interval_ms(&app) != interval_ms {
            elapsed_ms = 0;
            continue;
        }

        elapsed_ms = elapsed_ms.saturating_add(sleep_ms);
        if elapsed_ms < interval_ms {
            continue;
        }

        elapsed_ms = 0;
        if let Err(err) = refresh_quotes_inner(&app).await {
            eprintln!("auto refresh failed: {err}");
        }
    }
}

fn current_refresh_interval_ms(app: &AppHandle) -> u32 {
    if is_frontmost_surface_visible(app) || is_tray_hovered(app) {
        1_000
    } else {
        configured_background_refresh_interval_ms(app)
    }
}

fn configured_background_refresh_interval_ms(app: &AppHandle) -> u32 {
    let state = app.state::<SharedState>();
    state
        .0
        .lock()
        .map(|guard| guard.config.background_refresh_interval_ms)
        .unwrap_or_else(|_| default_background_refresh_interval_ms())
}

fn is_tray_hovered(app: &AppHandle) -> bool {
    let state = app.state::<SharedState>();
    state
        .0
        .lock()
        .map(|guard| guard.tray_hovered)
        .unwrap_or(false)
}

fn is_frontmost_surface_visible(app: &AppHandle) -> bool {
    ["popup", "settings"].iter().any(|label| {
        app.get_webview_window(label)
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false)
    })
}

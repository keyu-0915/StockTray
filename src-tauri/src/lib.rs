use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::UpdaterExt;
use tokio::time::{sleep, timeout};

mod config;
mod market;
mod models;
mod portfolio;
mod quotes;
mod state;
mod tray;
mod windowing;

use config::*;
use models::*;
use portfolio::compute_daily_pnl;
use quotes::{
    fetch_index_quotes, fetch_quotes, fetch_quotes_detailed, fetch_stock_name, normalize_code,
};
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
            refresh_market_analysis,
            hide_popup,
            set_popup_hovered,
            control_settings_window,
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
            #[cfg(debug_assertions)]
            windowing::show_window(&handle, "settings");
            let refresh_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = refresh_quotes_inner(&refresh_handle).await {
                    eprintln!("initial refresh failed: {err}");
                }
                #[cfg(debug_assertions)]
                windowing::toggle_popup(&refresh_handle, None);
            });
            let auto_refresh_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                auto_refresh_loop(auto_refresh_handle).await;
            });
            let market_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let enabled = market_handle
                    .state::<SharedState>()
                    .0
                    .lock()
                    .map(|guard| guard.config.market_analysis.enabled)
                    .unwrap_or(false);
                if enabled && is_market_trading_window() {
                    if let Err(err) = refresh_market_analysis_inner(&market_handle).await {
                        eprintln!("initial market analysis failed: {err}");
                    }
                }
                market_refresh_loop(market_handle).await;
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
        market: guard.market.clone(),
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
            market: guard.market.clone(),
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
async fn refresh_market_analysis(app: AppHandle) -> Result<MarketSnapshot, String> {
    let enabled = app
        .state::<SharedState>()
        .0
        .lock()
        .map_err(|error| error.to_string())?
        .config
        .market_analysis
        .enabled;
    if !enabled {
        return Err("市场风格分析已关闭".to_string());
    }
    refresh_market_analysis_inner(&app).await
}

async fn refresh_market_analysis_inner(app: &AppHandle) -> Result<MarketSnapshot, String> {
    let requested_at = Instant::now();
    let state = app.state::<SharedState>();
    // ponytail: one global flight is enough for a desktop client; split by account only if
    // multiple independent market profiles are ever introduced.
    let _flight = state.1.lock().await;
    let cached = {
        let guard = state.0.lock().map_err(|error| error.to_string())?;
        guard
            .market_refresh_finished_at
            .zip(guard.market_refresh_result.clone())
    };
    if let Some(result) = cached
        .filter(|(finished_at, _)| should_reuse_market_refresh(requested_at, *finished_at))
        .map(|(_, result)| result)
    {
        return result;
    }
    let result = timeout(Duration::from_secs(30), refresh_market_analysis_once(app))
        .await
        .map_err(|_| "市场分析刷新超过30秒，已取消".to_string())
        .and_then(|result| result);
    if let Ok(mut guard) = state.0.lock() {
        guard.market_refresh_finished_at = Some(Instant::now());
        guard.market_refresh_result = Some(result.clone());
        if let Err(error) = &result {
            guard.market.last_error = Some(error.clone());
        }
    }
    if result.is_err() {
        emit_state_to_windows(app);
    }
    result
}

fn should_reuse_market_refresh(requested_at: Instant, finished_at: Instant) -> bool {
    finished_at >= requested_at
}

async fn refresh_market_analysis_once(app: &AppHandle) -> Result<MarketSnapshot, String> {
    let state = app.state::<SharedState>();
    let needs_universe = state
        .0
        .lock()
        .map_err(|error| error.to_string())?
        .market_engine
        .members
        .is_empty();
    if needs_universe {
        let mut engine = market::MarketEngine::default();
        if let Err(error) = engine.ensure_universe().await {
            if let Ok(mut guard) = state.0.lock() {
                guard.market.last_error = Some(error.clone());
            }
            emit_state_to_windows(app);
            return Err(error);
        }
        let mut guard = state.0.lock().map_err(|error| error.to_string())?;
        if guard.market_engine.members.is_empty() {
            guard.market_engine = engine;
        }
    }
    let (codes, refresh_minutes) = {
        let guard = state.0.lock().map_err(|error| error.to_string())?;
        (
            guard.market_engine.codes(),
            guard.config.market_analysis.refresh_minutes,
        )
    };
    let mut fetched = fetch_quotes_detailed(&codes).await?;
    match fetch_index_quotes(&market::index_secids()).await {
        Ok(quotes) => fetched.index_quotes = quotes,
        Err(error) => fetched.index_error = error,
    }
    let persisted = {
        let mut guard = state.0.lock().map_err(|error| error.to_string())?;
        let previous = guard.market.current.clone();
        let mut engine = std::mem::take(&mut guard.market_engine);
        let snapshot = engine.analyze(fetched, previous.as_ref(), refresh_minutes);
        guard.market_engine = engine;
        let boundary_changed = previous.as_ref().is_some_and(|old| {
            old.trading_date != snapshot.trading_date
                || old.quality.sample_source != snapshot.quality.sample_source
        }) || guard.market.sample_version != market::SAMPLE_VERSION
            || guard.market.algorithm_version != market::ALGORITHM_VERSION;
        if boundary_changed {
            guard.market.history.clear();
        }
        if snapshot.quality.minimum_style_coverage >= 80.0 {
            let evidence = MarketEvidence {
                time: snapshot.time.clone(),
                leader: snapshot.leader.clone(),
                scores: snapshot.styles.iter().map(|style| style.score).collect(),
            };
            if let Some(last) = guard
                .market
                .history
                .last_mut()
                .filter(|item| item.time.get(..5) == evidence.time.get(..5))
            {
                *last = evidence;
            } else {
                guard.market.history.push(evidence);
                if guard.market.history.len() > 64 {
                    guard.market.history.remove(0);
                }
            }
        }
        guard.market.universe_size = codes.len();
        guard.market.sample_version = market::SAMPLE_VERSION.into();
        guard.market.algorithm_version = market::ALGORITHM_VERSION.into();
        guard.market.last_error = None;
        guard.market.current = Some(snapshot.clone());
        (snapshot, guard.market.clone())
    };
    save_market_state(&persisted.1)?;
    emit_state_to_windows(app);
    Ok(persisted.0)
}

#[tauri::command]
fn hide_popup(app: AppHandle) -> Result<(), String> {
    hide_popup_window(&app)
}

#[tauri::command]
fn set_popup_hovered(app: AppHandle, hovered: bool) -> Result<(), String> {
    set_popup_hovered_state(&app, hovered)
}

#[tauri::command]
fn control_settings_window(app: AppHandle, action: &str) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window not found".to_string())?;
    match action {
        "drag" => window.start_dragging(),
        "minimize" => window.minimize(),
        "toggle-maximize" if window.is_maximized().map_err(|error| error.to_string())? => {
            window.unmaximize()
        }
        "toggle-maximize" => window.maximize(),
        "close" => window.hide(),
        _ => return Err(format!("unsupported window action: {action}")),
    }
    .map_err(|error| error.to_string())
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
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart()
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

async fn market_refresh_loop(app: AppHandle) {
    loop {
        sleep(Duration::from_secs(30)).await;
        expire_runtime_market_state(&app);
        let (enabled, minutes, last_finished) = app
            .state::<SharedState>()
            .0
            .lock()
            .map(|guard| {
                (
                    guard.config.market_analysis.enabled,
                    guard.config.market_analysis.refresh_minutes,
                    guard.market_refresh_finished_at,
                )
            })
            .unwrap_or((false, 15, None));
        if !market_refresh_due(
            enabled,
            is_market_trading_window(),
            last_finished.map(|finished| finished.elapsed()),
            minutes,
        ) {
            continue;
        }
        if let Err(error) = refresh_market_analysis_inner(&app).await {
            eprintln!("market analysis refresh failed: {error}");
        }
    }
}

fn market_refresh_due(
    enabled: bool,
    in_trading_window: bool,
    elapsed: Option<Duration>,
    minutes: u32,
) -> bool {
    enabled
        && in_trading_window
        && elapsed.is_none_or(|elapsed| elapsed >= Duration::from_secs(minutes as u64 * 60))
}

fn expire_runtime_market_state(app: &AppHandle) {
    use chrono::{Datelike, Local};

    let now = Local::now();
    let persisted = app
        .state::<SharedState>()
        .0
        .lock()
        .ok()
        .and_then(|mut guard| {
            expire_market_state_if_needed(
                &mut guard.market,
                &now.format("%Y-%m-%d").to_string(),
                now.weekday().number_from_monday() <= 5,
            )
            .then(|| guard.market.clone())
        });
    if let Some(state) = persisted {
        if let Err(error) = save_market_state(&state) {
            eprintln!("expired market state save failed: {error}");
        }
        emit_state_to_windows(app);
    }
}

fn is_market_trading_window() -> bool {
    use chrono::{Datelike, Timelike};
    let now = chrono::Local::now();
    if now.weekday().number_from_monday() > 5 {
        return false;
    }
    let minute = now.hour() * 60 + now.minute();
    (570..=691).contains(&minute) || (780..=906).contains(&minute)
}

fn current_refresh_interval_ms(app: &AppHandle) -> u32 {
    let configured = configured_background_refresh_interval_ms(app);
    if configured == 0 {
        0
    } else if is_frontmost_surface_visible(app) || is_tray_hovered(app) {
        1_000
    } else {
        configured
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_waiters_reuse_a_flight_that_finished_after_their_request() {
        let requested = Instant::now();
        assert!(should_reuse_market_refresh(
            requested,
            requested + Duration::from_millis(1)
        ));
        assert!(!should_reuse_market_refresh(
            requested,
            requested - Duration::from_millis(1)
        ));
    }

    #[test]
    fn market_schedule_respects_enabled_window_and_configured_minutes() {
        assert!(!market_refresh_due(true, false, None, 5));
        assert!(!market_refresh_due(
            true,
            true,
            Some(Duration::from_secs(299)),
            5
        ));
        assert!(market_refresh_due(
            true,
            true,
            Some(Duration::from_secs(300)),
            5
        ));
        assert!(!market_refresh_due(false, true, None, 30));
    }
}

use std::{path::PathBuf, sync::Mutex, time::Instant};

use tauri::{AppHandle, Manager};
use tokio::sync::Mutex as AsyncMutex;

use crate::market::MarketEngine;
use crate::{
    config::load_market_state,
    models::{AppConfig, AppStatePayload, DailySummary, MarketAnalysisState, MarketSnapshot},
};

pub(crate) struct RuntimeState {
    pub(crate) config: AppConfig,
    pub(crate) summary: Option<DailySummary>,
    pub(crate) last_refreshed_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) market: MarketAnalysisState,
    pub(crate) market_engine: MarketEngine,
    pub(crate) config_path: PathBuf,
    pub(crate) popup_token: u64,
    pub(crate) popup_hovered: bool,
    pub(crate) popup_hide_pending: bool,
    pub(crate) tray_hovered: bool,
    pub(crate) last_tray_click_at: Option<Instant>,
    pub(crate) market_refresh_finished_at: Option<Instant>,
    pub(crate) market_refresh_result: Option<Result<MarketSnapshot, String>>,
}

pub(crate) struct SharedState(pub(crate) Mutex<RuntimeState>, pub(crate) AsyncMutex<()>);

impl SharedState {
    pub(crate) fn new(config: AppConfig, config_path: PathBuf) -> Self {
        Self(
            Mutex::new(RuntimeState {
                config,
                summary: None,
                last_refreshed_at: None,
                last_error: None,
                market: load_market_state(),
                market_engine: MarketEngine::default(),
                config_path,
                popup_token: 0,
                popup_hovered: false,
                popup_hide_pending: false,
                tray_hovered: false,
                last_tray_click_at: None,
                market_refresh_finished_at: None,
                market_refresh_result: None,
            }),
            AsyncMutex::new(()),
        )
    }
}

pub(crate) fn current_payload(app: &AppHandle) -> Option<AppStatePayload> {
    let state = app.state::<SharedState>();
    let guard = state.0.lock().ok()?;
    Some(AppStatePayload {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        config: guard.config.clone(),
        summary: guard.summary.clone(),
        last_refreshed_at: guard.last_refreshed_at.clone(),
        last_error: guard.last_error.clone(),
        market: guard.market.clone(),
    })
}

use std::{path::PathBuf, sync::Mutex};

use tauri::{AppHandle, Manager};

use crate::models::{AppConfig, AppStatePayload, DailySummary};

pub(crate) struct RuntimeState {
    pub(crate) config: AppConfig,
    pub(crate) summary: Option<DailySummary>,
    pub(crate) last_refreshed_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) config_path: PathBuf,
    pub(crate) popup_token: u64,
    pub(crate) popup_hovered: bool,
    pub(crate) popup_hide_pending: bool,
    pub(crate) tray_hovered: bool,
}

pub(crate) struct SharedState(pub(crate) Mutex<RuntimeState>);

impl SharedState {
    pub(crate) fn new(config: AppConfig, config_path: PathBuf) -> Self {
        Self(Mutex::new(RuntimeState {
            config,
            summary: None,
            last_refreshed_at: None,
            last_error: None,
            config_path,
            popup_token: 0,
            popup_hovered: false,
            popup_hide_pending: false,
            tray_hovered: false,
        }))
    }
}

pub(crate) fn current_payload(app: &AppHandle) -> Option<AppStatePayload> {
    let state = app.state::<SharedState>();
    let guard = state.0.lock().ok()?;
    Some(AppStatePayload {
        config: guard.config.clone(),
        summary: guard.summary.clone(),
        last_refreshed_at: guard.last_refreshed_at.clone(),
        last_error: guard.last_error.clone(),
    })
}

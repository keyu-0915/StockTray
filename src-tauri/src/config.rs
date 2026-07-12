use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{Datelike, Local};

use crate::models::{
    default_background_refresh_interval_ms, default_display_fields, default_theme,
    default_tooltip_fields, AppConfig, AppearanceConfig, MarketAnalysisConfig, MarketAnalysisState,
    PopupConfig, StockEntry, CURRENT_CONFIG_SCHEMA_VERSION,
};

pub(crate) fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("StockTray")
        .join("config.json")
}

pub(crate) fn market_state_path() -> PathBuf {
    config_path().with_file_name("market-snapshots.json")
}

pub(crate) fn load_market_state() -> MarketAnalysisState {
    let state = fs::read_to_string(market_state_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let now = Local::now();
    normalize_market_state(
        state,
        &now.format("%Y-%m-%d").to_string(),
        now.weekday().number_from_monday() <= 5,
    )
}

pub(crate) fn save_market_state(state: &MarketAnalysisState) -> Result<(), String> {
    let path = market_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    atomic_write(
        &path,
        serde_json::to_string_pretty(state)
            .map_err(|error| error.to_string())?
            .as_bytes(),
    )
}

pub(crate) fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        let legacy = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .map(|p| p.join("config.json"));
        if let Some(legacy) = legacy {
            if legacy.exists() {
                let _ = fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
                let _ = fs::copy(legacy, &path);
            }
        }
    }

    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<AppConfig>(&text) {
            Ok(config) => {
                let normalized = normalize_config(config);
                let _ = save_config_to(&path, &normalized);
                return normalized;
            }
            Err(_) => backup_invalid_config(&path),
        },
        Err(_) => {
            if path.exists() {
                backup_invalid_config(&path);
            }
        }
    }

    let config = default_config();
    let _ = save_config_to(&path, &config);
    config
}

pub(crate) fn save_config_to(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&normalize_config(config.clone()))
        .map_err(|e| e.to_string())?;
    atomic_write(path, text.as_bytes())
}

fn empty_market_state() -> MarketAnalysisState {
    MarketAnalysisState {
        sample_version: crate::market::SAMPLE_VERSION.into(),
        algorithm_version: crate::market::ALGORITHM_VERSION.into(),
        ..Default::default()
    }
}

fn normalize_market_state(
    mut state: MarketAnalysisState,
    today: &str,
    is_weekday: bool,
) -> MarketAnalysisState {
    expire_market_state_if_needed(&mut state, today, is_weekday);
    state
}

pub(crate) fn expire_market_state_if_needed(
    state: &mut MarketAnalysisState,
    today: &str,
    is_weekday: bool,
) -> bool {
    let wrong_version = state.sample_version != crate::market::SAMPLE_VERSION
        || state.algorithm_version != crate::market::ALGORITHM_VERSION;
    let old_trading_day = is_weekday
        && state
            .current
            .as_ref()
            .is_some_and(|snapshot| snapshot.trading_date != today);
    if wrong_version || old_trading_day || (state.current.is_none() && !state.history.is_empty()) {
        *state = empty_market_state();
        true
    } else {
        false
    }
}

pub(crate) fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("data");
    let temporary = path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), stamp));
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
        file.write_all(contents)
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        replace_file(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

pub(crate) fn normalize_config(mut config: AppConfig) -> AppConfig {
    let original_schema_version = config.schema_version;
    migrate_config(&mut config, original_schema_version);
    config.schema_version = CURRENT_CONFIG_SCHEMA_VERSION;

    let mut tooltip_selected = false;
    for stock in &mut config.stocks {
        stock.holdings = normalize_holding(stock.holdings);
        stock.cost_price = normalize_cost_price(stock.cost_price);
        if stock.show_in_tooltip {
            if tooltip_selected {
                stock.show_in_tooltip = false;
            } else {
                tooltip_selected = true;
            }
        }
    }
    if !tooltip_selected {
        if let Some(stock) = config.stocks.first_mut() {
            stock.show_in_tooltip = true;
        }
    }
    if config.display_fields.is_empty() {
        config.display_fields = default_display_fields();
    }
    if config.tooltip_fields.is_empty() {
        config.tooltip_fields = default_tooltip_fields();
    }
    config
        .display_fields
        .retain(|field| is_supported_field(field) && field != "name" && field != "code");
    if config.display_fields.is_empty() {
        config.display_fields = default_display_fields();
    }
    config
        .tooltip_fields
        .retain(|field| is_supported_field(field));
    if config.tooltip_fields.is_empty() {
        config.tooltip_fields = default_tooltip_fields();
    }
    if config.theme.is_empty() {
        config.theme = default_theme();
    }
    config.background_refresh_interval_ms =
        normalize_background_refresh_interval_ms(config.background_refresh_interval_ms);
    config.popup.auto_hide_ms = normalize_auto_hide_ms(config.popup.auto_hide_ms);
    config.market_analysis.refresh_minutes = match config.market_analysis.refresh_minutes {
        5 | 15 | 30 => config.market_analysis.refresh_minutes,
        _ => 15,
    };
    config
}

fn migrate_config(config: &mut AppConfig, from_version: u32) {
    if from_version < 2 {
        // v2 makes tray tooltip stock selection single-choice. The normalizer below
        // enforces the final invariant while preserving the first existing choice.
    }
    if from_version < 3 {
        // v3 adds background quote refresh. Serde default fills the field for old configs.
    }
    if from_version < 4
        && !config
            .tooltip_fields
            .iter()
            .any(|field| field == "position_pnl")
    {
        config.tooltip_fields.push("position_pnl".into());
    }
    if from_version < 5 {
        // v5 adds the independent market-analysis schedule; serde supplies defaults.
    }
}

fn backup_invalid_config(path: &Path) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let backup_path = path.with_file_name(format!("config.invalid.{timestamp}.json"));
    let _ = fs::copy(path, backup_path);
}

pub(crate) fn normalize_holding(value: f32) -> f32 {
    if value.is_finite() {
        ((value.max(0.0) / 100.0).round() * 100.0).max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn normalize_cost_price(value: f32) -> f32 {
    if value.is_finite() {
        round3(value)
    } else {
        0.0
    }
}

fn normalize_background_refresh_interval_ms(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        value.clamp(1_000, 600_000)
    }
}

fn normalize_auto_hide_ms(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        let clamped = value.clamp(100, 30_000);
        ((clamped + 50) / 100) * 100
    }
}

pub(crate) fn is_supported_field(field: &str) -> bool {
    matches!(
        field,
        "name"
            | "code"
            | "price"
            | "prev_close"
            | "open"
            | "high"
            | "low"
            | "change"
            | "change_percent"
            | "volume"
            | "amount"
            | "volume_ratio"
            | "turnover"
            | "holdings"
            | "cost_price"
            | "daily_pnl"
            | "daily_pnl_percent"
            | "position_pnl"
            | "position_pnl_percent"
    )
}

pub(crate) fn default_config() -> AppConfig {
    AppConfig {
        schema_version: CURRENT_CONFIG_SCHEMA_VERSION,
        stocks: vec![
            StockEntry {
                code: "sh600519".into(),
                name: "贵州茅台".into(),
                holdings: 100.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: true,
            },
            StockEntry {
                code: "sz000858".into(),
                name: "五粮液".into(),
                holdings: 500.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: false,
            },
            StockEntry {
                code: "sh600036".into(),
                name: "招商银行".into(),
                holdings: 1000.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: false,
            },
            StockEntry {
                code: "sz300750".into(),
                name: "宁德时代".into(),
                holdings: 200.0,
                cost_price: 0.0,
                show_in_popup: true,
                show_in_tooltip: false,
            },
        ],
        theme: default_theme(),
        show_daily_summary: true,
        background_refresh_interval_ms: default_background_refresh_interval_ms(),
        market_analysis: MarketAnalysisConfig::default(),
        popup: PopupConfig::default(),
        appearance: AppearanceConfig::default(),
        display_fields: default_display_fields(),
        tooltip_fields: default_tooltip_fields(),
    }
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MarketEvidence, MarketSnapshot};

    fn market_state(date: &str) -> MarketAnalysisState {
        MarketAnalysisState {
            current: Some(MarketSnapshot {
                trading_date: date.into(),
                ..Default::default()
            }),
            history: vec![MarketEvidence::default()],
            sample_version: crate::market::SAMPLE_VERSION.into(),
            algorithm_version: crate::market::ALGORITHM_VERSION.into(),
            ..Default::default()
        }
    }

    #[test]
    fn market_state_expires_on_version_or_new_weekday() {
        let mut wrong_version = market_state("2026-07-10");
        wrong_version.algorithm_version = "old".into();
        assert!(normalize_market_state(wrong_version, "2026-07-10", true)
            .current
            .is_none());
        assert!(
            normalize_market_state(market_state("2026-07-10"), "2026-07-13", true)
                .current
                .is_none()
        );
        assert!(
            normalize_market_state(market_state("2026-07-10"), "2026-07-12", false)
                .current
                .is_some()
        );
    }

    #[test]
    fn atomic_write_replaces_existing_content() {
        let path = std::env::temp_dir().join(format!(
            "stocktray-atomic-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        atomic_write(&path, b"old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn config_normalization_preserves_ui_contracts() {
        let mut config = default_config();
        for stock in &mut config.stocks {
            stock.show_in_tooltip = true;
        }
        config.display_fields = vec!["name".into(), "unknown".into()];
        config.tooltip_fields.clear();
        config.background_refresh_interval_ms = 0;
        config.popup.auto_hide_ms = 42;
        config.market_analysis.refresh_minutes = 7;

        let normalized = normalize_config(config);
        assert_eq!(
            normalized
                .stocks
                .iter()
                .filter(|stock| stock.show_in_tooltip)
                .count(),
            1
        );
        assert_eq!(normalized.display_fields, default_display_fields());
        assert_eq!(normalized.tooltip_fields, default_tooltip_fields());
        assert_eq!(normalized.background_refresh_interval_ms, 0);
        assert_eq!(normalized.popup.auto_hide_ms, 100);
        assert_eq!(normalized.market_analysis.refresh_minutes, 15);
        assert_eq!(normalize_auto_hide_ms(151), 200);
        assert_eq!(normalize_auto_hide_ms(30_001), 30_000);
    }
}

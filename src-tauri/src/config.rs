use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::models::{
    default_background_refresh_interval_ms, default_display_fields, default_theme,
    default_tooltip_fields, AppConfig, AppearanceConfig, PopupConfig, StockEntry,
    CURRENT_CONFIG_SCHEMA_VERSION,
};

pub(crate) fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("StockTray")
        .join("config.json")
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
    fs::write(path, text).map_err(|e| e.to_string())
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
        .retain(|field| is_supported_field(field));
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
    config
}

fn migrate_config(_config: &mut AppConfig, from_version: u32) {
    if from_version < 2 {
        // v2 makes tray tooltip stock selection single-choice. The normalizer below
        // enforces the final invariant while preserving the first existing choice.
    }
    if from_version < 3 {
        // v3 adds background quote refresh. Serde default fills the field for old configs.
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
    ((value.max(100.0) / 100.0).round() * 100.0).max(100.0)
}

pub(crate) fn normalize_cost_price(value: f32) -> f32 {
    if value.is_finite() {
        round2(value)
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
            | "turnover"
            | "holdings"
            | "cost_price"
            | "daily_pnl"
            | "daily_pnl_percent"
            | "position_pnl"
            | "position_pnl_percent"
    )
}

fn default_config() -> AppConfig {
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
        popup: PopupConfig::default(),
        appearance: AppearanceConfig::default(),
        display_fields: default_display_fields(),
        tooltip_fields: default_tooltip_fields(),
    }
}

fn round2(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

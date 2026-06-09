use serde::{Deserialize, Serialize};

pub(crate) const CURRENT_CONFIG_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppConfig {
    #[serde(default = "default_config_schema_version")]
    pub(crate) schema_version: u32,
    pub(crate) stocks: Vec<StockEntry>,
    #[serde(default = "default_theme")]
    pub(crate) theme: String,
    #[serde(default = "default_true")]
    pub(crate) show_daily_summary: bool,
    #[serde(
        default = "default_background_refresh_interval_ms",
        alias = "refresh_interval_ms"
    )]
    pub(crate) background_refresh_interval_ms: u32,
    #[serde(default)]
    pub(crate) popup: PopupConfig,
    #[serde(default)]
    pub(crate) appearance: AppearanceConfig,
    #[serde(default = "default_display_fields")]
    pub(crate) display_fields: Vec<String>,
    #[serde(default = "default_tooltip_fields")]
    pub(crate) tooltip_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StockEntry {
    pub(crate) code: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) holdings: f32,
    #[serde(default)]
    pub(crate) cost_price: f32,
    #[serde(default = "default_true")]
    pub(crate) show_in_popup: bool,
    #[serde(default = "default_true")]
    pub(crate) show_in_tooltip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PopupConfig {
    #[serde(default = "default_up_color")]
    pub(crate) up_color: String,
    #[serde(default = "default_down_color")]
    pub(crate) down_color: String,
    #[serde(default = "default_flat_color")]
    pub(crate) flat_color: String,
    #[serde(default)]
    pub(crate) auto_hide_ms: u32,
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            up_color: default_up_color(),
            down_color: default_down_color(),
            flat_color: default_flat_color(),
            auto_hide_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppearanceConfig {
    #[serde(default = "default_theme_mode")]
    pub(crate) theme_mode: String,
    #[serde(default = "default_backdrop")]
    pub(crate) backdrop: String,
    #[serde(default = "default_popup_tint_opacity")]
    pub(crate) popup_tint_opacity: f64,
    #[serde(default = "default_corner_radius")]
    pub(crate) corner_radius: f64,
    #[serde(default = "default_true")]
    pub(crate) animations_enabled: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme_mode: default_theme_mode(),
            backdrop: default_backdrop(),
            popup_tint_opacity: default_popup_tint_opacity(),
            corner_radius: default_corner_radius(),
            animations_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct StockData {
    pub(crate) code: String,
    pub(crate) name: String,
    pub(crate) price: f32,
    pub(crate) prev_close: f32,
    pub(crate) open: f32,
    pub(crate) high: f32,
    pub(crate) low: f32,
    pub(crate) volume: f32,
    pub(crate) amount: f32,
    pub(crate) volume_ratio: f32,
    pub(crate) change: f32,
    pub(crate) change_percent: f32,
    pub(crate) turnover: f32,
    pub(crate) date: String,
    pub(crate) time: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct DailyPnlItem {
    pub(crate) code: String,
    pub(crate) name: String,
    pub(crate) price: f32,
    pub(crate) prev_close: f32,
    pub(crate) open: f32,
    pub(crate) high: f32,
    pub(crate) low: f32,
    pub(crate) volume: f32,
    pub(crate) amount: f32,
    pub(crate) volume_ratio: f32,
    pub(crate) change: f32,
    pub(crate) change_percent: f32,
    pub(crate) turnover: f32,
    pub(crate) date: String,
    pub(crate) time: String,
    pub(crate) holdings: f32,
    pub(crate) cost_price: f32,
    pub(crate) daily_pnl: f32,
    pub(crate) daily_pnl_percent: f32,
    pub(crate) position_pnl: f32,
    pub(crate) position_pnl_percent: f32,
    pub(crate) show_in_popup: bool,
    pub(crate) show_in_tooltip: bool,
    pub(crate) error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct DailySummary {
    pub(crate) total_prev_value: f32,
    pub(crate) total_daily_pnl: f32,
    pub(crate) total_daily_pnl_percent: f32,
    pub(crate) items: Vec<DailyPnlItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AppStatePayload {
    pub(crate) app_version: String,
    pub(crate) config: AppConfig,
    pub(crate) summary: Option<DailySummary>,
    pub(crate) last_refreshed_at: Option<String>,
    pub(crate) last_error: Option<String>,
}

pub(crate) fn default_config_schema_version() -> u32 {
    1
}
pub(crate) fn default_theme() -> String {
    "dark".into()
}
pub(crate) fn default_theme_mode() -> String {
    "system".into()
}
pub(crate) fn default_backdrop() -> String {
    "acrylic".into()
}
pub(crate) fn default_up_color() -> String {
    "#C73E4E".into()
}
pub(crate) fn default_down_color() -> String {
    "#5B8C5A".into()
}
pub(crate) fn default_flat_color() -> String {
    "#999999".into()
}
pub(crate) fn default_popup_tint_opacity() -> f64 {
    0.38
}
pub(crate) fn default_corner_radius() -> f64 {
    14.0
}
pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_background_refresh_interval_ms() -> u32 {
    10_000
}

pub(crate) fn default_display_fields() -> Vec<String> {
    ["price", "change_percent", "daily_pnl", "daily_pnl_percent"]
        .into_iter()
        .map(String::from)
        .collect()
}

pub(crate) fn default_tooltip_fields() -> Vec<String> {
    ["price", "change_percent", "daily_pnl", "position_pnl"]
        .into_iter()
        .map(String::from)
        .collect()
}

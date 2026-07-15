use serde::{Deserialize, Serialize};

pub(crate) const CURRENT_CONFIG_SCHEMA_VERSION: u32 = 7;

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
    pub(crate) market_analysis: MarketAnalysisConfig,
    #[serde(default)]
    pub(crate) popup: PopupConfig,
    #[serde(default)]
    pub(crate) appearance: AppearanceConfig,
    #[serde(default)]
    pub(crate) external_data_sources: Vec<ExternalDataSourceConfig>,
    #[serde(default = "default_data_source_order")]
    pub(crate) data_source_order: Vec<String>,
    #[serde(default = "default_display_fields")]
    pub(crate) display_fields: Vec<String>,
    #[serde(default = "default_tooltip_fields")]
    pub(crate) tooltip_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExternalDataSourceConfig {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default = "default_futu_provider")]
    pub(crate) provider: String,
    #[serde(default = "default_futu_name")]
    pub(crate) name: String,
    #[serde(default = "default_futu_host")]
    pub(crate) host: String,
    #[serde(default = "default_futu_port")]
    pub(crate) port: u16,
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DataSourceTestResult {
    pub(crate) ok: bool,
    pub(crate) latency_ms: u128,
    pub(crate) message: String,
}

pub(crate) fn default_data_source_order() -> Vec<String> {
    vec!["eastmoney".into(), "tencent".into()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MarketAnalysisConfig {
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default = "default_market_refresh_minutes")]
    pub(crate) refresh_minutes: u32,
}

impl Default for MarketAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_minutes: default_market_refresh_minutes(),
        }
    }
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
    pub(crate) float_market_cap: f64,
    pub(crate) upper_limit: f32,
    pub(crate) lower_limit: f32,
    pub(crate) listing_date: String,
    pub(crate) source: String,
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
    pub(crate) market: MarketAnalysisState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketAnalysisState {
    pub(crate) current: Option<MarketSnapshot>,
    pub(crate) history: Vec<MarketEvidence>,
    pub(crate) last_error: Option<String>,
    pub(crate) universe_size: usize,
    pub(crate) sample_version: String,
    pub(crate) algorithm_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketHistoryStore {
    #[serde(default = "default_market_history_schema_version")]
    pub(crate) schema_version: u32,
    #[serde(default)]
    pub(crate) days: Vec<MarketDayArchive>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketDayArchive {
    pub(crate) trading_date: String,
    #[serde(default)]
    pub(crate) sample_version: String,
    #[serde(default)]
    pub(crate) algorithm_version: String,
    pub(crate) snapshot: MarketSnapshot,
    #[serde(default)]
    pub(crate) history: Vec<MarketEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketStorageInfo {
    pub(crate) total_days: usize,
    pub(crate) archived_days: usize,
    pub(crate) trend_points: usize,
    pub(crate) size_bytes: u64,
    pub(crate) earliest_date: String,
    pub(crate) latest_date: String,
    pub(crate) current_date: String,
    pub(crate) days: Vec<MarketDaySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketDaySummary {
    pub(crate) trading_date: String,
    pub(crate) trend_points: usize,
    pub(crate) leader_label: String,
    pub(crate) status: String,
    pub(crate) is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketSnapshot {
    pub(crate) trading_date: String,
    pub(crate) time: String,
    #[serde(default)]
    pub(crate) phase: String,
    pub(crate) status: String,
    pub(crate) leader: Option<String>,
    pub(crate) leader_label: String,
    pub(crate) signal_consistency: String,
    #[serde(default)]
    pub(crate) rotation_target: Option<String>,
    #[serde(default)]
    pub(crate) rotation_label: String,
    #[serde(default)]
    pub(crate) stability: f32,
    pub(crate) quality: MarketDataQuality,
    pub(crate) styles: Vec<StyleAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketEvidence {
    pub(crate) time: String,
    #[serde(default)]
    pub(crate) phase: String,
    #[serde(default)]
    pub(crate) sample_source: String,
    pub(crate) leader: Option<String>,
    pub(crate) scores: Vec<f32>,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) preferences: Vec<f32>,
    #[serde(default)]
    pub(crate) cap_weight_returns: Vec<f32>,
    #[serde(default)]
    pub(crate) equal_weight_returns: Vec<f32>,
    #[serde(default)]
    pub(crate) coverage: f32,
    #[serde(default)]
    pub(crate) minimum_style_coverage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketDataQuality {
    pub(crate) expected: usize,
    pub(crate) received: usize,
    pub(crate) coverage: f32,
    pub(crate) mode: String,
    #[serde(default)]
    pub(crate) sample_source: String,
    #[serde(default)]
    pub(crate) style_coverage: Vec<f32>,
    #[serde(default)]
    pub(crate) minimum_style_coverage: f32,
    #[serde(default)]
    pub(crate) raw_received: usize,
    #[serde(default)]
    pub(crate) excluded_st: usize,
    #[serde(default)]
    pub(crate) excluded_new: usize,
    #[serde(default)]
    pub(crate) excluded_halted: usize,
    #[serde(default)]
    pub(crate) timestamp_missing: usize,
    #[serde(default)]
    pub(crate) delayed_count: usize,
    #[serde(default)]
    pub(crate) index_expected: usize,
    #[serde(default)]
    pub(crate) index_received: usize,
    #[serde(default)]
    pub(crate) broad_index_received: usize,
    #[serde(default)]
    pub(crate) index_cached: bool,
    #[serde(default)]
    pub(crate) index_derived: bool,
    #[serde(default)]
    pub(crate) style_index_coverage: Vec<f32>,
    #[serde(default)]
    pub(crate) index_error: String,
    pub(crate) primary_count: usize,
    pub(crate) fallback_count: usize,
    pub(crate) stale_count: usize,
    pub(crate) updated_at: String,
    #[serde(default)]
    pub(crate) conclusion_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct StyleAnalysis {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) subtitle: String,
    pub(crate) score: f32,
    pub(crate) heat: f32,
    pub(crate) preference: f32,
    #[serde(default)]
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) score_change: f32,
    #[serde(default)]
    pub(crate) score_velocity: f32,
    pub(crate) relative_return: f32,
    pub(crate) breadth: f32,
    pub(crate) activity: f32,
    pub(crate) confirmation: f32,
    pub(crate) consistency: f32,
    pub(crate) concentration: f32,
    #[serde(default)]
    pub(crate) entropy: f32,
    #[serde(default)]
    pub(crate) diffusion: f32,
    #[serde(default)]
    pub(crate) direction: String,
    #[serde(default)]
    pub(crate) directional_share: f32,
    #[serde(default)]
    pub(crate) equal_weight_return: f32,
    #[serde(default)]
    pub(crate) cap_weight_return: f32,
    #[serde(default)]
    pub(crate) weighting_divergence: f32,
    #[serde(default)]
    pub(crate) float_cap_coverage: f32,
    #[serde(default)]
    pub(crate) top_stock_weight: f32,
    #[serde(default)]
    pub(crate) top_five_weight: f32,
    #[serde(default)]
    pub(crate) effective_sample_size: f32,
    pub(crate) subsectors: Vec<SubsectorAnalysis>,
    #[serde(default)]
    pub(crate) contributions: Vec<MarketContribution>,
    pub(crate) positive: Vec<MarketContribution>,
    pub(crate) negative: Vec<MarketContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct SubsectorAnalysis {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) contribution: f32,
    pub(crate) breadth: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MarketContribution {
    pub(crate) code: String,
    pub(crate) name: String,
    pub(crate) subsector: String,
    pub(crate) contribution: f32,
    pub(crate) change_percent: f32,
    #[serde(default)]
    pub(crate) stock_weight_percent: f32,
    #[serde(default)]
    pub(crate) attribution_weight_percent: f32,
    #[serde(default)]
    pub(crate) signal_score: f32,
    #[serde(default)]
    pub(crate) contribution_share: f32,
    #[serde(default)]
    pub(crate) gap_percent: f32,
    #[serde(default)]
    pub(crate) intraday_percent: f32,
    #[serde(default)]
    pub(crate) reason: String,
}

pub(crate) fn default_config_schema_version() -> u32 {
    1
}
pub(crate) fn default_market_history_schema_version() -> u32 {
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

fn default_futu_provider() -> String {
    "futu_opend".into()
}

fn default_futu_name() -> String {
    "富途 OpenD".into()
}

fn default_futu_host() -> String {
    "127.0.0.1".into()
}

fn default_futu_port() -> u16 {
    32179
}

pub(crate) fn default_background_refresh_interval_ms() -> u32 {
    10_000
}

pub(crate) fn default_market_refresh_minutes() -> u32 {
    15
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

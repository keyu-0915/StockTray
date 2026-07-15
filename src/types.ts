export type StockEntry = {
  code: string;
  name: string;
  holdings: number;
  cost_price: number;
  show_in_popup: boolean;
  show_in_tooltip: boolean;
};

export type PopupConfig = {
  up_color: string;
  down_color: string;
  flat_color: string;
  auto_hide_ms: number;
};

export type AppearanceConfig = {
  theme_mode: string;
  backdrop: string;
  popup_tint_opacity: number;
  corner_radius: number;
  animations_enabled: boolean;
};

export type ExternalDataSourceConfig = {
  id: string;
  provider: "futu_opend" | string;
  name: string;
  host: string;
  port: number;
  enabled: boolean;
};

export type DataSourceTestResult = {
  ok: boolean;
  latency_ms: number;
  message: string;
};

export type AppConfig = {
  schema_version: number;
  stocks: StockEntry[];
  theme: string;
  show_daily_summary: boolean;
  background_refresh_interval_ms: number;
  market_analysis: {
    enabled: boolean;
    refresh_minutes: number;
  };
  popup: PopupConfig;
  appearance: AppearanceConfig;
  external_data_sources: ExternalDataSourceConfig[];
  data_source_order: string[];
  display_fields: string[];
  tooltip_fields: string[];
};

export type DailyPnlItem = {
  code: string;
  name: string;
  price: number;
  prev_close: number;
  open: number;
  high: number;
  low: number;
  volume: number;
  amount: number;
  volume_ratio: number;
  change: number;
  change_percent: number;
  turnover: number;
  date: string;
  time: string;
  holdings: number;
  cost_price: number;
  daily_pnl: number;
  daily_pnl_percent: number;
  position_pnl: number;
  position_pnl_percent: number;
  show_in_popup: boolean;
  show_in_tooltip: boolean;
  error: string;
};

export type DailySummary = {
  total_prev_value: number;
  total_daily_pnl: number;
  total_daily_pnl_percent: number;
  items: DailyPnlItem[];
};

export type AppStatePayload = {
  app_version: string;
  config: AppConfig;
  summary: DailySummary | null;
  last_refreshed_at: string | null;
  last_error: string | null;
  market: MarketAnalysisState;
};

export type MarketDaySummary = {
  trading_date: string;
  trend_points: number;
  leader_label: string;
  status: string;
  is_current: boolean;
};

export type MarketStorageInfo = {
  total_days: number;
  archived_days: number;
  trend_points: number;
  size_bytes: number;
  earliest_date: string;
  latest_date: string;
  current_date: string;
  days: MarketDaySummary[];
};

export type MarketContribution = {
  code: string;
  name: string;
  subsector: string;
  contribution: number;
  change_percent: number;
  stock_weight_percent: number;
  attribution_weight_percent: number;
  signal_score: number;
  contribution_share: number;
  gap_percent: number;
  intraday_percent: number;
  reason: string;
};

export type SubsectorAnalysis = {
  id: string;
  name: string;
  contribution: number;
  breadth: number;
};

export type StyleAnalysis = {
  id: string;
  label: string;
  subtitle: string;
  score: number;
  heat: number;
  preference: number;
  state: string;
  score_change: number;
  score_velocity: number;
  relative_return: number;
  breadth: number;
  activity: number;
  confirmation: number;
  consistency: number;
  concentration: number;
  entropy: number;
  diffusion: number;
  direction: string;
  directional_share: number;
  equal_weight_return: number;
  cap_weight_return: number;
  weighting_divergence: number;
  float_cap_coverage: number;
  top_stock_weight: number;
  top_five_weight: number;
  effective_sample_size: number;
  subsectors: SubsectorAnalysis[];
  contributions: MarketContribution[];
  positive: MarketContribution[];
  negative: MarketContribution[];
};

export type MarketSnapshot = {
  trading_date: string;
  time: string;
  phase: string;
  status: string;
  leader: string | null;
  leader_label: string;
  signal_consistency: string;
  rotation_target: string | null;
  rotation_label: string;
  stability: number;
  quality: {
    expected: number;
    received: number;
    coverage: number;
    mode: string;
    sample_source: string;
    style_coverage: number[];
    minimum_style_coverage: number;
    raw_received: number;
    excluded_st: number;
    excluded_new: number;
    excluded_halted: number;
    timestamp_missing: number;
    delayed_count: number;
    index_expected: number;
    index_received: number;
    broad_index_received: number;
    index_cached: boolean;
    index_derived: boolean;
    style_index_coverage: number[];
    index_error: string;
    primary_count: number;
    fallback_count: number;
    stale_count: number;
    updated_at: string;
    conclusion_ready: boolean;
  };
  styles: StyleAnalysis[];
};

export type MarketAnalysisState = {
  current: MarketSnapshot | null;
  history: Array<{
    time: string;
    phase: string;
    leader: string | null;
    scores: number[];
    status: string;
    preferences: number[];
    cap_weight_returns: number[];
    equal_weight_returns: number[];
    coverage: number;
    minimum_style_coverage: number;
  }>;
  last_error: string | null;
  universe_size: number;
  sample_version: string;
  algorithm_version: string;
};

export type UpdateCheckResult = {
  available: boolean;
  current_version: string;
  version: string | null;
};

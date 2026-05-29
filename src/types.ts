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

export type AppConfig = {
  schema_version: number;
  stocks: StockEntry[];
  theme: string;
  show_daily_summary: boolean;
  background_refresh_interval_ms: number;
  popup: PopupConfig;
  appearance: AppearanceConfig;
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
  config: AppConfig;
  summary: DailySummary | null;
  last_refreshed_at: string | null;
  last_error: string | null;
};

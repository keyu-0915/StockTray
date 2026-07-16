import React, { useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { createPortal } from "react-dom";
import {
  addStock,
  checkAndInstallUpdate,
  getState,
  hidePopup,
  onState,
  onOpenPage,
  refreshMarketAnalysis,
  clearMarketSnapshots,
  clearMarketHistoryArchive,
  deleteMarketHistoryDate,
  getMarketStorageInfo,
  refreshQuotes,
  saveSettings,
  setPopupHovered,
  closeWindow,
  minimizeWindow,
  startWindowDragging,
  testDataSource,
  toggleMaximizeWindow,
} from "./tauri";
import type {
  AppConfig,
  AppStatePayload,
  DailyPnlItem,
  DataSourceTestResult,
  ExternalDataSourceConfig,
  MarketContribution,
  MarketStorageInfo,
  StockEntry,
  StyleAnalysis,
} from "./types";
import supportQr from "./assets/support-wechat-qr.png";
import "./styles.css";

type Page = "overview" | "holdings" | "market" | "settings";
type FieldKey =
  | "name"
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
  | "position_pnl_percent";

const QUOTE_FIELD_OPTIONS: Array<{ key: FieldKey; label: string }> = [
  ["price", "最新价"],
  ["change", "涨跌额"],
  ["change_percent", "涨跌幅"],
  ["prev_close", "昨收"],
  ["open", "今开"],
  ["high", "最高"],
  ["low", "最低"],
  ["volume", "成交量"],
  ["amount", "成交额"],
  ["volume_ratio", "量比"],
  ["turnover", "换手率"],
  ["holdings", "持仓"],
  ["cost_price", "成本"],
  ["daily_pnl", "当日盈亏"],
  ["daily_pnl_percent", "当日盈亏比"],
  ["position_pnl", "持仓盈亏"],
  ["position_pnl_percent", "持仓盈亏比"],
].map(([key, label]) => ({ key: key as FieldKey, label }));
const TOOLTIP_FIELD_OPTIONS = [
  { key: "name" as FieldKey, label: "名称" },
  { key: "code" as FieldKey, label: "代码" },
  ...QUOTE_FIELD_OPTIONS,
];
const DEFAULT_POPUP_FIELDS: FieldKey[] = [
  "price",
  "change_percent",
  "daily_pnl",
  "daily_pnl_percent",
];
const DEFAULT_TOOLTIP_FIELDS: FieldKey[] = [
  "price",
  "change_percent",
  "daily_pnl",
  "position_pnl",
];

const PAGES: Array<[Page, string]> = [
  ["overview", "概览"],
  ["holdings", "自选持仓"],
  ["market", "市场风格"],
  ["settings", "设置"],
];

function pageFromLocation(): Page {
  const route = window.location.hash.replace(/^#\/?/, "");
  return PAGES.some(([page]) => page === route) ? route as Page : "overview";
}

function isPage(value: string): value is Page {
  return PAGES.some(([page]) => page === value);
}
const SUPPORT_QR = supportQr;

function SlidingButtons<T extends string | number>({
  className = "",
  options,
  value,
  onChange,
}: {
  className?: string;
  options: Array<[T, string]>;
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <div
      className={`sliding-options ${className}`}
      style={{
        "--option-count": options.length,
        "--selected-index": Math.max(0, options.findIndex(([id]) => id === value)),
      } as React.CSSProperties}
    >
      {options.map(([id, label]) => (
        <button
          aria-pressed={id === value}
          className={id === value ? "active" : ""}
          key={id}
          onClick={() => onChange(id)}
          type="button"
        >
          {label}
        </button>
      ))}
    </div>
  );
}

function signed(value: number, digits = 2, suffix = "") {
  return `${value > 0 ? "+" : ""}${value.toFixed(digits)}${suffix}`;
}

function tone(value: number) {
  return value > 0.0001 ? "up" : value < -0.0001 ? "down" : "flat";
}

function money(value: number) {
  return value.toLocaleString("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function compactAmount(value: number) {
  if (!value) return "-";
  if (Math.abs(value) >= 100_000_000)
    return `${(value / 100_000_000).toFixed(2)}亿`;
  if (Math.abs(value) >= 10_000) return `${(value / 10_000).toFixed(1)}万`;
  return value.toFixed(0);
}

function selectedFields(
  config: AppConfig | undefined,
  source: "display_fields" | "tooltip_fields",
  fallback: FieldKey[],
  options: Array<{ key: FieldKey }>,
) {
  const allowed = new Set(options.map((option) => option.key));
  const fields = (config?.[source] ?? fallback).filter(
    (field): field is FieldKey => allowed.has(field as FieldKey),
  );
  return fields.length ? fields : fallback;
}

function fieldValue(item: DailyPnlItem, field: FieldKey) {
  const value = item[field];
  if (field === "name" || field === "code") return String(value || "-");
  const number = Number(value || 0);
  if (
    ["price", "prev_close", "open", "high", "low", "cost_price"].includes(field)
  )
    return number ? number.toFixed(3) : "-";
  if (field === "volume" || field === "amount") return compactAmount(number);
  if (field === "holdings") return number.toLocaleString("zh-CN");
  if (
    field === "change_percent" ||
    field === "daily_pnl_percent" ||
    field === "position_pnl_percent" ||
    field === "turnover"
  )
    return signed(number, 2, "%");
  if (field === "volume_ratio") return number ? number.toFixed(2) : "-";
  return signed(number, 2);
}

function fieldTone(item: DailyPnlItem, field: FieldKey) {
  if (field === "change" || field === "change_percent")
    return tone(item.change_percent);
  if (field === "daily_pnl" || field === "daily_pnl_percent")
    return tone(item.daily_pnl);
  if (field === "position_pnl" || field === "position_pnl_percent")
    return tone(item.position_pnl);
  return "";
}

function toggleField(
  config: AppConfig,
  key: "display_fields" | "tooltip_fields",
  field: FieldKey,
  checked: boolean,
  fallback: FieldKey[],
): AppConfig {
  const next = checked
    ? Array.from(new Set([...config[key], field]))
    : config[key].filter((value) => value !== field);
  return { ...config, [key]: next.length ? next : fallback };
}

function useTheme(themeMode?: string) {
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const theme =
        themeMode === "system"
          ? media.matches
            ? "dark"
            : "light"
          : themeMode || "dark";
      document.documentElement.dataset.theme = theme;
      document.documentElement.style.colorScheme =
        theme === "light" || theme === "soft" || theme === "morandi"
          ? "light"
          : "dark";
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [themeMode]);
}

function PopupApp() {
  const [state, setState] = useState<AppStatePayload | null>(null);
  useTheme(state?.config.appearance.theme_mode);
  useEffect(() => {
    document.body.dataset.view = "popup";
    document.documentElement.dataset.view = "popup";
    getState().then(setState).catch(console.error);
    const unlisten = onState(setState);
    return () => {
      delete document.body.dataset.view;
      delete document.documentElement.dataset.view;
      unlisten.then((fn) => fn()).catch(console.error);
    };
  }, []);
  const rows = state?.summary?.items.filter((item) => item.show_in_popup) ?? [];
  const fields = selectedFields(
    state?.config,
    "display_fields",
    DEFAULT_POPUP_FIELDS,
    QUOTE_FIELD_OPTIONS,
  );
  const market = state?.market.current;
  return (
    <main
      className={`popup-shell ${state?.config.appearance.animations_enabled === false ? "reduce-motion" : "motion"}`}
      style={
        {
          "--up-color": state?.config.popup.up_color,
          "--down-color": state?.config.popup.down_color,
          "--corner-radius": `${state?.config.appearance.corner_radius ?? 14}px`,
          "--popup-tint-opacity": Math.max(
            0.2,
            Math.min(0.95, state?.config.appearance.popup_tint_opacity ?? 0.9),
          ),
        } as React.CSSProperties
      }
      onDoubleClick={() => hidePopup().catch(console.error)}
      onMouseEnter={() => setPopupHovered(true).catch(console.error)}
      onMouseLeave={() => setPopupHovered(false).catch(console.error)}
    >
      <section className="popup-panel">
        {state?.config.show_daily_summary && state.summary && (
          <div
            className={`popup-summary ${tone(state.summary.total_daily_pnl)}`}
          >
            <span>今日合计</span>
            <strong>
              {signed(state.summary.total_daily_pnl)} ·{" "}
              {signed(state.summary.total_daily_pnl_percent, 2, "%")}
            </strong>
          </div>
        )}
        <div className="popup-market-card" aria-label="市场风格速览" role="status">
          <header>
            <div>
              <small>
                市场风格 · {market?.status === "dominant" ? "当前主导" : market?.status === "proxy" ? "代理倾向" : market?.status === "derived" ? market.quality.broad_index_received >= 3 ? "宽基+成分" : "成分推断" : market?.status === "auction" ? "竞价观察" : "观察中"}
              </small>
              <strong>{market?.leader_label ?? "等待分析"}</strong>
            </div>
            <span>{market ? `覆盖 ${market.quality.coverage.toFixed(0)}%` : "暂无快照"}</span>
          </header>
          <div className="popup-style-inline">
            {market?.styles.map((style) => (
              <span className={`${style.id} ${market.leader === style.id ? "active" : ""}`} key={style.id}>
                {style.label}
                <b>{style.score.toFixed(0)}</b>
              </span>
            )) ?? <small>首次有效行情后显示风格判断</small>}
          </div>
        </div>
        <div
          className={`quote-list fields-${fields.length <= 3 ? "compact" : fields.length <= 6 ? "balanced" : "detail"} ${rows.length === 1 ? "single" : ""}`}
        >
          {rows.length ? (
            rows.map((item) => (
              <PopupRow fields={fields} item={item} key={item.code} />
            ))
          ) : (
            <div className="empty">正在获取行情…</div>
          )}
        </div>
      </section>
    </main>
  );
}

function PopupRow({
  item,
  fields,
}: {
  item: DailyPnlItem;
  fields: FieldKey[];
}) {
  return (
    <div className="quote-row">
      <div className="quote-id">
        <strong>{item.name || item.code}</strong>
        <small>{item.code.toUpperCase()}</small>
      </div>
      <div className="quote-metrics">
        {fields.map((field) => (
          <div className={fieldTone(item, field)} key={field}>
            <strong>{fieldValue(item, field)}</strong>
            <small>
              {
                TOOLTIP_FIELD_OPTIONS.find((option) => option.key === field)
                  ?.label
              }
            </small>
          </div>
        ))}
      </div>
    </div>
  );
}

function DesktopApp() {
  const [state, setState] = useState<AppStatePayload | null>(null);
  const [draft, setDraft] = useState<AppConfig | null>(null);
  const [page, setPage] = useState<Page>(pageFromLocation);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  useTheme(draft?.appearance.theme_mode);

  useEffect(() => {
    document.body.dataset.view = "settings";
    document.documentElement.dataset.view = "settings";
    getState()
      .then((payload) => {
        setState(payload);
        setDraft(structuredClone(payload.config));
      })
      .catch(console.error);
    const unlisten = onState((payload) => {
      setState(payload);
      setDraft((current) => current ?? structuredClone(payload.config));
    });
    const unlistenPage = onOpenPage((nextPage) => {
      if (isPage(nextPage)) setPage(nextPage);
    });
    return () => {
      delete document.body.dataset.view;
      delete document.documentElement.dataset.view;
      unlisten.then((fn) => fn()).catch(console.error);
      unlistenPage.then((fn) => fn()).catch(console.error);
    };
  }, []);

  async function save() {
    if (!draft) return;
    setBusy(true);
    try {
      const payload = await saveSettings(draft);
      setState(payload);
      setDraft(structuredClone(payload.config));
      setMessage("设置已保存");
    } catch (error) {
      setMessage(`保存失败：${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function refreshAll() {
    setBusy(true);
    setMessage("正在刷新…");
    const tasks: Promise<unknown>[] = [refreshQuotes()];
    if (draft?.market_analysis.enabled) tasks.push(refreshMarketAnalysis());
    const results = await Promise.allSettled(tasks);
    const payload = await getState().catch(() => null);
    if (payload) setState(payload);
    const failed = results.find((result) => result.status === "rejected");
    setMessage(
      failed
        ? `部分刷新失败：${String((failed as PromiseRejectedResult).reason)}`
        : "刷新完成",
    );
    setBusy(false);
  }

  if (!draft || !state)
    return <main className="app-shell loading">正在加载 StockTray…</main>;

  return (
    <main
      className={`app-shell ${draft.appearance.animations_enabled ? "motion" : "reduce-motion"}`}
      style={
        {
          "--up-color": draft.popup.up_color,
          "--down-color": draft.popup.down_color,
        } as React.CSSProperties
      }
    >
      <header
        className="app-header"
        onMouseDown={(event) => {
          if (event.button === 0 && !(event.target as HTMLElement).closest("button")) {
            startWindowDragging().catch((error) => setMessage(`移动窗口失败：${String(error)}`));
          }
        }}
      >
        <div className="window-drag">
          <div className="wordmark">
            Stock<span>Tray</span>
            <small>v{state.app_version}</small>
          </div>
        </div>
        <nav aria-label="主导航">
          <SlidingButtons options={PAGES} value={page} onChange={setPage} />
        </nav>
        <div className="header-actions">
          <div className="header-status" role="status">
            <i className={state.last_error ? "bad" : ""} />
            {state.last_error ? "行情异常" : "数据正常"}
          </div>
          <div className="window-controls" aria-label="窗口控制">
            <button aria-label="最小化" onClick={() => minimizeWindow().catch((error) => setMessage(`最小化失败：${String(error)}`))} type="button">−</button>
            <button aria-label="最大化或还原" onClick={() => toggleMaximizeWindow().catch((error) => setMessage(`最大化失败：${String(error)}`))} type="button">□</button>
            <button aria-label="关闭到托盘" className="close" onClick={() => closeWindow().catch((error) => setMessage(`关闭失败：${String(error)}`))} type="button">×</button>
          </div>
        </div>
      </header>
      <div className="app-toolbar">
        <div>
          <strong>{PAGES.find(([id]) => id === page)?.[1]}</strong>
          <span>
            {message || (page === "market"
              ? marketStatusText(state)
              : `行情更新 ${state.last_refreshed_at ?? "-"}`)}
          </span>
        </div>
        <button disabled={busy} onClick={refreshAll}>
          {busy ? "刷新中…" : "刷新全部"}
        </button>
      </div>
      <div className={`page-content ${page === "holdings" || page === "settings" ? "has-save-fab" : ""}`}>
        {page === "overview" && <OverviewPage state={state} />}
        {page === "holdings" && (
          <HoldingsPage
            draft={draft}
            setDraft={setDraft}
            state={state}
            setMessage={setMessage}
          />
        )}
        {page === "market" && <MarketPage state={state} />}
        {page === "settings" && (
          <SettingsPage
            draft={draft}
            setDraft={setDraft}
            state={state}
            setMessage={setMessage}
          />
        )}
      </div>
      {(page === "holdings" || page === "settings") && (
        <button className="primary save-fab" disabled={busy} onClick={save}>
          保存并应用
        </button>
      )}
    </main>
  );
}

function OverviewPage({ state }: { state: AppStatePayload }) {
  const summary = state.summary;
  const market = state.market.current;
  return (
    <div className="overview-grid">
      <section className="hero-panel">
        <span>今日盈亏</span>
        <h1 className={tone(summary?.total_daily_pnl ?? 0)}>
          {summary ? signed(summary.total_daily_pnl) : "-"}
        </h1>
        <b className={tone(summary?.total_daily_pnl_percent ?? 0)}>
          {summary ? signed(summary.total_daily_pnl_percent, 2, "%") : "-"}
        </b>
        <div className="hero-metrics">
          <Metric label="持仓数量" value={`${summary?.items.length ?? 0}`} />
          <Metric label="胜率" value={holdingWinRate(summary?.items ?? [])} />
          <Metric
            label="持仓昨市值"
            value={compactAmount(summary?.total_prev_value ?? 0)}
          />
        </div>
      </section>
      <section className="market-summary panel">
        <header>
          <div>
            <span>市场风格</span>
            <h2>
              {market?.status === "dominant" ? "当前主导" : market?.status === "auction" ? "竞价观察" : "当前倾向"}：
              <em>{market?.leader_label ?? "等待分析"}</em>
            </h2>
          </div>
          <small>{market?.time ?? "-"}</small>
        </header>
        <div className="mini-styles">
          {market?.styles.map((style) => (
            <div key={style.id} className={style.id}>
              <b>{style.label}</b>
              <strong>{style.score.toFixed(0)}</strong>
              <small>偏好 {style.preference.toFixed(0)}</small>
              <Progress value={style.score} />
            </div>
          )) ?? <EmptyMarket />}
        </div>
        <p>
          {market
            ? `覆盖率 ${market.quality.coverage.toFixed(1)}%（最低分类 ${market.quality.minimum_style_coverage.toFixed(1)}%）· ${sampleSourceLabel(market.quality.sample_source)} · ${modeLabel(market.quality.mode)} · 信号${market.signal_consistency}`
            : "市场分析将在首次有效行情后生成"}
        </p>
      </section>
      <section className="panel holdings-summary">
        <SectionTitle
          title="持仓概览"
          detail={summary ? `总盈亏 ${signed(summary.total_daily_pnl)}` : ""}
        />
        <HoldingsTable items={summary?.items.slice(0, 5) ?? []} />
      </section>
      <StyleTrendPanel compact history={state.market.history} styles={state.market.current?.styles ?? []} />
    </div>
  );
}

function HoldingsPage({
  draft,
  setDraft,
  state,
  setMessage,
}: {
  draft: AppConfig;
  setDraft: React.Dispatch<React.SetStateAction<AppConfig | null>>;
  state: AppStatePayload;
  setMessage: (value: string) => void;
}) {
  const [code, setCode] = useState("");
  const [holdings, setHoldings] = useState("0");
  const [cost, setCost] = useState("");
  const [sort, setSort] = useState<{
    field: "holdings" | "change_percent";
    desc: boolean;
  } | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const dragSource = useRef<string | null>(null);
  const quotes = useMemo(
    () =>
      new Map((state.summary?.items ?? []).map((item) => [item.code, item])),
    [state.summary],
  );
  const update = (code: string, patch: Partial<StockEntry>) =>
    setDraft(
      (current) =>
        current && {
          ...current,
          stocks: current.stocks.map((stock) =>
            stock.code === code ? { ...stock, ...patch } : stock,
          ),
        },
    );
  const reorder = (code: string, offset: number) =>
    setDraft((current) => {
      if (!current) return current;
      const from = current.stocks.findIndex((stock) => stock.code === code);
      const to = Math.max(
        0,
        Math.min(current.stocks.length - 1, from + offset),
      );
      if (from < 0 || from === to) return current;
      const stocks = [...current.stocks];
      const [stock] = stocks.splice(from, 1);
      stocks.splice(to, 0, stock);
      return { ...current, stocks };
    });
  const sortBy = (field: "holdings" | "change_percent") => {
    const desc = sort?.field === field ? !sort.desc : true;
    setSort({ field, desc });
    setDraft(
      (current) =>
        current && {
          ...current,
          stocks: [...current.stocks].sort((a, b) => {
            const left =
              field === "holdings"
                ? a.holdings
                : (quotes.get(a.code)?.change_percent ?? -Infinity);
            const right =
              field === "holdings"
                ? b.holdings
                : (quotes.get(b.code)?.change_percent ?? -Infinity);
            return (left - right) * (desc ? -1 : 1);
          }),
        },
    );
  };
  const selectTooltip = (code: string) =>
    setDraft(
      (current) =>
        current && {
          ...current,
          stocks: current.stocks.map((stock) => ({
            ...stock,
            show_in_tooltip: stock.code === code,
          })),
        },
    );
  const remove = (code: string) =>
    setDraft(
      (current) =>
        current && {
          ...current,
          stocks: current.stocks
            .filter((stock) => stock.code !== code)
            .map((stock, index, stocks) => ({
              ...stock,
              show_in_tooltip:
                stock.show_in_tooltip ||
                (!stocks.some((item) => item.show_in_tooltip) && index === 0),
            })),
        },
    );
  async function add() {
    try {
      const config = await addStock(
        code,
        Number(holdings) || 0,
        cost ? Number(cost) : undefined,
      );
      setDraft(structuredClone(config));
      setCode("");
      setHoldings("0");
      setCost("");
      setMessage("股票已添加并保存");
    } catch (error) {
      setMessage(String(error));
    }
  }
  return (
    <div className="stack">
      <section className="summary-strip">
        <Metric
          label="今日盈亏"
          value={state.summary ? signed(state.summary.total_daily_pnl) : "-"}
          toneValue={state.summary?.total_daily_pnl}
        />
        <Metric
          label="持仓昨市值"
          value={compactAmount(state.summary?.total_prev_value ?? 0)}
        />
        <Metric label="持仓数量" value={`${draft.stocks.length}`} />
        <Metric
          label="市场主导"
          value={state.market.current?.leader_label ?? "-"}
        />
      </section>
      <section className="panel holdings-manager">
        <SectionTitle
          title="自选与持仓"
          detail={`已跟踪 ${draft.stocks.length} 只股票`}
        />
        <div className="add-stock">
          <input
            aria-label="股票代码"
            placeholder="股票代码，如 600519"
            value={code}
            onChange={(event) => setCode(event.target.value)}
          />
          <input
            aria-label="持仓数量"
            type="number"
            min="0"
            step="100"
            value={holdings}
            onChange={(event) => setHoldings(event.target.value)}
          />
          <input
            aria-label="成本价"
            type="number"
            step="0.001"
            placeholder="成本价"
            value={cost}
            onChange={(event) => setCost(event.target.value)}
          />
          <button disabled={!code.trim()} onClick={add}>添加股票</button>
        </div>
        <div className="stock-sort-actions">
          <span>排序</span>
          <button className="ghost" onClick={() => sortBy("holdings")}>
            持仓{sort?.field === "holdings" ? (sort.desc ? " ↓" : " ↑") : ""}
          </button>
          <button className="ghost" onClick={() => sortBy("change_percent")}>
            涨跌幅
            {sort?.field === "change_percent" ? (sort.desc ? " ↓" : " ↑") : ""}
          </button>
          <small>按住 ⋮⋮ 拖动或使用箭头调整顺序</small>
        </div>
        <div className="holdings-editor">
          <div className="editor-row editor-head">
            <span>代码 / 名称</span>
            <span>最新价</span>
            <span>涨跌幅</span>
            <span>持仓数量</span>
            <span>成本价</span>
            <span>今日盈亏</span>
            <span>显示</span>
            <span>操作</span>
          </div>
          {draft.stocks.map((stock, index) => {
            const quote = quotes.get(stock.code);
            return (
              <div
                className={`editor-row ${dragging === stock.code ? "dragging" : ""}`}
                data-stock-code={stock.code}
                onPointerEnter={(event) => {
                  const source = dragSource.current;
                  if (event.buttons !== 1) {
                    dragSource.current = null;
                    setDragging(null);
                  } else if (source && source !== stock.code) {
                    reorder(
                      source,
                      index -
                        draft.stocks.findIndex(
                          (item) => item.code === source,
                        ),
                    );
                  }
                }}
                onPointerUp={() => {
                  dragSource.current = null;
                  setDragging(null);
                }}
                key={stock.code}
              >
                <span className="stock-identity">
                  <i
                    aria-hidden="true"
                    className="drag-handle"
                    onPointerDown={(event) => {
                      event.preventDefault();
                      dragSource.current = stock.code;
                      setDragging(stock.code);
                    }}
                  >⋮⋮</i>
                  <span>
                    <b>{stock.code.toUpperCase()}</b>
                    <small>{stock.name}</small>
                  </span>
                </span>
                <span>{quote?.price.toFixed(3) ?? "-"}</span>
                <span className={tone(quote?.change_percent ?? 0)}>
                  {quote ? signed(quote.change_percent, 2, "%") : "-"}
                </span>
                <input
                  aria-label={`${stock.name} 持仓数量`}
                  type="number"
                  min="0"
                  step="100"
                  value={stock.holdings}
                  onChange={(event) =>
                    update(stock.code, {
                      holdings: Math.max(
                        0,
                        Math.round(Number(event.target.value) / 100) * 100,
                      ),
                    })
                  }
                />
                <input
                  aria-label={`${stock.name} 成本价`}
                  type="number"
                  step="0.001"
                  value={stock.cost_price}
                  onChange={(event) =>
                    update(stock.code, {
                      cost_price: Number(event.target.value) || 0,
                    })
                  }
                />
                <span className={tone(quote?.daily_pnl ?? 0)}>
                  {quote ? signed(quote.daily_pnl) : "-"}
                </span>
                <span className="display-controls">
                  <label title="弹窗显示">
                    <input
                      type="checkbox"
                      checked={stock.show_in_popup}
                      onChange={(event) =>
                        update(stock.code, {
                          show_in_popup: event.target.checked,
                        })
                      }
                    />
                    弹窗
                  </label>
                  <label title="托盘提示股票">
                    <input
                      type="radio"
                      name="tooltip-stock"
                      checked={stock.show_in_tooltip}
                      onChange={() => selectTooltip(stock.code)}
                    />
                    提示
                  </label>
                </span>
                <span className="row-actions">
                  <button
                    className="ghost"
                    disabled={index === 0}
                    onClick={() => reorder(stock.code, -1)}
                    aria-label="上移"
                  >
                    ↑
                  </button>
                  <button
                    className="ghost"
                    disabled={index === draft.stocks.length - 1}
                    onClick={() => reorder(stock.code, 1)}
                    aria-label="下移"
                  >
                    ↓
                  </button>
                  <button
                    className="danger ghost"
                    onClick={() => remove(stock.code)}
                  >
                    删除
                  </button>
                </span>
              </div>
            );
          })}
        </div>
      </section>
    </div>
  );
}

const STYLE_TREND_SERIES = [
  { id: "young", label: "小登", index: 0 },
  { id: "middle", label: "中登", index: 1 },
  { id: "old", label: "老登", index: 2 },
] as const;
const MARKET_DAY_START = 9 * 60 + 30;
const MARKET_DAY_END = 15 * 60;
const MARKET_TIME_TICKS = ["09:30", "10:30", "11:30", "13:00", "14:00", "15:00"];

function marketMinute(time: string) {
  const [hour = 9, minute = 30] = time.split(":").map(Number);
  return Math.max(MARKET_DAY_START, Math.min(MARKET_DAY_END, hour * 60 + minute));
}

function scoreDomain(history: AppStatePayload["market"]["history"]) {
  const scores = history.flatMap((item) => item.scores.slice(0, 3)).filter(Number.isFinite);
  if (!scores.length) return { min: 40, max: 60 };
  const rawMin = Math.min(...scores);
  const rawMax = Math.max(...scores);
  const rawRange = Math.max(10, rawMax - rawMin);
  let min = Math.max(0, Math.floor((rawMin - Math.max(3, rawRange * .18)) / 5) * 5);
  let max = Math.min(100, Math.ceil((rawMax + Math.max(3, rawRange * .18)) / 5) * 5);
  if (max - min < 10) {
    min = Math.max(0, min - 5);
    max = Math.min(100, max + 5);
  }
  return { min, max };
}

function StyleTrendPanel({
  history,
  styles,
  compact = false,
  children,
}: {
  history: AppStatePayload["market"]["history"];
  styles: StyleAnalysis[];
  compact?: boolean;
  children?: React.ReactNode;
}) {
  const width = 720;
  const height = compact ? 150 : 190;
  const padding = { left: 34, right: 12, top: 12, bottom: 25 };
  const chartWidth = width - padding.left - padding.right;
  const chartHeight = height - padding.top - padding.bottom;
  const continuousHistory = history.filter((item) => item.phase !== "auction_final");
  const auction = [...history].reverse().find((item) => item.phase === "auction_final");
  const domain = scoreDomain(history);
  const x = (time: string) => padding.left + (marketMinute(time) - MARKET_DAY_START) / (MARKET_DAY_END - MARKET_DAY_START) * chartWidth;
  const y = (score: number) => padding.top + (domain.max - Math.max(domain.min, Math.min(domain.max, score))) / (domain.max - domain.min) * chartHeight;
  const yTicks = Array.from({ length: 5 }, (_, index) => domain.min + (domain.max - domain.min) * index / 4);
  const first = continuousHistory[0] ?? history[0];
  const latest = continuousHistory[continuousHistory.length - 1] ?? history[history.length - 1];
  const seriesDefinitions = STYLE_TREND_SERIES.map((series) => ({
    ...series,
    label: styles.find((style) => style.id === series.id)?.label ?? series.label,
  }));
  const ranked = seriesDefinitions
    .map((series) => ({ ...series, score: latest?.scores[series.index] ?? 0 }))
    .sort((a, b) => b.score - a.score);
  const leaderGap = ranked.length > 1 ? ranked[0].score - ranked[1].score : 0;

  return (
    <section className={`panel style-trend-panel ${compact ? "compact" : ""}`}>
      <SectionTitle
        title="今日风格走势"
        detail={history.length
          ? `${history.length} 个快照 · ${ranked[0].label}当前领先 ${leaderGap.toFixed(1)} 分`
          : "等待首个有效快照"}
      />
      {history.length ? (
        <div className="style-trend-content">
          <div className="style-trend-chart">
            <svg aria-label={`${seriesDefinitions.map((series) => series.label).join("、")}今日分数走势`} role="img" viewBox={`0 0 ${width} ${height}`}>
              <rect className="trend-lunch" x={x("11:30")} y={padding.top} width={x("13:00") - x("11:30")} height={chartHeight} />
              <text className="trend-lunch-label" textAnchor="middle" x={(x("11:30") + x("13:00")) / 2} y={padding.top + 12}>午间休市</text>
              {yTicks.map((score) => (
                <g className={score <= 50 && score + (domain.max - domain.min) / 4 > 50 ? "trend-baseline" : ""} key={score}>
                  <line x1={padding.left} x2={width - padding.right} y1={y(score)} y2={y(score)} />
                  <text textAnchor="end" x={padding.left - 8} y={y(score) + 4}>{score.toFixed(0)}</text>
                </g>
              ))}
              {seriesDefinitions.map((series) => {
                const sessions = [
                  continuousHistory.filter((item) => marketMinute(item.time) <= 11 * 60 + 30),
                  continuousHistory.filter((item) => marketMinute(item.time) >= 13 * 60),
                ];
                return (
                  <g className={`trend-series ${series.id}`} key={series.id}>
                    {sessions.map((items, sessionIndex) => (
                      <polyline key={sessionIndex} points={items.map((item) => `${x(item.time)},${y(item.scores[series.index] ?? 0)}`).join(" ")} />
                    ))}
                    {continuousHistory.map((item, index) => (
                      <circle cx={x(item.time)} cy={y(item.scores[series.index] ?? 0)} key={`${item.time}-${series.id}`} r={index === continuousHistory.length - 1 ? 4 : 2.5}>
                        <title>{`${item.time.slice(0, 5)} ${series.label} ${(item.scores[series.index] ?? 0).toFixed(1)}`}</title>
                      </circle>
                    ))}
                    {auction && (
                      <rect className="trend-auction-point" height="7" width="7" x={x("09:30") - 3.5} y={y(auction.scores[series.index] ?? 0) - 3.5} transform={`rotate(45 ${x("09:30")} ${y(auction.scores[series.index] ?? 0)})`}>
                        <title>{`竞价 ${series.label} ${(auction.scores[series.index] ?? 0).toFixed(1)}`}</title>
                      </rect>
                    )}
                  </g>
                );
              })}
              {MARKET_TIME_TICKS.map((time, index) => (
                <text className="trend-time" key={time} textAnchor={index === 0 ? "start" : index === MARKET_TIME_TICKS.length - 1 ? "end" : "middle"} x={x(time)} y={height - 4}>
                  {time}
                </text>
              ))}
            </svg>
          </div>
          <div className="style-trend-legend">
            {seriesDefinitions.map((series) => {
              const value = latest?.scores[series.index] ?? 0;
              const change = value - (first?.scores[series.index] ?? value);
              return (
                <div className={series.id} key={series.id}>
                  <i />
                  <span><b>{series.label}</b><small>较首点 {signed(change, 1)}</small></span>
                  <strong>{value.toFixed(0)}</strong>
                </div>
              );
            })}
          </div>
        </div>
      ) : (
        <p className="empty">首个有效行情快照生成后，将显示三类风格的日内变化曲线</p>
      )}
      {children}
    </section>
  );
}

function MarketPage({ state }: { state: AppStatePayload }) {
  const market = state.market.current;
  const [selectedStyleId, setSelectedStyleId] = useState<string | null>(null);
  const [selectedSubsector, setSelectedSubsector] = useState<string | null>(null);
  if (!market)
    return (
      <section className="panel empty-state">
        <h2>尚未生成市场风格结果</h2>
        <p>{state.market.last_error || "点击“刷新全部”获取首个有效快照。"}</p>
      </section>
    );
  const focusedStyle =
    market.styles.find((style) => style.id === selectedStyleId) ??
    market.styles.find((style) => style.id === market.leader) ??
    [...market.styles].sort((a, b) => b.score - a.score)[0];
  const detailedContributions = selectedSubsector
    ? (focusedStyle?.contributions ?? [...(focusedStyle?.positive ?? []), ...(focusedStyle?.negative ?? [])])
        .filter((item) => item.subsector === selectedSubsector)
    : [];
  return (
    <div className="stack market-page">
      <section className="market-headline">
        <div>
          <span>
            {market.status === "dominant"
              ? "当前主导"
              : market.status === "proxy"
                ? "代理样本倾向"
                : market.status === "derived"
                  ? market.quality.broad_index_received >= 3
                    ? "板块指数暂缺 · 宽基+成分推断"
                    : "指数暂缺 · 成分推断"
                : market.status === "auction"
                  ? "竞价观察 · 不确认主导"
                : market.status === "relative"
                  ? "相对占优 · 尚未形成主线"
                : "当前状态"}
          </span>
          <h1>{market.leader_label}</h1>
        </div>
        <Metric
          label="轮动 / 稳定性"
          value={`${market.rotation_label} / ${market.stability.toFixed(0)}`}
        />
        <Metric
          label="最低分类覆盖"
          value={`${market.quality.minimum_style_coverage.toFixed(1)}%`}
        />
        <Metric
          label="数据质量"
          value={`${sampleSourceLabel(market.quality.sample_source)} / ${modeLabel(market.quality.mode)}`}
        />
        <Metric
          label="样本定义"
          value={`${market.quality.definition_version} / ${definitionSourceLabel(market.quality.definition_source)}`}
        />
        <Metric
          label="行情时间"
          value={`${market.trading_date} ${market.time}`}
        />
      </section>
      <section className="style-cards">
        {market.styles.map((style, index) => (
          <StyleCard
            active={focusedStyle?.id === style.id}
            coverage={market.quality.style_coverage[index] ?? 0}
            key={style.id}
            leader={market.leader === style.id}
            onSelect={() => setSelectedStyleId(style.id)}
            proxy={market.status === "proxy"}
            style={style}
          />
        ))}
      </section>
      <div className="market-detail-grid">
        <section className="panel">
          <SectionTitle
            title="贡献拆解"
            detail={focusedStyle ? `${focusedStyle.label} → ${focusedStyle.subtitle} · 流通市值有效 ${focusedStyle.float_cap_coverage.toFixed(1)}% · 点击细分查看全部样本${isProxySample(market.quality.sample_source) ? " · 当前为离线替代样本" : ""}` : ""}
          />
          {focusedStyle && (
            <div className="style-metrics">
              <Metric
                label="方向占比"
                value={`${directionLabel(focusedStyle.direction)} ${focusedStyle.directional_share.toFixed(0)}%`}
              />
              <Metric
                label="扩散 / 熵"
                value={`${focusedStyle.diffusion.toFixed(0)} / ${focusedStyle.entropy.toFixed(0)}`}
              />
              <Metric
                label="资金权-等权"
                value={signed(focusedStyle.weighting_divergence, 2, "%")}
              />
            </div>
          )}
          {focusedStyle?.subsectors.map((subsector) => (
            <button aria-pressed={selectedSubsector === subsector.id} className="subsector-row" key={subsector.id} onClick={() => setSelectedSubsector(subsector.id)} type="button">
              <div>
                <b>{subsectorLabel(subsector.name, isProxySample(market.quality.sample_source))}</b>
                <small>上涨广度 {subsector.breadth.toFixed(0)}%</small>
              </div>
              <Progress
                value={Math.min(100, Math.abs(subsector.contribution) * 5)}
              />
              <strong className={tone(subsector.contribution)}>
                {signed(subsector.contribution, 2)}
              </strong>
            </button>
          ))}
        </section>
        <section className="panel">
          <SectionTitle title="主要贡献" detail="原始贡献点" />
          <div className="dual-list">
            <div>
              <h3>正向贡献 TOP 5</h3>
              {focusedStyle?.positive.map((item) => (
                <ContributionRow
                  item={item}
                  key={`p-${item.code}-${item.subsector}`}
                  proxy={isProxySample(market.quality.sample_source)}
                />
              ))}
            </div>
            <div>
              <h3>负向贡献 TOP 5</h3>
              {focusedStyle?.negative.map((item) => (
                <ContributionRow
                  item={item}
                  key={`n-${item.code}-${item.subsector}`}
                  proxy={isProxySample(market.quality.sample_source)}
                />
              ))}
            </div>
          </div>
        </section>
      </div>
      <StyleTrendPanel history={state.market.history} styles={market.styles}>
        <p className="trend-methodology">
          主贡献按交易日冻结的前收盘自由流通市值加权，单股不超过10%、前五大合计不超过40%；等权结果仅用于观察上涨广度。任一分类有效覆盖率低于80%、流通市值有效率低于95%、关键指数不足、时间戳缺失或盘中延迟超限时不输出结论；ST、新股和停牌不参与评分，涨跌停保留并标注。
          {market.quality.index_derived
            ? market.quality.broad_index_received >= 3
              ? " 独立板块指数暂不可用；宽基指数仍用于市场基准，风格方向由高覆盖率成分行情推断，不能视为板块指数确认。"
              : " 独立指数暂不可用，当前结果使用高覆盖率成分行情推断，不能视为指数确认。"
            : market.quality.index_cached
              ? " 当前使用10分钟内的最近有效指数快照。"
            : market.quality.index_error
            ? ` 指数错误：${market.quality.index_error}`
            : ""}{" "}
          结果不构成投资建议。
        </p>
      </StyleTrendPanel>
      {selectedSubsector && (
        <div className="modal-backdrop" role="presentation" onMouseDown={() => setSelectedSubsector(null)}>
          <section aria-labelledby="contribution-detail-title" aria-modal="true" className="contribution-modal" onMouseDown={(event) => event.stopPropagation()} role="dialog">
            <button className="modal-close" aria-label="关闭" onClick={() => setSelectedSubsector(null)}>×</button>
            <header>
              <span>{focusedStyle?.label} · 样本贡献明细</span>
              <h2 id="contribution-detail-title">{subsectorLabel(selectedSubsector, isProxySample(market.quality.sample_source))}</h2>
              <small>共 {detailedContributions.length} 条，按贡献值从高到低排列</small>
            </header>
            <div className="contribution-detail-list">
              {detailedContributions.map((item) => (
                <ContributionRow item={item} key={`${item.code}-${item.subsector}`} prefix={item.code} proxy={isProxySample(market.quality.sample_source)} />
              ))}
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

function StyleCard({
  style,
  active,
  leader,
  onSelect,
  proxy,
  coverage,
}: {
  style: StyleAnalysis;
  active: boolean;
  leader: boolean;
  onSelect: () => void;
  proxy: boolean;
  coverage: number;
}) {
  return (
    <article
      aria-pressed={active}
      className={`style-card ${style.id} ${active ? "active" : ""}`}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onSelect();
        }
      }}
      role="button"
      tabIndex={0}
    >
      <header>
        <div>
          <span>{style.label}</span>
          <h2>{style.score.toFixed(0)}</h2>
          <p>{style.subtitle}</p>
        </div>
        <b>
          {leader
            ? proxy
              ? "代理倾向"
              : "当前主导"
            : active
              ? "查看中"
            : styleStateLabel(style.state)}
        </b>
      </header>
      <div className="style-metrics">
        <Metric label="独立强度" value={style.score.toFixed(0)} />
        <Metric label="相对偏好" value={style.preference.toFixed(0)} />
        <Metric label="上涨广度" value={`${style.breadth.toFixed(0)}%`} />
      </div>
      <Progress value={style.score} />
      <footer>
        <span>变化 {signed(style.score_change, 1)}</span>
        <span>覆盖 {coverage.toFixed(1)}%</span>
        <span>前五权重 {style.top_five_weight.toFixed(1)}%</span>
      </footer>
    </article>
  );
}

function SettingsPage({
  draft,
  setDraft,
  state,
  setMessage,
}: {
  draft: AppConfig;
  setDraft: React.Dispatch<React.SetStateAction<AppConfig | null>>;
  state: AppStatePayload;
  setMessage: (value: string) => void;
}) {
  const [coffeeOpen, setCoffeeOpen] = useState(false);
  const [dataSourceEditor, setDataSourceEditor] = useState<ExternalDataSourceConfig | null>(null);
  const [sourceTesting, setSourceTesting] = useState<string | null>(null);
  const [sourceTests, setSourceTests] = useState<Record<string, DataSourceTestResult>>({});
  const [draggingSource, setDraggingSource] = useState<string | null>(null);
  const [storageInfo, setStorageInfo] = useState<MarketStorageInfo | null>(null);
  const [storageBusy, setStorageBusy] = useState(false);
  const sourceDrag = useRef<string | null>(null);
  useEffect(() => {
    getMarketStorageInfo()
      .then(setStorageInfo)
      .catch((error) => setMessage(`读取历史存储失败：${String(error)}`));
  }, [state.market.current?.trading_date, state.market.history.length, setMessage]);
  const patch = (value: Partial<AppConfig>) =>
    setDraft((current) => current && { ...current, ...value });
  const appearance = (value: Partial<AppConfig["appearance"]>) =>
    setDraft(
      (current) =>
        current && {
          ...current,
          appearance: { ...current.appearance, ...value },
        },
    );
  const popup = (value: Partial<AppConfig["popup"]>) =>
    setDraft(
      (current) =>
        current && { ...current, popup: { ...current.popup, ...value } },
    );
  function openNewDataSource() {
    setDataSourceEditor({
      id: `futu-opend-${Date.now()}`,
      provider: "futu_opend",
      name: "富途 OpenD",
      host: "127.0.0.1",
      port: 32179,
      enabled: true,
    });
  }
  function keepDataSource(source: ExternalDataSourceConfig) {
    if (!source.name.trim() || !source.host.trim() || source.host.includes("://")) {
      setMessage("请填写名称和有效的主机地址（不要包含 http://）");
      return;
    }
    if (!Number.isInteger(source.port) || source.port < 1 || source.port > 65535) {
      setMessage("端口必须在 1 到 65535 之间");
      return;
    }
    setDraft((current) => {
      if (!current) return current;
      const exists = current.external_data_sources.some((item) => item.id === source.id);
      return {
        ...current,
        external_data_sources: exists
          ? current.external_data_sources.map((item) => item.id === source.id ? source : item)
          : [...current.external_data_sources, source],
        data_source_order: exists || current.data_source_order.includes(source.id)
          ? current.data_source_order
          : [source.id, ...current.data_source_order],
      };
    });
    setDataSourceEditor(null);
    setMessage("数据源已加入草稿，请点击保存并应用");
  }
  function removeDataSource(id: string) {
    setDraft((current) => current && ({
      ...current,
      external_data_sources: current.external_data_sources.filter((item) => item.id !== id),
      data_source_order: current.data_source_order.filter((key) => key !== id),
    }));
    setSourceTests((current) => {
      const next = { ...current };
      delete next[id];
      return next;
    });
    setMessage("数据源已从草稿移除，请点击保存并应用");
  }
  function reorderDataSource(sourceKey: string, targetKey: string) {
    if (sourceKey === targetKey) return;
    setDraft((current) => {
      if (!current) return current;
      const order = [...current.data_source_order];
      const from = order.indexOf(sourceKey);
      const to = order.indexOf(targetKey);
      if (from < 0 || to < 0) return current;
      const [moved] = order.splice(from, 1);
      order.splice(to, 0, moved);
      return { ...current, data_source_order: order };
    });
  }
  async function testSource(source: ExternalDataSourceConfig, key = source.id) {
    setSourceTesting(key);
    try {
      const result = await testDataSource(source);
      setSourceTests((current) => ({ ...current, [key]: result }));
      setMessage(`${source.name || "富途 OpenD"}：${result.message}${result.ok ? ` · ${result.latency_ms}ms` : ""}`);
    } catch (error) {
      setSourceTests((current) => ({
        ...current,
        [key]: { ok: false, latency_ms: 0, message: String(error) },
      }));
      setMessage(`连接测试失败：${String(error)}`);
    } finally {
      setSourceTesting(null);
    }
  }
  async function update() {
    try {
      const result = await checkAndInstallUpdate();
      setMessage(
        result.available
          ? `已安装 ${result.version}`
          : `当前已是最新版本 v${result.current_version}`,
      );
    } catch (error) {
      setMessage(`检查更新失败：${String(error)}`);
    }
  }
  async function clearSnapshots() {
    if (!window.confirm("确定清除今天的市场风格快照和走势记录吗？此操作无法撤销。")) return;
    try {
      await clearMarketSnapshots();
      setStorageInfo(await getMarketStorageInfo());
      setMessage("今日市场风格快照已清除");
    } catch (error) {
      setMessage(`清除失败：${String(error)}`);
    }
  }
  async function deleteStoredDay(tradingDate: string) {
    if (!window.confirm(`确定删除 ${tradingDate} 的市场风格历史吗？此操作无法撤销。`)) return;
    setStorageBusy(true);
    try {
      setStorageInfo(await deleteMarketHistoryDate(tradingDate));
      setMessage(`${tradingDate} 的历史记录已删除`);
    } catch (error) {
      setMessage(`删除失败：${String(error)}`);
    } finally {
      setStorageBusy(false);
    }
  }
  async function clearArchivedDays() {
    if (!window.confirm("确定删除全部已归档交易日吗？今天的数据会保留，此操作无法撤销。")) return;
    setStorageBusy(true);
    try {
      setStorageInfo(await clearMarketHistoryArchive());
      setMessage("历史归档已清空，今日记录已保留");
    } catch (error) {
      setMessage(`清理失败：${String(error)}`);
    } finally {
      setStorageBusy(false);
    }
  }
  return (
    <div className="settings-layout">
      <div className="stack">
        <section className="panel settings-card">
          <SectionTitle title="行情与刷新" />
          <SettingRow label="自选行情后台刷新">
            <select
              value={draft.background_refresh_interval_ms}
              onChange={(event) =>
                patch({
                  background_refresh_interval_ms: Number(event.target.value),
                })
              }
            >
              <option value={0}>关闭</option>
              <option value={5000}>5秒</option>
              <option value={10000}>10秒</option>
              <option value={30000}>30秒</option>
              <option value={60000}>60秒</option>
            </select>
          </SettingRow>
          <SettingRow label="市场风格分析">
            <label className="switch">
              <input
                type="checkbox"
                checked={draft.market_analysis.enabled}
                onChange={(event) =>
                  patch({
                    market_analysis: {
                      ...draft.market_analysis,
                      enabled: event.target.checked,
                    },
                  })
                }
              />
              <i />
            </label>
          </SettingRow>
          <SettingRow label="市场风格刷新频率">
            <SlidingButtons
              options={[[5, "5分钟"], [15, "15分钟 · 推荐"], [30, "30分钟"]]}
              value={draft.market_analysis.refresh_minutes}
              onChange={(minutes) => patch({ market_analysis: { ...draft.market_analysis, refresh_minutes: minutes } })}
            />
          </SettingRow>
          <div className="source-toolbar">
            <div>
              <b>行情数据源</b>
              <small>按顺序读取，失败时自动切换</small>
            </div>
            <button className="ghost" onClick={openNewDataSource} type="button">＋ 新增其他数据源</button>
          </div>
          <div className="data-source-stack">
            {draft.data_source_order.map((key) => {
              const source = draft.external_data_sources.find((item) => item.id === key);
              const isSystem = key === "eastmoney" || key === "tencent";
              if (!source && !isSystem) return null;
              const test = source ? sourceTests[source.id] : undefined;
              const onPointerEnter = (event: React.PointerEvent<HTMLElement>) => {
                const dragged = sourceDrag.current;
                if (event.buttons !== 1) {
                  sourceDrag.current = null;
                  setDraggingSource(null);
                } else if (dragged && dragged !== key) {
                  reorderDataSource(dragged, key);
                }
              };
              const finishDrag = () => {
                sourceDrag.current = null;
                setDraggingSource(null);
              };
              if (source) {
                return (
                  <article
                    className={`data-source-card external ${source.enabled ? "enabled" : "disabled"} ${draggingSource === key ? "dragging" : ""}`}
                    key={source.id}
                    onPointerEnter={onPointerEnter}
                    onPointerUp={finishDrag}
                  >
                    <i
                      aria-label={`拖动调整 ${source.name} 的优先顺序`}
                      className="source-drag-handle"
                      onPointerDown={(event) => {
                        event.preventDefault();
                        sourceDrag.current = key;
                        setDraggingSource(key);
                      }}
                    >⋮⋮</i>
                    <span className={`source-provider-icon futu ${test?.ok ? "online" : ""}`}>F</span>
                    <div className="external-source-copy">
                      <div className="source-name-line">
                        <b>{source.name || "富途 OpenD"}</b>
                        <span className={`source-state-pill ${test?.ok ? "online" : source.enabled ? "ready" : "off"}`}>
                          {test?.ok ? "连接正常" : source.enabled ? "已启用" : "已停用"}
                        </span>
                      </div>
                      <small>OpenD · {source.host}:{source.port}</small>
                    </div>
                    <div className="source-card-role">
                      <label className="switch" title="是否启用此数据源">
                        <input
                          aria-label={`启用 ${source.name}`}
                          checked={source.enabled}
                          onChange={(event) => setDraft((current) => current && ({
                            ...current,
                            external_data_sources: current.external_data_sources.map((item) => item.id === source.id ? { ...item, enabled: event.target.checked } : item),
                          }))}
                          type="checkbox"
                        />
                        <i />
                      </label>
                    </div>
                    {test && <div className={`source-test-note ${test.ok ? "source-ok" : "source-error"}`}>{test.message}{test.ok ? ` · ${test.latency_ms}ms` : ""}</div>}
                    <div className="external-source-actions">
                      <button disabled={sourceTesting === source.id} onClick={() => testSource(source)} type="button">
                        {sourceTesting === source.id ? "测试中" : "测试连接"}
                      </button>
                      <button onClick={() => setDataSourceEditor({ ...source })} type="button">编辑</button>
                      <button className="danger" onClick={() => removeDataSource(source.id)} type="button">删除</button>
                    </div>
                  </article>
                );
              }
              const eastmoney = key === "eastmoney";
              return (
                <article
                  className={`data-source-card system enabled ${draggingSource === key ? "dragging" : ""}`}
                  key={key}
                  onPointerEnter={onPointerEnter}
                  onPointerUp={finishDrag}
                >
                  <i
                    aria-label={`拖动调整${eastmoney ? "东方财富" : "腾讯行情"}的优先顺序`}
                    className="source-drag-handle"
                    onPointerDown={(event) => {
                      event.preventDefault();
                      sourceDrag.current = key;
                      setDraggingSource(key);
                    }}
                  >⋮⋮</i>
                  <span className={`source-provider-icon ${eastmoney ? "eastmoney" : "tencent"}`}>{eastmoney ? "东" : "腾"}</span>
                  <div className="external-source-copy">
                    <div className="source-name-line">
                      <b>{eastmoney ? "东方财富" : "腾讯行情"}</b>
                      <span className={`source-state-pill ${eastmoney ? "online" : "standby"}`}>{eastmoney ? "在线" : "备用"}</span>
                    </div>
                    <small>{eastmoney ? "A 股行情 · 板块成分与指数证据" : "A 股快照 · 宽基指数备用"}</small>
                  </div>
                </article>
              );
            })}
          </div>
          <p>按住卡片左侧拖动柄调整读取顺序；连接、握手或单个标的数据失败时，将继续尝试下一数据源。</p>
          <p>市场风格使用独立定时器，不影响托盘行情刷新。</p>
        </section>
        <section className="panel settings-card">
          <SectionTitle title="托盘弹窗" />
          <FieldChecklist
            fields={draft.display_fields}
            options={QUOTE_FIELD_OPTIONS}
            onToggle={(field, checked) => setDraft((current) => current && toggleField(current, 'display_fields', field, checked, ['price']))}
          />
          <SettingRow label="显示今日合计">
            <label className="switch">
              <input
                type="checkbox"
                checked={draft.show_daily_summary}
                onChange={(event) =>
                  patch({ show_daily_summary: event.target.checked })
                }
              />
              <i />
            </label>
          </SettingRow>
          <SettingRow label={`自动隐藏 ${autoHideLabel(draft.popup.auto_hide_ms)}`}>
            <div className="auto-hide-control">
              <input
                aria-label="自动隐藏时长"
                type="range"
                min="0"
                max="30"
                step="0.1"
                value={draft.popup.auto_hide_ms / 1000}
                onChange={(event) => popup({ auto_hide_ms: Math.round(Number(event.target.value) * 1000) })}
              />
              <output>{autoHideLabel(draft.popup.auto_hide_ms)}</output>
            </div>
          </SettingRow>
        </section>
        <section className="panel settings-card">
          <SectionTitle title="托盘提示字段" detail={`${draft.tooltip_fields.length} 个指标`} />
          <FieldChecklist
            fields={draft.tooltip_fields}
            options={TOOLTIP_FIELD_OPTIONS}
            onToggle={(field, checked) => setDraft((current) => current && toggleField(current, 'tooltip_fields', field, checked, DEFAULT_TOOLTIP_FIELDS))}
          />
        </section>
      </div>
      <div className="stack">
        <section className="panel settings-card">
          <SectionTitle title="外观" />
          <SettingRow label="主题">
            <SlidingButtons
              className="theme-options"
              options={[["system", "跟随系统"], ["dark", "深色"], ["light", "浅色"], ["soft", "低对比"], ["morandi", "莫兰迪"]]}
              value={draft.appearance.theme_mode}
              onChange={(theme_mode) => appearance({ theme_mode })}
            />
          </SettingRow>
          <SettingRow
            label={`窗口透明度 ${Math.round(draft.appearance.popup_tint_opacity * 100)}%`}
          >
            <input
              type="range"
              min="0.2"
              max="0.95"
              step="0.01"
              value={draft.appearance.popup_tint_opacity}
              onChange={(event) =>
                appearance({ popup_tint_opacity: Number(event.target.value) })
              }
            />
          </SettingRow>
          <SettingRow label="上涨 / 下跌颜色">
            <div className="color-picks">
              <input
                aria-label="上涨颜色"
                type="color"
                value={draft.popup.up_color}
                onChange={(event) => popup({ up_color: event.target.value })}
              />
              <input
                aria-label="下跌颜色"
                type="color"
                value={draft.popup.down_color}
                onChange={(event) => popup({ down_color: event.target.value })}
              />
            </div>
          </SettingRow>
          <SettingRow label={`圆角 ${draft.appearance.corner_radius}px`}>
            <input
              type="range"
              min="0"
              max="24"
              value={draft.appearance.corner_radius}
              onChange={(event) =>
                appearance({ corner_radius: Number(event.target.value) })
              }
            />
          </SettingRow>
          <SettingRow label="动态效果">
            <label className="switch">
              <input
                type="checkbox"
                checked={draft.appearance.animations_enabled}
                onChange={(event) => appearance({ animations_enabled: event.target.checked })}
              />
              <i />
            </label>
          </SettingRow>
        </section>
        <section className="panel settings-card">
          <SectionTitle title="市场风格历史库" detail="默认长期保留" />
          <div className="storage-summary">
            <span><b>{storageInfo?.total_days ?? "—"}</b><small>交易日</small></span>
            <span><b>{storageInfo?.trend_points ?? "—"}</b><small>趋势点</small></span>
            <span><b>{formatStorageSize(storageInfo?.size_bytes ?? 0)}</b><small>本地占用</small></span>
          </div>
          <p>每天保留一份最终完整分析和全部盘中趋势点；新交易日会自动归档，不再删除旧数据。</p>
          {storageInfo && storageInfo.days.length > 0 && (
            <div className="storage-day-list">
              {storageInfo.days.map((day) => (
                <div key={day.trading_date}>
                  <span>
                    <b>{day.trading_date}</b>
                    <small>{day.is_current ? "今日 · " : ""}{day.trend_points} 个趋势点{day.leader_label ? ` · ${day.leader_label}` : ""}</small>
                  </span>
                  {day.is_current
                    ? <em>记录中</em>
                    : <button disabled={storageBusy} onClick={() => deleteStoredDay(day.trading_date)} type="button">删除</button>}
                </div>
              ))}
            </div>
          )}
          <div className="storage-actions">
            <button className="ghost" onClick={clearSnapshots} type="button">清除今日记录</button>
            <button className="ghost danger" disabled={storageBusy || !storageInfo?.archived_days} onClick={clearArchivedDays} type="button">清空历史归档</button>
          </div>
        </section>
        <section className="panel settings-card about-card">
          <SectionTitle title="关于 StockTray" detail={`v${state.app_version}`} />
          <p>轻量的 A 股托盘行情、持仓盈亏与市场风格分析工具。</p>
          <div className="release-notes">
            <b>本版本主要更新</b>
            <span>市场风格分析与今日风格走势</span>
            <span>全新主题、无边框窗口和交互动效</span>
            <span>持仓拖动排序与窄窗口适配</span>
          </div>
          <div className="about-actions">
            <button className="primary" onClick={() => setCoffeeOpen(true)}>☕ 请作者喝咖啡</button>
            <button onClick={update}>检查更新</button>
          </div>
        </section>
      </div>
      {dataSourceEditor && createPortal(
        <div className="modal-backdrop" role="presentation" onMouseDown={() => setDataSourceEditor(null)}>
          <section aria-labelledby="data-source-title" aria-modal="true" className="data-source-modal" onMouseDown={(event) => event.stopPropagation()} role="dialog">
            <button aria-label="关闭" className="modal-close" onClick={() => setDataSourceEditor(null)} type="button">×</button>
            <header>
              <span className="source-provider-icon">F</span>
              <div>
                <h2 id="data-source-title">配置富途 OpenD</h2>
                <p>填写 OpenD 对客户端开放的主机和 API 端口。</p>
              </div>
            </header>
            <form onSubmit={(event) => { event.preventDefault(); keepDataSource(dataSourceEditor); }}>
              <label>
                <span>数据源类型</span>
                <select disabled value={dataSourceEditor.provider}>
                  <option value="futu_opend">富途 OpenD</option>
                </select>
              </label>
              <label>
                <span>显示名称</span>
                <input
                  autoFocus
                  maxLength={40}
                  onChange={(event) => setDataSourceEditor((current) => current && ({ ...current, name: event.target.value }))}
                  placeholder="例如：服务器 OpenD"
                  value={dataSourceEditor.name}
                />
              </label>
              <div className="data-source-address">
                <label>
                  <span>主机 / IP</span>
                  <input
                    onChange={(event) => setDataSourceEditor((current) => current && ({ ...current, host: event.target.value }))}
                    placeholder="127.0.0.1 或服务器域名"
                    spellCheck={false}
                    value={dataSourceEditor.host}
                  />
                </label>
                <label>
                  <span>API 端口</span>
                  <input
                    max={65535}
                    min={1}
                    onChange={(event) => setDataSourceEditor((current) => current && ({ ...current, port: Number(event.target.value) }))}
                    type="number"
                    value={dataSourceEditor.port}
                  />
                </label>
              </div>
              <label className="data-source-enabled">
                <span>
                  <b>启用此配置</b>
                  <small>保存并启用后，富途将成为行情刷新首选数据源</small>
                </span>
                <span className="switch">
                  <input
                    checked={dataSourceEditor.enabled}
                    onChange={(event) => setDataSourceEditor((current) => current && ({ ...current, enabled: event.target.checked }))}
                    type="checkbox"
                  />
                  <i />
                </span>
              </label>
              <div className="data-source-hint">
                <b>服务器安全提示</b>
                <span>若 OpenD 仅绑定 127.0.0.1，请先建立 SSH 隧道，再填写 127.0.0.1:32179。无需在客户端填写富途账号或密码。</span>
              </div>
              {sourceTests.__editor && (
                <div className={`data-source-test-result ${sourceTests.__editor.ok ? "ok" : "error"}`}>
                  <b>{sourceTests.__editor.ok ? "端口可达" : "连接失败"}</b>
                  <span>{sourceTests.__editor.message}{sourceTests.__editor.ok ? ` · ${sourceTests.__editor.latency_ms}ms` : ""}</span>
                </div>
              )}
              <footer>
                <button disabled={sourceTesting === "__editor"} onClick={() => testSource(dataSourceEditor, "__editor")} type="button">
                  {sourceTesting === "__editor" ? "正在测试…" : "测试连接"}
                </button>
                <button className="primary" type="submit">添加到配置</button>
              </footer>
            </form>
          </section>
        </div>,
        document.body,
      )}
      {coffeeOpen && createPortal(
        <div className="modal-backdrop" role="presentation" onMouseDown={() => setCoffeeOpen(false)}>
          <section aria-labelledby="coffee-title" aria-modal="true" className="coffee-modal" onMouseDown={(event) => event.stopPropagation()} role="dialog">
            <button aria-label="关闭" className="modal-close" onClick={() => setCoffeeOpen(false)}>×</button>
            <h2 id="coffee-title">请作者喝杯咖啡</h2>
            <p>感谢你的支持，这会帮助 StockTray 持续改进。</p>
            <img alt="StockTray 项目支持二维码" src={SUPPORT_QR} />
            <small>扫码访问项目主页</small>
          </section>
        </div>,
        document.body,
      )}
    </div>
  );
}

function HoldingsTable({ items }: { items: DailyPnlItem[] }) {
  if (!items.length) return <div className="empty">暂无持仓行情</div>;
  return (
    <div className="simple-table">
      <div>
        <span>代码 / 名称</span>
        <span>最新价</span>
        <span>涨跌幅</span>
        <span>今日盈亏</span>
      </div>
      {items.map((item) => (
        <div key={item.code}>
          <span>
            <b>{item.code.toUpperCase()}</b>
            <small>{item.name}</small>
          </span>
            <span>{item.price.toFixed(3)}</span>
          <span className={tone(item.change_percent)}>
            {signed(item.change_percent, 2, "%")}
          </span>
          <span className={tone(item.daily_pnl)}>{signed(item.daily_pnl)}</span>
        </div>
      ))}
    </div>
  );
}

function Metric({
  label,
  value,
  toneValue,
}: {
  label: string;
  value: string;
  toneValue?: number;
}) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong className={toneValue === undefined ? "" : tone(toneValue)}>
        {value}
      </strong>
    </div>
  );
}
function Progress({ value }: { value: number }) {
  const normalized = Math.max(0, Math.min(100, value));
  return (
    <div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(normalized)}>
      <i style={{ width: `${normalized}%` }} />
    </div>
  );
}
function SectionTitle({
  title,
  detail = "",
}: {
  title: string;
  detail?: string;
}) {
  return (
    <header className="section-title">
      <h2>{title}</h2>
      <span>{detail}</span>
    </header>
  );
}
function SettingRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="setting-row">
      <span>{label}</span>
      <div>{children}</div>
    </div>
  );
}
function FieldChecklist({ fields, options, onToggle }: { fields: string[]; options: Array<{ key: FieldKey; label: string }>; onToggle: (field: FieldKey, checked: boolean) => void }) {
  return <div className="field-list">{options.map((option) => <label key={option.key}><input type="checkbox" checked={fields.includes(option.key)} onChange={(event) => onToggle(option.key, event.target.checked)} /><span>{option.label}</span></label>)}</div>;
}
function EmptyMarket() {
  return <div className="empty">等待市场行情</div>;
}
function ContributionRow({
  item,
  prefix,
  proxy = false,
}: {
  item: MarketContribution;
  prefix?: string;
  proxy?: boolean;
}) {
  const subsector = subsectorLabel(item.subsector, proxy);
  return (
    <div className="contribution-row">
      <div>
        <b>{item.name || item.code}</b>
        <small title={`${prefix ? `${prefix} · ` : ""}${subsector} · 权重 ${item.stock_weight_percent.toFixed(2)}% · 信号 ${signed(item.signal_score, 1)} · 贡献占比 ${item.contribution_share.toFixed(1)}%`}>
          {prefix ? `${prefix} · ` : ""}
          {subsector} · 权重 {item.stock_weight_percent.toFixed(2)}% · 贡献占比 {item.contribution_share.toFixed(1)}%
        </small>
        <small className="contribution-reason" title={`${item.reason} · 竞价 ${signed(item.gap_percent, 2, "%")} · 盘中 ${signed(item.intraday_percent, 2, "%")}`}>
          信号 {signed(item.signal_score, 1)} · {item.reason}
        </small>
      </div>
      <span className={tone(item.contribution)}>
        {signed(item.contribution, 2)}
      </span>
      <em className={tone(item.change_percent)}>
        {signed(item.change_percent, 2, "%")}
      </em>
    </div>
  );
}
function subsectorLabel(name: string, proxy: boolean) {
  if (!proxy) return name;
  return ({
    "电子代理": "AI硬件（电子样本）",
    "通信代理": "光通信（通信样本）",
    "计算机代理": "算力基础设施（计算机样本）",
    "机械设备代理": "机器人（机械设备样本）",
    "传媒代理": "游戏（传媒样本）",
    "国防军工代理": "商业航天（军工样本）",
  } as Record<string, string>)[name] ?? name;
}
function modeLabel(mode: string) {
  return mode === "fallback" ? "部分备用源" : mode === "cached_index" ? "指数短时缓存" : mode === "derived_index" ? "宽基+成分推断" : "完整行情";
}
function sampleSourceLabel(source: string) {
  return source === "online_exact"
    ? "在线精确样本"
    : source === "cache_exact"
      ? "缓存精确样本"
      : source === "offline_proxy"
        ? "离线行业代理"
        : source === "remote_fallback"
          ? "远程签名兜底样本"
        : "样本待确认";
}
function definitionSourceLabel(source: string) {
  return source === "remote_signed" ? "远程签名" : "程序内置";
}
function isProxySample(source: string) {
  return source === "offline_proxy" || source === "remote_fallback";
}
function styleStateLabel(state: string) {
  return state === "strong" ? "强势" : state === "weak" ? "偏弱" : "中性";
}
function directionLabel(direction: string) {
  return direction === "positive"
    ? "正向"
    : direction === "negative"
      ? "负向"
      : "混合";
}
function marketStatusText(state: AppStatePayload) {
  const market = state.market.current;
  return market
    ? `${market.trading_date} ${market.time} · 覆盖率 ${market.quality.coverage.toFixed(1)}%`
    : state.market.last_error || "等待首次分析";
}
function autoHideLabel(milliseconds: number) {
  return milliseconds === 0 ? "关闭" : `${(milliseconds / 1000).toFixed(1)} 秒`;
}
function formatStorageSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(bytes < 10 * 1024 * 1024 ? 1 : 0)} MB`;
}
function holdingWinRate(items: DailyPnlItem[]) {
  if (!items.length) return "-";
  return `${((items.filter((item) => item.daily_pnl > 0).length / items.length) * 100).toFixed(0)}%`;
}

function AppRouter() {
  const route =
    new URLSearchParams(window.location.search).get("view") ||
    window.location.hash.replace(/^#\/?/, "");
  return route === "popup" ||
    window.location.pathname.endsWith("/popup.html") ? (
    <PopupApp />
  ) : (
    <DesktopApp />
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppRouter />
  </React.StrictMode>,
);

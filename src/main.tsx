import React, { useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { createPortal } from "react-dom";
import {
  addStock,
  checkAndInstallUpdate,
  getState,
  hidePopup,
  onState,
  refreshMarketAnalysis,
  refreshQuotes,
  saveSettings,
  setPopupHovered,
  closeWindow,
  minimizeWindow,
  startWindowDragging,
  toggleMaximizeWindow,
} from "./tauri";
import type {
  AppConfig,
  AppStatePayload,
  DailyPnlItem,
  MarketContribution,
  StockEntry,
  StyleAnalysis,
} from "./types";
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
const SUPPORT_QR = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAANwAAADcAQMAAAAhlF3CAAAABlBMVEX///8AAABVwtN+AAAACXBIWXMAAA7EAAAOxAGVKw4bAAABnUlEQVRYhd2YPY6EMAyFH6KgzBE4yhwtHI2jcISUFAiPnx2yo13t9HYKJPOlcfzznADfVhWuEyjHuquJSa7Z/skRDdKf+VxaOejYBkzXfPLnmgyKPNb+2tD36hFEhWZ5GIeZEaI04CVbDQ17ANsq+395mwF6q2EJMm8r7ulPHwoCn1VovEToJ36vBNATVT9N40frxnwtZ3nMQBCgBRM62at0oWuWmqngJF3QGU5h3gJL6xGNBHUYudUwx7SdABpQeu3hzQMh3iaLWKuBdOiNJhbUAMq1WDIygjJZxxx5mwlqzZ0eziq2F4uMQ4gD1YQHkAKweUXST2+iaaCV3Dy2bpq3pg49qSNBG0bYTSgAOk5qczHZKz5OJoIjgkMdZoN2CLGgsOR0GPEatL2cjF0dEsHbLEJXBz+S4u8JkWBfTw+tPhkv3JAKVvP549I2PZdTIBqkdzabiF9mqHMcJ1c5ckF/QGimB3ok9+R564cQEforjz+NwCT8w89EsPWbTh8nvTyjQfRXnn6ZYW6OS3giOFoN1YEleONH0PPAb+sN9xiyThJMLToAAAAASUVORK5CYII=";

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
                市场风格 · {market?.status === "dominant" ? "当前主导" : market?.status === "proxy" ? "代理倾向" : "观察中"}
              </small>
              <strong>{market?.leader_label ?? "等待分析"}</strong>
            </div>
            <span>{market ? `覆盖 ${market.quality.coverage.toFixed(0)}%` : "暂无快照"}</span>
          </header>
          <div className="popup-style-scores">
            {market?.styles.map((style) => (
              <div className={`${style.id} ${market.leader === style.id ? "active" : ""}`} key={style.id}>
                <span>{style.label}</span>
                <b>{style.score.toFixed(0)}</b>
                <Progress value={style.score} />
              </div>
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
  const [page, setPage] = useState<Page>("overview");
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
    return () => {
      delete document.body.dataset.view;
      delete document.documentElement.dataset.view;
      unlisten.then((fn) => fn()).catch(console.error);
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
            setState={setState}
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
              {market?.status === "dominant" ? "当前主导" : "当前倾向"}：
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
      <section className="panel evidence overview-evidence">
        <SectionTitle
          title="盘中证据"
          detail={`${state.market.history.length} 个快照 · 小登 / 中登 / 老登`}
        />
        {state.market.history.length ? (
          <div>
            {state.market.history.map((item) => (
              <div className="evidence-item" key={item.time}>
                <b>{item.time.slice(0, 5)}</b>
                <span>{item.leader ? styleLabel(item.leader) : "均衡/观察"}</span>
                <small>{item.scores.map((score) => score.toFixed(0)).join(" / ")}</small>
              </div>
            ))}
          </div>
        ) : (
          <p className="empty">等待首个盘中快照</p>
        )}
      </section>
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

function MarketPage({ state }: { state: AppStatePayload }) {
  const market = state.market.current;
  const [selectedStyleId, setSelectedStyleId] = useState<string | null>(null);
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
  return (
    <div className="stack market-page">
      <section className="market-headline">
        <div>
          <span>
            {market.status === "dominant"
              ? "当前主导"
              : market.status === "proxy"
                ? "代理样本倾向"
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
            detail={focusedStyle ? `${focusedStyle.label} → ${focusedStyle.subtitle}` : ""}
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
                label="市值权-等权"
                value={signed(focusedStyle.weighting_divergence, 2, "%")}
              />
            </div>
          )}
          {focusedStyle?.subsectors.map((subsector) => (
            <div className="subsector-row" key={subsector.id}>
              <div>
                <b>{subsector.name}</b>
                <small>上涨广度 {subsector.breadth.toFixed(0)}%</small>
              </div>
              <Progress
                value={Math.min(100, Math.abs(subsector.contribution) * 5)}
              />
              <strong className={tone(subsector.contribution)}>
                {signed(subsector.contribution, 2)}
              </strong>
            </div>
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
                />
              ))}
            </div>
            <div>
              <h3>负向贡献 TOP 5</h3>
              {focusedStyle?.negative.map((item) => (
                <ContributionRow
                  item={item}
                  key={`n-${item.code}-${item.subsector}`}
                />
              ))}
            </div>
          </div>
        </section>
      </div>
      <section className="panel evidence">
        <SectionTitle
          title="盘中证据"
          detail={`指数 ${market.quality.index_received}/${market.quality.index_expected} · 宽基 ${market.quality.broad_index_received}/5 · 排除 ST ${market.quality.excluded_st} / 新股 ${market.quality.excluded_new} / 停牌 ${market.quality.excluded_halted}`}
        />
        <div>
          {state.market.history.map((item) => (
            <div className="evidence-item" key={item.time}>
              <b>{item.time.slice(0, 5)}</b>
              <span>{item.leader ? styleLabel(item.leader) : "均衡/观察"}</span>
              <small>
                {item.scores.map((score) => score.toFixed(0)).join(" / ")}
              </small>
            </div>
          ))}
        </div>
        <p>
          任一分类有效覆盖率低于80%、关键指数不足、时间戳缺失或盘中延迟超限时不输出结论；ST、新股和停牌不参与评分，涨跌停保留并标注。
          {market.quality.index_error
            ? ` 指数错误：${market.quality.index_error}`
            : ""}{" "}
          结果不构成投资建议。
        </p>
      </section>
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
        <span>集中度 {style.concentration.toFixed(1)}%</span>
      </footer>
    </article>
  );
}

function SettingsPage({
  draft,
  setDraft,
  state,
  setState,
  setMessage,
}: {
  draft: AppConfig;
  setDraft: React.Dispatch<React.SetStateAction<AppConfig | null>>;
  state: AppStatePayload;
  setState: React.Dispatch<React.SetStateAction<AppStatePayload | null>>;
  setMessage: (value: string) => void;
}) {
  const [coffeeOpen, setCoffeeOpen] = useState(false);
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
  function showMock(leader: "young" | "middle" | "old") {
    if (!state.market.current) {
      setMessage("需要先获取一次市场行情，再应用演示场景");
      return;
    }
    const scores = {
      young: [86, 54, 31],
      middle: [48, 83, 57],
      old: [35, 51, 84],
    }[leader];
    const labels = { young: "小登", middle: "中登", old: "老登" };
    const current = structuredClone(state.market.current);
    current.leader = leader;
    current.leader_label = labels[leader];
    current.status = "dominant";
    current.styles.forEach((style, index) => {
      style.score_change = scores[index] - style.score;
      style.score = scores[index];
      style.state = style.id === leader ? "strong" : "neutral";
    });
    setState({
      ...state,
      market: {
        ...state.market,
        current,
        history: [
          { time: "09:45:00", leader: "young", scores: [71, 55, 43] },
          { time: "10:30:00", leader: "middle", scores: [62, 68, 49] },
          { time: "13:30:00", leader, scores },
        ],
      },
    });
    setMessage(`已载入${labels[leader]}演示数据（不会保存）`);
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
          <div className="source-status">
            <span>
              <i />
              东方财富 <b>主数据源</b>
            </span>
            <span>
              <i />
              腾讯行情 <b>备用</b>
            </span>
          </div>
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
          <SectionTitle title="数据与存储" />
          <SettingRow label="当日快照">
            <span>仅保存当前交易日</span>
          </SettingRow>
          <p>下一交易日首个有效行情到达后自动清除上一交易日数据。</p>
          <button className="ghost" disabled>
            清除今日快照
          </button>
        </section>
        <section className="panel settings-card demo-card">
          <SectionTitle title="功能演示" detail="仅修改当前界面，不保存" />
          <SlidingButtons
            options={[["young", "小登行情"], ["middle", "中登行情"], ["old", "老登行情"]]}
            value={(state.market.current?.leader as "young" | "middle" | "old") ?? "middle"}
            onChange={showMock}
          />
        </section>
        <section className="panel settings-card about-card">
          <SectionTitle title="关于 StockTray" detail={`v${state.app_version}`} />
          <p>轻量的 A 股托盘行情、持仓盈亏与市场风格分析工具。</p>
          <div className="release-notes">
            <b>本版本主要更新</b>
            <span>市场风格分析与盘中证据</span>
            <span>全新主题、无边框窗口和交互动效</span>
            <span>持仓拖动排序与窄窗口适配</span>
          </div>
          <div className="about-actions">
            <button className="primary" onClick={() => setCoffeeOpen(true)}>☕ 请作者喝咖啡</button>
            <button onClick={update}>检查更新</button>
          </div>
        </section>
      </div>
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
}: {
  item: MarketContribution;
  prefix?: string;
}) {
  return (
    <div className="contribution-row">
      <div>
        <b>{item.name || item.code}</b>
        <small title={`${prefix ? `${prefix} · ` : ""}${item.subsector} · ${item.reason}`}>
          {prefix ? `${prefix} · ` : ""}
          {item.subsector} · {item.reason}
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
function styleLabel(id: string) {
  return id === "young" ? "小登" : id === "middle" ? "中登" : "老登";
}
function modeLabel(mode: string) {
  return mode === "fallback" ? "部分备用源" : "完整行情";
}
function sampleSourceLabel(source: string) {
  return source === "online_exact"
    ? "在线精确样本"
    : source === "cache_exact"
      ? "缓存精确样本"
      : source === "offline_proxy"
        ? "离线行业代理"
        : "样本待确认";
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

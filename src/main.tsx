import React, { useEffect, useMemo, useRef, useState } from 'react';
import ReactDOM from 'react-dom/client';
import { addStock, checkAndInstallUpdate, getState, hidePopup, onState, refreshQuotes, saveSettings, setPopupHovered } from './tauri';
import type { AppConfig, AppStatePayload, DailyPnlItem, StockEntry } from './types';
import './styles.css';

type FieldKey =
  | 'name'
  | 'code'
  | 'price'
  | 'prev_close'
  | 'open'
  | 'high'
  | 'low'
  | 'change'
  | 'change_percent'
  | 'volume'
  | 'amount'
  | 'volume_ratio'
  | 'turnover'
  | 'holdings'
  | 'cost_price'
  | 'daily_pnl'
  | 'daily_pnl_percent'
  | 'position_pnl'
  | 'position_pnl_percent';

const QUOTE_FIELD_OPTIONS: Array<{ key: FieldKey; label: string }> = [
  { key: 'price', label: '最新价' },
  { key: 'change', label: '涨跌额' },
  { key: 'change_percent', label: '涨跌幅' },
  { key: 'prev_close', label: '昨收' },
  { key: 'open', label: '今开' },
  { key: 'high', label: '最高' },
  { key: 'low', label: '最低' },
  { key: 'volume', label: '成交量' },
  { key: 'amount', label: '成交额' },
  { key: 'volume_ratio', label: '量比' },
  { key: 'turnover', label: '换手率' },
  { key: 'holdings', label: '持仓' },
  { key: 'cost_price', label: '成本' },
  { key: 'daily_pnl', label: '当日盈亏' },
  { key: 'daily_pnl_percent', label: '当日盈亏比' },
  { key: 'position_pnl', label: '持仓盈亏' },
  { key: 'position_pnl_percent', label: '持仓盈亏比' }
];

const TOOLTIP_FIELD_OPTIONS: Array<{ key: FieldKey; label: string }> = [
  { key: 'name', label: '名称' },
  { key: 'code', label: '代码' },
  ...QUOTE_FIELD_OPTIONS
];

const DEFAULT_POPUP_FIELDS: FieldKey[] = ['price', 'change_percent', 'daily_pnl', 'daily_pnl_percent'];
const DEFAULT_TOOLTIP_FIELDS: FieldKey[] = ['price', 'change_percent', 'daily_pnl', 'position_pnl'];

function formatSigned(value: number, digits = 2, suffix = '') {
  const sign = value > 0 ? '+' : '';
  return `${sign}${value.toFixed(digits)}${suffix}`;
}

function formatPrice(value: number) {
  return value ? value.toFixed(3) : '-';
}

function toneClass(value: number) {
  if (value > 0.0001) return 'up';
  if (value < -0.0001) return 'down';
  return 'flat';
}

function selectedFields(config: AppConfig | undefined, source: 'display_fields' | 'tooltip_fields', fallback: FieldKey[], options: Array<{ key: FieldKey }>) {
  const allowed = new Set(options.map((option) => option.key));
  const fields = (config?.[source] ?? fallback).filter((field): field is FieldKey => allowed.has(field as FieldKey));
  return fields.length ? fields : fallback;
}

function useThemeMode(themeMode: string | undefined) {
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');

    function applyTheme() {
      const resolved = themeMode === 'system'
        ? (media.matches ? 'dark' : 'light')
        : themeMode === 'light' ? 'light' : 'dark';
      document.documentElement.dataset.theme = resolved;
      document.documentElement.style.colorScheme = resolved;
    }

    applyTheme();
    media.addEventListener('change', applyTheme);
    return () => media.removeEventListener('change', applyTheme);
  }, [themeMode]);
}

function PopupApp() {
  const [state, setState] = useState<AppStatePayload | null>(null);
  useThemeMode(state?.config.appearance.theme_mode);

  useEffect(() => {
    document.body.dataset.view = 'popup';
    return () => {
      delete document.body.dataset.view;
    };
  }, []);

  useEffect(() => {
    getState().then(setState).catch(console.error);
    const unlisten = onState(setState);
    return () => {
      unlisten.then((fn) => fn()).catch(console.error);
    };
  }, []);

  const rows = state?.summary?.items.filter((item) => item.show_in_popup) ?? [];
  const summary = state?.summary;
  const config = state?.config;
  const fields = selectedFields(config, 'display_fields', DEFAULT_POPUP_FIELDS, QUOTE_FIELD_OPTIONS);
  const opacity = config?.appearance.popup_tint_opacity ?? 0.38;
  const softAlpha = Math.max(0.015, Math.min(0.16, opacity * 0.18));
  const radius = config?.appearance.corner_radius ?? 14;
  const upColor = config?.popup.up_color ?? '#C73E4E';
  const downColor = config?.popup.down_color ?? '#5B8C5A';
  const flatColor = config?.popup.flat_color ?? '#999999';
  const popupDensity = fields.length <= 3 ? 'compact' : fields.length <= 6 ? 'balanced' : 'detail';
  const metricBasis = fields.length <= 1 ? '100%' : fields.length === 2 ? 'calc((100% - 6px) / 2)' : 'calc((100% - 12px) / 3)';

  return (
    <main
      className={`popup-shell popup-${popupDensity}`}
      style={{
        '--panel-alpha': opacity,
        '--soft-alpha': softAlpha,
        '--corner-radius': `${radius}px`,
        '--up-color': upColor,
        '--down-color': downColor,
        '--flat-color': flatColor,
        '--metric-basis': metricBasis
      } as React.CSSProperties}
      onDoubleClick={() => hidePopup().catch(console.error)}
      onMouseEnter={() => setPopupHovered(true).catch(console.error)}
      onMouseLeave={() => setPopupHovered(false).catch(console.error)}
    >
      <section className="popup-panel">
        {summary && summary.total_prev_value > 0 && config?.show_daily_summary && (
          <div className={`popup-summary ${toneClass(summary.total_daily_pnl)}`}>
            <div>
              <span>今日合计</span>
              <strong>{formatSigned(summary.total_daily_pnl, 2)}</strong>
            </div>
            <b>{formatSigned(summary.total_daily_pnl_percent, 2, '%')}</b>
          </div>
        )}
        <div className="quote-list">
          {rows.length === 0 ? (
            <div className="empty">正在刷新行情...</div>
          ) : (
            rows.map((item) => <QuoteRow fields={fields} item={item} key={item.code} />)
          )}
        </div>
      </section>
    </main>
  );
}

function QuoteRow({ fields, item }: { fields: FieldKey[]; item: DailyPnlItem }) {
  const cls = toneClass(item.daily_pnl || item.change_percent);
  return (
    <div className="quote-row">
      <div className="quote-main">
        <span className="name" title={item.name || item.code}>{item.name || item.code}</span>
        <span className="code">{item.code.toUpperCase()}</span>
      </div>
      <div className="quote-metrics">
        {fields.map((field) => (
          <MetricChip field={field} item={item} key={field} tone={cls} />
        ))}
      </div>
    </div>
  );
}

function MetricChip({ field, item, tone }: { field: FieldKey; item: DailyPnlItem; tone: string }) {
  const label = TOOLTIP_FIELD_OPTIONS.find((option) => option.key === field)?.label ?? field;
  const isMarketTone = field === 'change' || field === 'change_percent' || field === 'daily_pnl' || field === 'daily_pnl_percent' || field === 'position_pnl' || field === 'position_pnl_percent';
  return (
    <span className="metric-chip">
      <small>{label}</small>
      <strong className={isMarketTone ? metricTone(field, item, tone) : ''}>{formatMetric(field, item)}</strong>
    </span>
  );
}

function metricTone(field: FieldKey, item: DailyPnlItem, fallback: string) {
  if (field === 'change') return toneClass(item.change);
  if (field === 'change_percent') return toneClass(item.change_percent);
  if (field === 'daily_pnl') return toneClass(item.daily_pnl);
  if (field === 'daily_pnl_percent') return toneClass(item.daily_pnl_percent);
  if (field === 'position_pnl') return toneClass(item.position_pnl);
  if (field === 'position_pnl_percent') return toneClass(item.position_pnl_percent);
  return fallback;
}

function formatMetric(field: FieldKey, item: DailyPnlItem) {
  switch (field) {
    case 'name':
      return item.name || '-';
    case 'code':
      return item.code.toUpperCase();
    case 'price':
      return formatPrice(item.price);
    case 'prev_close':
      return formatPrice(item.prev_close);
    case 'open':
      return formatPrice(item.open);
    case 'high':
      return formatPrice(item.high);
    case 'low':
      return formatPrice(item.low);
    case 'change':
      return formatSigned(item.change, 3);
    case 'change_percent':
      return formatSigned(item.change_percent, 2, '%');
    case 'volume':
      return item.volume ? item.volume.toLocaleString('zh-CN', { maximumFractionDigits: 0 }) : '-';
    case 'amount':
      return item.amount ? `${item.amount.toLocaleString('zh-CN', { maximumFractionDigits: 0 })} 万` : '-';
    case 'volume_ratio':
      return item.volume_ratio ? item.volume_ratio.toFixed(2) : '-';
    case 'turnover':
      return item.turnover ? `${item.turnover.toFixed(2)}%` : '-';
    case 'holdings':
      return item.holdings.toLocaleString('zh-CN', { maximumFractionDigits: 0 });
    case 'cost_price':
      return item.cost_price.toFixed(3);
    case 'daily_pnl':
      return formatSigned(item.daily_pnl, 0);
    case 'daily_pnl_percent':
      return formatSigned(item.daily_pnl_percent, 2, '%');
    case 'position_pnl':
      return formatSigned(item.position_pnl, 0);
    case 'position_pnl_percent':
      return formatSigned(item.position_pnl_percent, 2, '%');
  }
}

function SettingsApp() {
  const [state, setState] = useState<AppStatePayload | null>(null);
  const [draft, setDraft] = useState<AppConfig | null>(null);
  const [sortState, setSortState] = useState<{ field: 'holdings' | 'change_percent'; direction: 'asc' | 'desc' } | null>(null);
  const [draggingCode, setDraggingCode] = useState<string | null>(null);
  const [dragOverCode, setDragOverCode] = useState<string | null>(null);
  const draggingCodeRef = useRef<string | null>(null);
  const dragOverCodeRef = useRef<string | null>(null);
  const [code, setCode] = useState('');
  const [holdings, setHoldings] = useState('0');
  const [costPrice, setCostPrice] = useState('');
  const [message, setMessage] = useState('');
  const [updating, setUpdating] = useState(false);
  useThemeMode(draft?.appearance.theme_mode);

  useEffect(() => {
    document.body.dataset.view = 'settings';
    return () => {
      delete document.body.dataset.view;
    };
  }, []);

  useEffect(() => {
    getState().then((payload) => {
      setState(payload);
      setDraft(cloneConfig(payload.config));
    }).catch(console.error);
    const unlisten = onState((payload) => {
      setState(payload);
      setDraft((current) => current ?? cloneConfig(payload.config));
    });
    return () => {
      unlisten.then((fn) => fn()).catch(console.error);
    };
  }, []);

  useEffect(() => {
    draggingCodeRef.current = draggingCode;
  }, [draggingCode]);

  useEffect(() => {
    dragOverCodeRef.current = dragOverCode;
  }, [dragOverCode]);

  useEffect(() => {
    if (!draggingCode) return;

    function stockCodeFromPoint(x: number, y: number) {
      const element = document.elementFromPoint(x, y);
      return element?.closest<HTMLElement>('[data-stock-code]')?.dataset.stockCode ?? null;
    }

    function handlePointerMove(event: PointerEvent) {
      const code = stockCodeFromPoint(event.clientX, event.clientY);
      if (code && code !== dragOverCodeRef.current) {
        setDragOverCode(code);
      }
    }

    function handlePointerUp(event: PointerEvent) {
      const dragCode = draggingCodeRef.current;
      const targetCode = stockCodeFromPoint(event.clientX, event.clientY) ?? dragOverCodeRef.current;
      if (dragCode && targetCode) {
        reorderStock(dragCode, targetCode);
      }
      finishDrag();
    }

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp, { once: true });
    window.addEventListener('pointercancel', finishDrag, { once: true });
    return () => {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', finishDrag);
    };
  }, [draggingCode]);

  const summary = state?.summary;
  const stocks = draft?.stocks ?? [];
  const quoteByCode = useMemo(() => {
    return new Map((summary?.items ?? []).map((item) => [item.code, item]));
  }, [summary]);

  async function handleAdd() {
    setMessage('');
    try {
      const config = await addStock(code, Number(holdings) || 0, costPrice.trim() === '' ? undefined : normalizeCostInput(costPrice));
      setDraft(cloneConfig(config));
      setCode('');
      setHoldings('0');
      setCostPrice('');
      setMessage('已添加');
    } catch (err) {
      setMessage(String(err));
    }
  }

  async function handleSave() {
    if (!draft) return;
    setMessage('');
    try {
      const payload = await saveSettings(draft);
      setState(payload);
      setDraft(cloneConfig(payload.config));
      setMessage('已保存');
    } catch (err) {
      setMessage(`保存失败：${String(err)}`);
    }
  }

  async function handleRefresh() {
    setMessage('');
    try {
      await refreshQuotes();
      const payload = await getState();
      setState(payload);
      setMessage('已刷新');
    } catch (err) {
      setMessage(`刷新失败：${String(err)}`);
    }
  }

  async function handleCheckUpdate() {
    if (updating) return;
    setUpdating(true);
    setMessage('正在检查更新...');
    try {
      const result = await checkAndInstallUpdate();
      if (result.available) {
        setMessage(`已安装新版本 ${result.version ?? ''}，正在重启...`);
      } else {
        setMessage(`当前已是最新版本 v${result.current_version}`);
      }
    } catch (err) {
      setMessage(`检查更新失败：${String(err)}`);
    } finally {
      setUpdating(false);
    }
  }

  function updateStock(code: string, patch: Partial<StockEntry>) {
    setSortState(null);
    setDraft((current) => current && {
      ...current,
      stocks: current.stocks.map((stock) => stock.code === code ? { ...stock, ...patch } : stock)
    });
  }

  function selectTooltipStock(code: string) {
    setSortState(null);
    setDraft((current) => current && {
      ...current,
      stocks: current.stocks.map((stock) => ({
        ...stock,
        show_in_tooltip: stock.code === code
      }))
    });
  }

  function removeStock(code: string) {
    setSortState(null);
    setDraft((current) => {
      if (!current) return current;
      const nextStocks = current.stocks.filter((stock) => stock.code !== code);
      if (nextStocks.length > 0 && !nextStocks.some((stock) => stock.show_in_tooltip)) {
        nextStocks[0] = { ...nextStocks[0], show_in_tooltip: true };
      }
      return {
        ...current,
        stocks: nextStocks
      };
    });
  }

  function moveStock(code: string, direction: -1 | 1) {
    setSortState(null);
    setDraft((current) => {
      if (!current) return current;
      const index = current.stocks.findIndex((stock) => stock.code === code);
      const targetIndex = index + direction;
      if (index < 0 || targetIndex < 0 || targetIndex >= current.stocks.length) return current;
      const nextStocks = [...current.stocks];
      [nextStocks[index], nextStocks[targetIndex]] = [nextStocks[targetIndex], nextStocks[index]];
      return {
        ...current,
        stocks: nextStocks
      };
    });
  }

  function reorderStock(dragCode: string, targetCode: string) {
    if (dragCode === targetCode) return;
    setSortState(null);
    setDraft((current) => {
      if (!current) return current;
      const fromIndex = current.stocks.findIndex((stock) => stock.code === dragCode);
      const toIndex = current.stocks.findIndex((stock) => stock.code === targetCode);
      if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return current;
      const nextStocks = [...current.stocks];
      const [moved] = nextStocks.splice(fromIndex, 1);
      nextStocks.splice(toIndex, 0, moved);
      return {
        ...current,
        stocks: nextStocks
      };
    });
  }

  function finishDrag() {
    setDraggingCode(null);
    setDragOverCode(null);
  }

  function sortStocks(field: 'holdings' | 'change_percent') {
    const direction = sortState?.field === field && sortState.direction === 'desc' ? 'asc' : 'desc';
    setSortState({ field, direction });
    setDraft((current) => current && {
      ...current,
      stocks: sortStockEntries(current.stocks, quoteByCode, field, direction)
    });
  }

  if (!draft) return <main className="settings-shell">加载中...</main>;

  return (
    <main
      className="settings-shell"
      style={{
        '--up-color': draft.popup.up_color || '#C73E4E',
        '--down-color': draft.popup.down_color || '#5B8C5A',
        '--flat-color': draft.popup.flat_color || '#999999'
      } as React.CSSProperties}
    >
      <header className="settings-title">
        <div>
          <div className="title-line">
            <h1>韭菜托盘设置</h1>
            <span className="version-badge">v{state?.app_version ?? '-'}</span>
          </div>
          <p>行情、弹窗、托盘提示和外观偏好</p>
        </div>
        <div className="title-actions">
          <button disabled={updating} onClick={handleCheckUpdate}>
            {updating ? '检查中...' : '检查更新'}
          </button>
          <button onClick={handleRefresh}>立即刷新</button>
          <button className="primary" onClick={handleSave}>保存并应用</button>
        </div>
      </header>

      <section className="dashboard">
        <div className="stat">
          <span>今日盈亏</span>
          <strong className={toneClass(summary?.total_daily_pnl ?? 0)}>
            {summary ? formatSigned(summary.total_daily_pnl, 2) : '-'}
          </strong>
        </div>
        <div className="stat">
          <span>盈亏比</span>
          <strong className={toneClass(summary?.total_daily_pnl ?? 0)}>
            {summary ? formatSigned(summary.total_daily_pnl_percent, 2, '%') : '-'}
          </strong>
        </div>
        <div className="stat">
          <span>自选股</span>
          <strong>{stocks.length} 只</strong>
        </div>
        <div className="stat" title={state?.last_error ?? ''}>
          <span>刷新状态</span>
          <strong className={state?.last_error ? 'down' : 'flat'}>
            {state?.last_error ? '失败' : (state?.last_refreshed_at ?? '-')}
          </strong>
          {state?.last_error && <small>{state.last_error}</small>}
        </div>
      </section>

      <section className="settings-section stock-manager">
        <div className="section-heading">
          <div>
            <h2>自选股管理</h2>
              <p>名称随行情自动更新，持仓可为 0，正数按 100 股取整</p>
          </div>
          <div className="stock-sort-actions">
            <button type="button" onClick={() => sortStocks('holdings')}>
              持仓{sortState?.field === 'holdings' ? (sortState.direction === 'desc' ? '↓' : '↑') : ''}
            </button>
            <button type="button" onClick={() => sortStocks('change_percent')}>
              涨跌幅{sortState?.field === 'change_percent' ? (sortState.direction === 'desc' ? '↓' : '↑') : ''}
            </button>
            <span>{stocks.filter((stock) => stock.show_in_popup).length} 只显示在弹窗</span>
          </div>
        </div>
        <div className="add-row">
          <input value={code} onChange={(e) => setCode(e.target.value)} placeholder="代码，如 600519" />
          <input value={holdings} onChange={(e) => setHoldings(e.target.value)} placeholder="持仓" type="number" min="0" step="100" />
          <input value={costPrice} onChange={(e) => setCostPrice(e.target.value)} placeholder="成本，留空取实时价" type="number" step="0.001" />
          <button onClick={handleAdd}>添加</button>
        </div>
        <div className="stock-table-wrap">
          <div className="stock-table" role="table">
            <div className="stock-table-row stock-table-head" role="row">
              <span>股票</span>
              <span>实时</span>
              <span>持仓</span>
              <span>成本</span>
              <span>显示</span>
              <span>操作</span>
            </div>
            {stocks.map((stock) => (
              <StockTableRow
                key={stock.code}
                quote={quoteByCode.get(stock.code)}
                canMoveDown={stocks[stocks.length - 1]?.code !== stock.code}
                canMoveUp={stocks[0]?.code !== stock.code}
                isDragging={draggingCode === stock.code}
                isDragOver={dragOverCode === stock.code && draggingCode !== stock.code}
                stock={stock}
                onChange={(patch) => updateStock(stock.code, patch)}
                onDelete={() => removeStock(stock.code)}
                onDragEnd={finishDrag}
                onDragStart={(event) => {
                  event.preventDefault();
                  event.currentTarget.setPointerCapture?.(event.pointerId);
                  setDraggingCode(stock.code);
                  setDragOverCode(null);
                }}
                onMoveDown={() => moveStock(stock.code, 1)}
                onMoveUp={() => moveStock(stock.code, -1)}
                onSelectTooltip={() => selectTooltipStock(stock.code)}
              />
            ))}
          </div>
        </div>
      </section>

      <div className="settings-grid">
        <section className="settings-section">
          <div className="section-heading">
            <div>
              <h2>弹窗显示</h2>
              <p>{draft.display_fields.length} 个指标</p>
            </div>
          </div>
          <label>
            <span>后台自动刷新</span>
            <div className="inline-control">
              <input
                type="number"
                min="0"
                max="600"
                step="1"
                value={Math.round((draft.background_refresh_interval_ms ?? 10000) / 1000)}
                onChange={(e) => setDraft({ ...draft, background_refresh_interval_ms: Math.round(Math.max(0, Number(e.target.value) || 0) * 1000) })}
              />
              <span>秒</span>
            </div>
          </label>
          <label>
            <span>自动消失时长</span>
            <div className="inline-control">
              <input
                type="number"
                min="0"
                max="600"
                step="0.1"
                value={Number(((draft.popup.auto_hide_ms ?? 0) / 1000).toFixed(1))}
                onChange={(e) => setDraft(updatePopup(draft, { auto_hide_ms: Math.round(Math.max(0, Number(e.target.value) || 0) * 1000) }))}
              />
              <span>秒</span>
            </div>
          </label>
          <FieldChecklist
            fields={draft.display_fields}
            options={QUOTE_FIELD_OPTIONS}
            onToggle={(field, checked) => setDraft(toggleField(draft, 'display_fields', field, checked, ['price']))}
          />
          <label className="check">
            <input type="checkbox" checked={draft.show_daily_summary} onChange={(e) => setDraft({ ...draft, show_daily_summary: e.target.checked })} />
            <span>显示当日合计</span>
          </label>
        </section>

        <section className="settings-section">
          <div className="section-heading">
            <div>
              <h2>托盘提示</h2>
              <p>{draft.tooltip_fields.length} 个指标</p>
            </div>
          </div>
          <FieldChecklist
            fields={draft.tooltip_fields}
            options={TOOLTIP_FIELD_OPTIONS}
            onToggle={(field, checked) => setDraft(toggleField(draft, 'tooltip_fields', field, checked, DEFAULT_TOOLTIP_FIELDS))}
          />
        </section>

        <section className="settings-section">
          <div className="section-heading">
            <div>
              <h2>外观</h2>
              <p>主题和涨跌颜色</p>
            </div>
          </div>
          <div className="setting-block">
            <span>主题</span>
            <div className="segmented">
              {[
                ['system', '跟随系统'],
                ['dark', '深色'],
                ['light', '浅色']
              ].map(([value, label]) => (
                <button
                  className={draft.appearance.theme_mode === value ? 'active' : ''}
                  key={value}
                  onClick={() => setDraft(updateAppearance(draft, { theme_mode: value, theme: value === 'light' ? 'light' : 'dark' }))}
                  type="button"
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
          <label>
            <span>弹窗不透明度 {Math.round(draft.appearance.popup_tint_opacity * 100)}%</span>
            <input
              type="range"
              min="0"
              max="0.95"
              step="0.01"
              value={draft.appearance.popup_tint_opacity}
              onChange={(e) => setDraft(updateAppearance(draft, { popup_tint_opacity: Number(e.target.value) }))}
            />
          </label>
          <div className="color-grid">
            <label>
              <span>上涨</span>
              <input type="color" value={draft.popup.up_color} onChange={(e) => setDraft(updatePopup(draft, { up_color: e.target.value }))} />
            </label>
            <label>
              <span>下跌</span>
              <input type="color" value={draft.popup.down_color} onChange={(e) => setDraft(updatePopup(draft, { down_color: e.target.value }))} />
            </label>
            <label>
              <span>持平</span>
              <input type="color" value={draft.popup.flat_color} onChange={(e) => setDraft(updatePopup(draft, { flat_color: e.target.value }))} />
            </label>
          </div>
          <label>
            <span>圆角 {draft.appearance.corner_radius}px</span>
            <input type="range" min="0" max="24" step="1" value={draft.appearance.corner_radius} onChange={(e) => setDraft(updateAppearance(draft, { corner_radius: Number(e.target.value) }))} />
          </label>
          <label className="check">
            <input type="checkbox" checked={draft.appearance.animations_enabled} onChange={(e) => setDraft(updateAppearance(draft, { animations_enabled: e.target.checked }))} />
            <span>启用基础动效</span>
          </label>
        </section>
      </div>
      {message && <div className="message">{message}</div>}
    </main>
  );
}

function FieldChecklist({ fields, options, onToggle }: { fields: string[]; options: Array<{ key: FieldKey; label: string }>; onToggle: (field: FieldKey, checked: boolean) => void }) {
  return (
    <div className="field-list">
      {options.map((field) => (
        <label className="check" key={field.key}>
          <input
            type="checkbox"
            checked={fields.includes(field.key)}
            onChange={(e) => onToggle(field.key, e.target.checked)}
          />
          <span>{field.label}</span>
        </label>
      ))}
    </div>
  );
}

function StockTableRow({
  canMoveDown,
  canMoveUp,
  isDragging,
  isDragOver,
  quote,
  stock,
  onChange,
  onDelete,
  onDragEnd,
  onDragStart,
  onMoveDown,
  onMoveUp,
  onSelectTooltip
}: {
  canMoveDown: boolean;
  canMoveUp: boolean;
  isDragging: boolean;
  isDragOver: boolean;
  quote?: DailyPnlItem;
  stock: StockEntry;
  onChange: (patch: Partial<StockEntry>) => void;
  onDelete: () => void;
  onDragEnd: () => void;
  onDragStart: (event: React.PointerEvent<HTMLButtonElement>) => void;
  onMoveDown: () => void;
  onMoveUp: () => void;
  onSelectTooltip: () => void;
}) {
  const quoteTone = toneClass(quote?.change_percent ?? 0);
  return (
    <div
      className={`stock-table-row stock-table-body${isDragging ? ' dragging' : ''}${isDragOver ? ' drag-over' : ''}`}
      data-stock-code={stock.code}
      role="row"
    >
      <div className="stock-id">
        <button
          className="drag-handle"
          onPointerCancel={onDragEnd}
          onPointerDown={onDragStart}
          title="拖动排序"
          type="button"
        >
          ↕
        </button>
        <strong>{stock.code.toUpperCase()}</strong>
        <span className="stock-name">{stock.name || '-'}</span>
      </div>
      <div className="stock-quote">
        <span>
          <small>最新价</small>
          <strong className={quoteTone}>{quote ? formatPrice(quote.price) : '-'}</strong>
        </span>
        <span>
          <small>涨跌幅</small>
          <strong className={quoteTone}>{quote ? formatSigned(quote.change_percent, 2, '%') : '-'}</strong>
        </span>
      </div>
      <label className="stock-input">
        <span>持仓</span>
        <input
          type="number"
          min="0"
          step="100"
          value={stock.holdings}
          onChange={(e) => onChange({ holdings: normalizeHoldingInput(e.target.value) })}
        />
      </label>
      <label className="stock-input">
        <span>成本</span>
        <input
          type="number"
          step="0.001"
          value={stock.cost_price}
          onChange={(e) => onChange({ cost_price: normalizeCostInput(e.target.value) })}
        />
      </label>
      <div className="stock-actions">
        <label><input type="checkbox" checked={stock.show_in_popup} onChange={(e) => onChange({ show_in_popup: e.target.checked })} />弹窗</label>
        <label><input type="radio" name="tooltip-stock" checked={stock.show_in_tooltip} onChange={onSelectTooltip} />提示</label>
      </div>
      <div className="row-actions">
        <button className="ghost icon-button" disabled={!canMoveUp} onClick={onMoveUp} title="上移" type="button">↑</button>
        <button className="ghost icon-button" disabled={!canMoveDown} onClick={onMoveDown} title="下移" type="button">↓</button>
        <button className="ghost danger" onClick={onDelete} type="button">删除</button>
      </div>
    </div>
  );
}

function sortStockEntries(
  stocks: StockEntry[],
  quoteByCode: Map<string, DailyPnlItem>,
  field: 'holdings' | 'change_percent',
  direction: 'asc' | 'desc'
) {
  return stocks
    .map((stock, index) => ({ stock, index, value: stockSortValue(stock, quoteByCode, field) }))
    .sort((left, right) => {
      const leftMissing = left.value === null;
      const rightMissing = right.value === null;
      if (leftMissing && rightMissing) return left.index - right.index;
      if (leftMissing) return 1;
      if (rightMissing) return -1;
      const leftValue = left.value ?? 0;
      const rightValue = right.value ?? 0;
      const delta = leftValue - rightValue;
      if (Math.abs(delta) < Number.EPSILON) return left.index - right.index;
      return direction === 'asc' ? delta : -delta;
    })
    .map((entry) => entry.stock);
}

function stockSortValue(stock: StockEntry, quoteByCode: Map<string, DailyPnlItem>, field: 'holdings' | 'change_percent') {
  if (field === 'holdings') return stock.holdings;
  const value = quoteByCode.get(stock.code)?.change_percent;
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function normalizeHoldingInput(value: string) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return Math.max(0, Math.round(parsed / 100) * 100);
}

function normalizeCostInput(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? Math.round(parsed * 1000) / 1000 : 0;
}

function updateAppearance(config: AppConfig, patch: Partial<AppConfig['appearance']> & { theme?: string }): AppConfig {
  const { theme, ...appearancePatch } = patch;
  return {
    ...config,
    theme: theme ?? config.theme,
    appearance: {
      ...config.appearance,
      ...appearancePatch
    }
  };
}

function updatePopup(config: AppConfig, patch: Partial<AppConfig['popup']>): AppConfig {
  return {
    ...config,
    popup: {
      ...config.popup,
      ...patch
    }
  };
}

function toggleField(config: AppConfig, key: 'display_fields' | 'tooltip_fields', field: FieldKey, checked: boolean, fallback: FieldKey[]): AppConfig {
  const next = checked
    ? Array.from(new Set([...config[key], field]))
    : config[key].filter((current) => current !== field);
  return {
    ...config,
    [key]: next.length ? next : fallback
  };
}

function cloneConfig(config: AppConfig): AppConfig {
  return JSON.parse(JSON.stringify(config)) as AppConfig;
}

function AppRouter() {
  const page = useMemo(() => window.location.pathname.toLowerCase(), []);
  const route = useMemo(() => {
    const hashRoute = window.location.hash.replace(/^#\/?/, '').toLowerCase();
    if (hashRoute) return hashRoute;
    const queryRoute = new URLSearchParams(window.location.search).get('view')?.toLowerCase();
    if (queryRoute) return queryRoute;
    if (page.endsWith('/popup.html')) return 'popup';
    return 'settings';
  }, [page]);
  if (route === 'popup') return <PopupApp />;
  return <SettingsApp />;
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <AppRouter />
  </React.StrictMode>
);

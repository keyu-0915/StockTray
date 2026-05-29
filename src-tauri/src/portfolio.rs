use crate::models::{AppConfig, DailyPnlItem, DailySummary, StockData};

pub(crate) fn compute_daily_pnl(quotes: &[StockData], config: &AppConfig) -> DailySummary {
    let mut summary = DailySummary::default();
    for stock in &config.stocks {
        if let Some(quote) = quotes
            .iter()
            .find(|q| q.code == stock.code && q.error.is_empty())
        {
            let daily_pnl = round2(stock.holdings * quote.change);
            let prev_value = round2(stock.holdings * quote.prev_close);
            let daily_pnl_percent = if prev_value > 0.0 {
                round2(daily_pnl / prev_value * 100.0)
            } else {
                0.0
            };
            let position_pnl = round2(stock.holdings * (quote.price - stock.cost_price));
            let cost_value_abs = (stock.holdings * stock.cost_price).abs();
            let position_pnl_percent = if cost_value_abs > 0.0001 {
                round2(position_pnl / cost_value_abs * 100.0)
            } else {
                0.0
            };
            summary.total_prev_value += prev_value;
            summary.total_daily_pnl += daily_pnl;
            summary.items.push(DailyPnlItem {
                code: quote.code.clone(),
                name: quote.name.clone(),
                price: quote.price,
                prev_close: quote.prev_close,
                open: quote.open,
                high: quote.high,
                low: quote.low,
                volume: quote.volume,
                amount: quote.amount,
                change: quote.change,
                change_percent: quote.change_percent,
                turnover: quote.turnover,
                date: quote.date.clone(),
                time: quote.time.clone(),
                holdings: stock.holdings,
                cost_price: stock.cost_price,
                daily_pnl,
                daily_pnl_percent,
                position_pnl,
                position_pnl_percent,
                show_in_popup: stock.show_in_popup,
                show_in_tooltip: stock.show_in_tooltip,
                error: quote.error.clone(),
            });
        } else {
            summary.items.push(DailyPnlItem {
                code: stock.code.clone(),
                name: stock.name.clone(),
                holdings: stock.holdings,
                cost_price: stock.cost_price,
                show_in_popup: stock.show_in_popup,
                show_in_tooltip: stock.show_in_tooltip,
                error: "no_data".into(),
                ..Default::default()
            });
        }
    }
    summary.total_daily_pnl = round2(summary.total_daily_pnl);
    summary.total_daily_pnl_percent = if summary.total_prev_value > 0.0 {
        round2(summary.total_daily_pnl / summary.total_prev_value * 100.0)
    } else {
        0.0
    };
    summary
}

fn round2(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

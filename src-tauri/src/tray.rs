use std::time::{Duration, Instant};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::{
    config::is_supported_field,
    models::{AppConfig, DailyPnlItem, DailySummary},
    refresh_quotes_inner,
    state::{current_payload, SharedState},
    windowing::{show_window, toggle_popup},
};

const MAIN_TRAY_ID: &str = "stocktray-main";

pub(crate) fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示行情", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &refresh, &settings, &quit])?;

    TrayIconBuilder::with_id(MAIN_TRAY_ID)
        .icon(tray_status_icon(0.0, 0.0))
        .tooltip("韭菜托盘")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => toggle_popup(app, None),
            "refresh" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = refresh_quotes_inner(&app).await;
                });
            }
            "settings" => show_window(app, "settings"),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Enter { .. } | TrayIconEvent::Move { .. } => {
                set_tray_hovered(tray.app_handle(), true);
            }
            TrayIconEvent::Leave { .. } => {
                set_tray_hovered(tray.app_handle(), false);
            }
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Down | MouseButtonState::Up,
                position,
                ..
            } => {
                let app = tray.app_handle();
                if accept_tray_click(app) {
                    toggle_popup(app, Some((position.x as i32, position.y as i32)));
                }
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn set_tray_hovered(app: &AppHandle, hovered: bool) {
    let state = app.state::<SharedState>();
    let result = state.0.lock();
    if let Ok(mut guard) = result {
        guard.tray_hovered = hovered;
    }
}

fn accept_tray_click(app: &AppHandle) -> bool {
    let state = app.state::<SharedState>();
    let result = state.0.lock();
    if let Ok(mut guard) = result {
        let now = Instant::now();
        let accept = guard
            .last_tray_click_at
            .map(|last| now.duration_since(last) > Duration::from_millis(350))
            .unwrap_or(true);
        if accept {
            guard.last_tray_click_at = Some(now);
        }
        return accept;
    }
    false
}

pub(crate) fn update_tray_status(app: &AppHandle, summary: &DailySummary) {
    if let Some(tray) = app.tray_by_id(MAIN_TRAY_ID) {
        let _ = tray.set_icon(Some(tray_status_icon(
            summary.total_daily_pnl,
            summary.total_daily_pnl_percent,
        )));
        let trend = if summary.total_daily_pnl > 0.0001 {
            "盈利"
        } else if summary.total_daily_pnl < -0.0001 {
            "亏损"
        } else {
            "持平"
        };
        let tooltip = current_payload(app)
            .map(|payload| build_native_tray_tooltip(&payload.config, summary, trend))
            .unwrap_or_else(|| {
                format!(
                    "韭菜托盘 今日{} {} ({:+.2}%)",
                    trend, summary.total_daily_pnl, summary.total_daily_pnl_percent
                )
            });
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn build_native_tray_tooltip(config: &AppConfig, summary: &DailySummary, trend: &str) -> String {
    let mut fields = config
        .tooltip_fields
        .iter()
        .filter(|field| is_supported_field(field.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if fields.is_empty() {
        fields = vec!["price", "change_percent", "daily_pnl"];
    }

    let mut lines = vec![format!(
        "韭菜托盘 今日{} {} ({:+.2}%)",
        trend, summary.total_daily_pnl, summary.total_daily_pnl_percent
    )];

    let selected_code = config
        .stocks
        .iter()
        .find(|stock| stock.show_in_tooltip)
        .map(|stock| stock.code.as_str());

    for item in summary.items.iter().filter(|item| match selected_code {
        Some(code) => item.code == code,
        None => item.show_in_tooltip,
    }) {
        let mut parts = Vec::with_capacity(fields.len() + 1);
        parts.push(if item.name.is_empty() {
            item.code.to_uppercase()
        } else {
            item.name.clone()
        });
        for field in &fields {
            parts.push(format!(
                "{} {}",
                tray_tooltip_field_label(field),
                tray_tooltip_field_value(field, item)
            ));
        }
        lines.push(parts.join(" "));
    }

    if lines.len() == 1 {
        lines.push("暂无已选择提示的自选股".to_string());
    }

    lines.join("\n")
}

fn tray_tooltip_field_label(field: &str) -> &'static str {
    match field {
        "name" => "名称",
        "code" => "代码",
        "price" => "最新价",
        "prev_close" => "昨收",
        "open" => "今开",
        "high" => "最高",
        "low" => "最低",
        "change" => "涨跌额",
        "change_percent" => "涨跌幅",
        "volume" => "成交量",
        "amount" => "成交额",
        "volume_ratio" => "量比",
        "turnover" => "换手率",
        "holdings" => "持仓",
        "cost_price" => "成本",
        "daily_pnl" => "当日盈亏",
        "daily_pnl_percent" => "当日盈亏比",
        "position_pnl" => "持仓盈亏",
        "position_pnl_percent" => "持仓盈亏比",
        _ => "指标",
    }
}

fn tray_tooltip_field_value(field: &str, item: &DailyPnlItem) -> String {
    match field {
        "name" => non_empty_or_dash(&item.name),
        "code" => item.code.to_uppercase(),
        "price" => price_or_dash(item.price),
        "prev_close" => price_or_dash(item.prev_close),
        "open" => price_or_dash(item.open),
        "high" => price_or_dash(item.high),
        "low" => price_or_dash(item.low),
        "change" => format_signed(item.change, 3, ""),
        "change_percent" => format_signed(item.change_percent, 2, "%"),
        "volume" => integer_or_dash(item.volume, ""),
        "amount" => integer_or_dash(item.amount, "万"),
        "volume_ratio" => decimal_or_dash(item.volume_ratio),
        "turnover" => {
            if item.turnover.abs() > f32::EPSILON {
                format!("{:.2}%", item.turnover)
            } else {
                "-".to_string()
            }
        }
        "holdings" => format!("{:.0}", item.holdings),
        "cost_price" => format!("{:.3}", item.cost_price),
        "daily_pnl" => format_signed(item.daily_pnl, 0, ""),
        "daily_pnl_percent" => format_signed(item.daily_pnl_percent, 2, "%"),
        "position_pnl" => format_signed(item.position_pnl, 0, ""),
        "position_pnl_percent" => format_signed(item.position_pnl_percent, 2, "%"),
        _ => "-".to_string(),
    }
}

fn non_empty_or_dash(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn price_or_dash(value: f32) -> String {
    if value.abs() > f32::EPSILON {
        format!("{value:.3}")
    } else {
        "-".to_string()
    }
}

fn integer_or_dash(value: f32, suffix: &str) -> String {
    if value.abs() > f32::EPSILON {
        format!("{value:.0}{suffix}")
    } else {
        "-".to_string()
    }
}

fn decimal_or_dash(value: f32) -> String {
    if value.abs() > f32::EPSILON {
        format!("{value:.2}")
    } else {
        "-".to_string()
    }
}

fn format_signed(value: f32, digits: usize, suffix: &str) -> String {
    let sign = if value > 0.0 { "+" } else { "" };
    format!("{sign}{value:.digits$}{suffix}")
}

fn tray_status_icon(value: f32, percent: f32) -> Image<'static> {
    if value > 0.0001 {
        build_tray_icon(TrendDirection::Up, trend_color(percent))
    } else if value < -0.0001 {
        build_tray_icon(TrendDirection::Down, trend_color(percent))
    } else {
        build_tray_icon(TrendDirection::Flat, trend_color(0.0))
    }
}

enum TrendDirection {
    Up,
    Down,
    Flat,
}

fn trend_color(percent: f32) -> (u8, u8, u8) {
    const STOPS: [(f32, (u8, u8, u8)); 5] = [
        (-15.0, (0, 48, 30)),
        (-5.0, (34, 184, 83)),
        (0.0, (150, 150, 150)),
        (5.0, (232, 64, 78)),
        (15.0, (168, 28, 104)),
    ];

    let clamped = percent.clamp(-15.0, 15.0);
    for pair in STOPS.windows(2) {
        let (left_percent, left_color) = pair[0];
        let (right_percent, right_color) = pair[1];
        if clamped >= left_percent && clamped <= right_percent {
            let t = (clamped - left_percent) / (right_percent - left_percent);
            return (
                lerp_u8(left_color.0, right_color.0, t),
                lerp_u8(left_color.1, right_color.1, t),
                lerp_u8(left_color.2, right_color.2, t),
            );
        }
    }
    STOPS[STOPS.len() - 1].1
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn build_tray_icon(direction: TrendDirection, (r, g, b): (u8, u8, u8)) -> Image<'static> {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let alpha = match direction {
                TrendDirection::Up => up_arrow_alpha(x, y),
                TrendDirection::Down => down_arrow_alpha(x, y),
                TrendDirection::Flat => flat_line_alpha(x, y),
            };
            if alpha == 0 {
                continue;
            }
            let shine = if y < 10 { 18 } else { 0 };
            let idx = ((y as u32 * size + x as u32) * 4) as usize;
            rgba[idx] = r.saturating_add(shine);
            rgba[idx + 1] = g.saturating_add(shine);
            rgba[idx + 2] = b.saturating_add(shine);
            rgba[idx + 3] = alpha;
        }
    }

    Image::new_owned(rgba, size, size)
}

fn up_arrow_alpha(x: i32, y: i32) -> u8 {
    let in_triangle = (5..=26).contains(&y) && (x - 16).abs() * 21 <= (y - 5) * 13;
    shape_alpha(in_triangle, x, y)
}

fn down_arrow_alpha(x: i32, y: i32) -> u8 {
    let in_triangle = (5..=26).contains(&y) && (x - 16).abs() * 21 <= (26 - y) * 13;
    shape_alpha(in_triangle, x, y)
}

fn flat_line_alpha(x: i32, y: i32) -> u8 {
    let in_line = (6..=26).contains(&x) && (13..=19).contains(&y);
    shape_alpha(in_line, x, y)
}

fn shape_alpha(on_shape: bool, x: i32, y: i32) -> u8 {
    if !on_shape {
        return 0;
    }
    if x <= 1 || x >= 30 || y <= 1 || y >= 30 {
        180
    } else {
        245
    }
}

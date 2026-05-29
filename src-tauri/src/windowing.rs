use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::{
    models::AppStatePayload,
    state::{current_payload, SharedState},
};

pub(crate) fn hide_popup_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("popup") {
        window.hide().map_err(|e| e.to_string())?;
    }
    bump_popup_token(app);
    Ok(())
}

pub(crate) fn set_popup_hovered_state(app: &AppHandle, hovered: bool) -> Result<(), String> {
    let should_hide = {
        let state = app.state::<SharedState>();
        let result = state.0.lock();
        if let Ok(mut guard) = result {
            guard.popup_hovered = hovered;
            if hovered {
                false
            } else if guard.popup_hide_pending {
                guard.popup_hide_pending = false;
                guard.popup_token = guard.popup_token.wrapping_add(1);
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    if should_hide {
        if let Some(window) = app.get_webview_window("popup") {
            window.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn toggle_popup(app: &AppHandle, pos: Option<(i32, i32)>) {
    if let Some(window) = app.get_webview_window("popup") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            bump_popup_token(app);
            return;
        }
        let payload = current_payload(app);
        let (width, height) = popup_dimensions(payload.as_ref());
        let (auto_hide_ms, token) = arm_popup_token(app);
        if let Some((x, y)) = pos {
            let _ = window.set_size(tauri::Size::Physical(PhysicalSize { width, height }));
            let position = clamped_popup_position(&window, x, y, width, height);
            let _ = window.set_position(tauri::Position::Physical(position));
        } else {
            let _ = window.set_size(tauri::Size::Physical(PhysicalSize { width, height }));
        }
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("stocktray-state", payload);
        schedule_auto_hide(app.clone(), auto_hide_ms, token);
    }
}

pub(crate) fn show_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("stocktray-state", current_payload(app));
    }
}

pub(crate) fn emit_state_to_windows(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("popup") {
        let _ = window.emit("stocktray-state", current_payload(app));
    }
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.emit("stocktray-state", current_payload(app));
    }
}

fn bump_popup_token(app: &AppHandle) {
    let state = app.state::<SharedState>();
    let result = state.0.lock();
    if let Ok(mut guard) = result {
        guard.popup_token = guard.popup_token.wrapping_add(1);
        guard.popup_hide_pending = false;
    }
}

fn arm_popup_token(app: &AppHandle) -> (u32, u64) {
    let state = app.state::<SharedState>();
    let result = state.0.lock();
    if let Ok(mut guard) = result {
        guard.popup_token = guard.popup_token.wrapping_add(1);
        guard.popup_hovered = false;
        guard.popup_hide_pending = false;
        (guard.config.popup.auto_hide_ms, guard.popup_token)
    } else {
        (0, 0)
    }
}

fn schedule_auto_hide(app: AppHandle, auto_hide_ms: u32, token: u64) {
    if auto_hide_ms == 0 {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(auto_hide_ms as u64));
        let should_hide = {
            let state = app.state::<SharedState>();
            state
                .0
                .lock()
                .map(|mut guard| {
                    let is_current = guard.popup_token == token;
                    if is_current && guard.popup_hovered {
                        guard.popup_hide_pending = true;
                        false
                    } else {
                        is_current
                    }
                })
                .unwrap_or(false)
        };
        if should_hide {
            if let Some(window) = app.get_webview_window("popup") {
                let _ = window.hide();
            }
            bump_popup_token(&app);
        }
    });
}

fn popup_dimensions(payload: Option<&AppStatePayload>) -> (u32, u32) {
    let visible_rows = payload
        .and_then(|p| p.summary.as_ref())
        .map(|summary| {
            summary
                .items
                .iter()
                .filter(|item| item.show_in_popup)
                .count()
        })
        .unwrap_or(4)
        .max(1);
    let field_count = payload
        .map(|p| p.config.display_fields.len())
        .unwrap_or(4)
        .clamp(1, 16);
    let has_summary = payload
        .map(|p| {
            p.config.show_daily_summary
                && p.summary
                    .as_ref()
                    .map(|s| s.total_prev_value > 0.0)
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    let compact = field_count <= 3;
    let balanced = field_count <= 6;
    let width = if compact { 860 } else { 820 };
    let metric_rows = ((field_count as u32 + 2) / 3).max(1);
    let row_height = if compact {
        88
    } else if balanced {
        68 + metric_rows * 46
    } else {
        66 + metric_rows * 46
    };
    let cards_per_row = if compact || balanced { 2 } else { 1 };
    let row_groups = if cards_per_row > 1 {
        ((visible_rows as u32 + cards_per_row - 1) / cards_per_row).clamp(1, 5)
    } else {
        (visible_rows as u32).clamp(1, 5)
    };
    let height = (22 + row_groups * row_height + if has_summary { 52 } else { 0 }).clamp(320, 780);
    (width, height)
}

fn clamped_popup_position(
    window: &WebviewWindow,
    anchor_x: i32,
    anchor_y: i32,
    width: u32,
    height: u32,
) -> PhysicalPosition<i32> {
    let desired = PhysicalPosition {
        x: anchor_x - width as i32 / 2,
        y: anchor_y - height as i32 - 12,
    };
    let Some(bounds) = monitor_bounds_for_anchor(window, anchor_x, anchor_y) else {
        return desired;
    };

    let min_x = bounds.x;
    let min_y = bounds.y;
    let max_x = bounds.x + bounds.width as i32 - width as i32;
    let max_y = bounds.y + bounds.height as i32 - height as i32;
    PhysicalPosition {
        x: desired.x.clamp(min_x, max_x.max(min_x)),
        y: desired.y.clamp(min_y, max_y.max(min_y)),
    }
}

struct MonitorBounds {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn monitor_bounds_for_anchor(
    window: &WebviewWindow,
    anchor_x: i32,
    anchor_y: i32,
) -> Option<MonitorBounds> {
    let monitors = window.available_monitors().ok()?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            anchor_x >= position.x
                && anchor_y >= position.y
                && anchor_x < position.x + size.width as i32
                && anchor_y < position.y + size.height as i32
        })
        .or_else(|| monitors.first())?;
    let position = monitor.position();
    let size = monitor.size();
    Some(MonitorBounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

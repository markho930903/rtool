use crate::app::state::AppState;
use crate::constants::{
    CLIPBOARD_COMPACT_WIDTH_LOGICAL, CLIPBOARD_MIN_HEIGHT_LOGICAL, CLIPBOARD_REGULAR_WIDTH_LOGICAL,
    CLIPBOARD_WINDOW_LABEL,
};
use anyhow::Context;
use app_core::models::ClipboardWindowModeAppliedDto;
use app_core::{AppError, AppResult, ResultExt};
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition};

fn clamp_clipboard_window_position(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitor: &tauri::Monitor,
) -> (i32, i32) {
    let work_area = monitor.work_area();
    let min_x = work_area.position.x;
    let min_y = work_area.position.y;
    let max_x = min_x + work_area.size.width.saturating_sub(width) as i32;
    let max_y = min_y + work_area.size.height.saturating_sub(height) as i32;
    let clamped_x = x.clamp(min_x, max_x.max(min_x));
    let clamped_y = y.clamp(min_y, max_y.max(min_y));
    (clamped_x, clamped_y)
}

pub(crate) fn apply_clipboard_window_mode(
    app: &AppHandle,
    compact: bool,
    source: &str,
) -> AppResult<ClipboardWindowModeAppliedDto> {
    let window = app
        .get_webview_window(CLIPBOARD_WINDOW_LABEL)
        .ok_or_else(|| AppError::new("clipboard_window_not_found", "剪贴板窗口不存在"))?;

    let scale_factor = window
        .scale_factor()
        .with_context(|| "读取窗口缩放比例失败".to_string())
        .with_code("clipboard_window_resize_failed", "设置剪贴板窗口尺寸失败")
        .map_err(|error| error.with_context("source", source))?
        .max(0.1);
    let before_size = window
        .outer_size()
        .with_context(|| "读取窗口尺寸失败".to_string())
        .with_code("clipboard_window_resize_failed", "设置剪贴板窗口尺寸失败")
        .map_err(|error| error.with_context("source", source))?;
    let before_position = window
        .outer_position()
        .with_context(|| "读取窗口位置失败".to_string())
        .with_code("clipboard_window_resize_failed", "设置剪贴板窗口尺寸失败")
        .map_err(|error| error.with_context("source", source))?;

    let target_width_logical = if compact {
        CLIPBOARD_COMPACT_WIDTH_LOGICAL
    } else {
        CLIPBOARD_REGULAR_WIDTH_LOGICAL
    };
    let target_height_logical =
        (before_size.height as f64 / scale_factor).max(CLIPBOARD_MIN_HEIGHT_LOGICAL);
    window
        .set_size(LogicalSize::new(
            target_width_logical,
            target_height_logical,
        ))
        .with_context(|| {
            format!(
                "设置窗口尺寸失败: width={}, height={}",
                target_width_logical, target_height_logical
            )
        })
        .with_code("clipboard_window_resize_failed", "设置剪贴板窗口尺寸失败")
        .map_err(|error| error.with_context("source", source))?;

    let target_width_px = (target_width_logical * scale_factor).round().max(1.0) as u32;
    let target_height_px = (target_height_logical * scale_factor).round().max(1.0) as u32;
    let mut next_x = before_position.x;
    let mut next_y = before_position.y;
    match window.current_monitor() {
        Ok(Some(monitor)) => {
            let (x, y) = clamp_clipboard_window_position(
                next_x,
                next_y,
                target_width_px,
                target_height_px,
                &monitor,
            );
            next_x = x;
            next_y = y;
        }
        Ok(None) => {
            tracing::debug!(
                event = "clipboard_window_monitor_missing",
                source = source,
                compact = compact
            );
        }
        Err(error) => {
            tracing::warn!(
                event = "clipboard_window_monitor_read_failed",
                source = source,
                compact = compact,
                error = error.to_string()
            );
        }
    }
    if (next_x, next_y) != (before_position.x, before_position.y) {
        window
            .set_position(PhysicalPosition::new(next_x, next_y))
            .with_context(|| format!("设置窗口位置失败: x={}, y={}", next_x, next_y))
            .with_code("clipboard_window_resize_failed", "设置剪贴板窗口尺寸失败")
            .map_err(|error| error.with_context("source", source))?;
    }

    let after_size = match window.outer_size() {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(
                event = "clipboard_window_after_size_read_failed",
                source = source,
                compact = compact,
                error = error.to_string()
            );
            before_size
        }
    };
    let applied_width_logical = after_size.width as f64 / scale_factor;
    let applied_height_logical = after_size.height as f64 / scale_factor;

    tracing::info!(
        event = "clipboard_window_mode_applied",
        source = source,
        compact = compact,
        scale_factor = scale_factor,
        before_width_px = before_size.width,
        before_height_px = before_size.height,
        target_width_logical = target_width_logical,
        target_height_logical = target_height_logical,
        after_width_px = after_size.width,
        after_height_px = after_size.height,
        position_x = next_x,
        position_y = next_y
    );

    Ok(ClipboardWindowModeAppliedDto {
        compact,
        applied_width_logical,
        applied_height_logical,
        scale_factor,
    })
}

pub(crate) fn set_clipboard_window_compact_state(app: &AppHandle, compact: bool) {
    if let Some(state) = app.try_state::<AppState>() {
        state.set_clipboard_window_compact(compact);
    }
}

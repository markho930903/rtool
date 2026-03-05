use image::{RgbaImage, imageops};
use rtool_contracts::models::{
    ScreenshotCommitInputDto, ScreenshotCommitResultDto, ScreenshotDisplayDto,
    ScreenshotSessionDto, SettingsScreenshotDto,
};
use rtool_contracts::{AppError, AppResult};
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::ffi::c_void;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use time::macros::format_description;
use uuid::Uuid;
use walkdir::WalkDir;
use xcap::Monitor;

pub const SCREENSHOT_SESSION_TTL_MS: i64 = 15_000;
pub const SCREENSHOT_SHORTCUT_DEFAULT: &str = "Alt+Shift+S";
pub const SCREENSHOT_MAX_ITEMS_MIN: u32 = 50;
pub const SCREENSHOT_MAX_ITEMS_MAX: u32 = 10_000;
pub const SCREENSHOT_MAX_TOTAL_SIZE_MB_MIN: u32 = 100;
pub const SCREENSHOT_MAX_TOTAL_SIZE_MB_MAX: u32 = 20_480;
pub const SCREENSHOT_PIN_MAX_INSTANCES_MIN: u32 = 1;
pub const SCREENSHOT_PIN_MAX_INSTANCES_MAX: u32 = 6;
const SCREENSHOT_MEMORY_RELIEF_THROTTLE_MS: i64 = 1_500;

#[derive(Debug, Clone)]
pub struct ScreenshotCommitImage {
    pub result: ScreenshotCommitResultDto,
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub display_x: i32,
    pub display_y: i32,
    pub selection_x: u32,
    pub selection_y: u32,
}

#[derive(Debug, Clone)]
struct CapturedDisplay {
    dto: ScreenshotDisplayDto,
    image: Arc<RgbaImage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenshotSessionPhase {
    Active,
    Committing,
}

#[derive(Debug, Clone)]
struct ScreenshotSession {
    id: String,
    started_at_ms: i64,
    expires_at_ms: i64,
    phase: ScreenshotSessionPhase,
    display: CapturedDisplay,
}

#[derive(Default)]
struct SessionStore {
    sessions: HashMap<String, ScreenshotSession>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

fn session_store() -> &'static Mutex<SessionStore> {
    static STORE: OnceLock<Mutex<SessionStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(SessionStore::default()))
}

fn release_process_memory(reason: &'static str) {
    static LAST_RELIEF_MS: AtomicI64 = AtomicI64::new(0);
    let now = now_ms();
    let last = LAST_RELIEF_MS.load(Ordering::Relaxed);
    if last > 0 && now.saturating_sub(last) < SCREENSHOT_MEMORY_RELIEF_THROTTLE_MS {
        return;
    }
    LAST_RELIEF_MS.store(now, Ordering::Relaxed);
    release_process_memory_impl(reason);
}

#[cfg(target_os = "macos")]
fn release_process_memory_impl(reason: &'static str) {
    let zone = unsafe { malloc_default_zone() };
    let released = unsafe { malloc_zone_pressure_relief(zone, 0) };
    tracing::debug!(
        event = "screenshot_memory_pressure_relief",
        platform = "macos",
        reason,
        released
    );
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn malloc_default_zone() -> *mut c_void;
    fn malloc_zone_pressure_relief(zone: *mut c_void, goal: usize) -> usize;
}

#[cfg(target_os = "windows")]
fn release_process_memory_impl(reason: &'static str) {
    use windows_sys::Win32::System::ProcessStatus::EmptyWorkingSet;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let process = unsafe { GetCurrentProcess() };
    let ok = unsafe { EmptyWorkingSet(process) };
    if ok == 0 {
        let error = std::io::Error::last_os_error();
        tracing::debug!(
            event = "screenshot_memory_pressure_relief_failed",
            platform = "windows",
            reason,
            error = error.to_string()
        );
        return;
    }
    tracing::debug!(
        event = "screenshot_memory_pressure_relief",
        platform = "windows",
        reason
    );
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn release_process_memory_impl(_reason: &'static str) {}

fn map_xcap_error(error: impl std::fmt::Display) -> AppError {
    AppError::new("screenshot_capture_failed", "截图失败").with_context("detail", error.to_string())
}

fn session_to_dto(session: &ScreenshotSession) -> ScreenshotSessionDto {
    ScreenshotSessionDto {
        session_id: session.id.clone(),
        started_at_ms: session.started_at_ms,
        ttl_ms: SCREENSHOT_SESSION_TTL_MS,
        active_display_id: session.display.dto.id.clone(),
        displays: vec![session.display.dto.clone()],
    }
}

fn sweep_expired_sessions(store: &mut SessionStore, now: i64) -> usize {
    let before = store.sessions.len();
    store
        .sessions
        .retain(|_, session| session.expires_at_ms > now);
    before.saturating_sub(store.sessions.len())
}

fn build_display_dto(monitor: &Monitor) -> AppResult<ScreenshotDisplayDto> {
    let id = monitor.id().map_err(map_xcap_error)?.to_string();
    let name = monitor
        .name()
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("display-{id}"));
    let x = monitor.x().map_err(map_xcap_error)?;
    let y = monitor.y().map_err(map_xcap_error)?;
    let width = monitor.width().map_err(map_xcap_error)?;
    let height = monitor.height().map_err(map_xcap_error)?;
    let scale_factor = f64::from(monitor.scale_factor().unwrap_or(1.0));
    let primary = monitor.is_primary().unwrap_or(false);
    Ok(ScreenshotDisplayDto {
        id,
        name,
        x,
        y,
        width,
        height,
        scale_factor,
        primary,
    })
}

fn pick_display_index(
    displays: &[ScreenshotDisplayDto],
    requested_display_id: Option<&str>,
) -> Option<usize> {
    if let Some(requested) = requested_display_id
        && !requested.trim().is_empty()
        && let Some(index) = displays.iter().position(|item| item.id == requested)
    {
        return Some(index);
    }

    displays
        .iter()
        .position(|item| item.primary)
        .or_else(|| (!displays.is_empty()).then_some(0))
}

pub fn start_session(requested_display_id: Option<String>) -> AppResult<ScreenshotSessionDto> {
    if requested_display_id.is_none() {
        let now = now_ms();
        let mut guard = match session_store().lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        };
        if sweep_expired_sessions(&mut guard, now) > 0 {
            release_process_memory("sweep_expired_sessions");
        }
        if let Some(existing) = guard
            .sessions
            .values()
            .filter(|session| session.phase == ScreenshotSessionPhase::Active)
            .max_by_key(|session| session.started_at_ms)
        {
            return Ok(session_to_dto(existing));
        }
    }

    let monitors = Monitor::all().map_err(map_xcap_error)?;
    if monitors.is_empty() {
        return Err(AppError::new(
            "screenshot_display_not_found",
            "未检测到可用显示器",
        ));
    }

    let mut monitor_entries = Vec::with_capacity(monitors.len());
    let mut display_entries = Vec::with_capacity(monitors.len());
    for monitor in monitors {
        let dto = build_display_dto(&monitor)?;
        monitor_entries.push(monitor);
        display_entries.push(dto);
    }

    let selected_index = pick_display_index(&display_entries, requested_display_id.as_deref())
        .ok_or_else(|| AppError::new("screenshot_display_not_found", "未找到目标显示器"))?;
    let selected_dto = display_entries
        .get(selected_index)
        .cloned()
        .ok_or_else(|| AppError::new("screenshot_display_not_found", "未找到目标显示器"))?;
    let selected_monitor = monitor_entries
        .get(selected_index)
        .ok_or_else(|| AppError::new("screenshot_display_not_found", "未找到目标显示器"))?;
    let image = selected_monitor.capture_image().map_err(map_xcap_error)?;

    let session_id = Uuid::new_v4().to_string();
    let started_at_ms = now_ms();
    let expires_at_ms = started_at_ms.saturating_add(SCREENSHOT_SESSION_TTL_MS);
    let session = ScreenshotSession {
        id: session_id.clone(),
        started_at_ms,
        expires_at_ms,
        phase: ScreenshotSessionPhase::Active,
        display: CapturedDisplay {
            dto: selected_dto.clone(),
            image: Arc::new(image),
        },
    };

    let mut guard = match session_store().lock() {
        Ok(value) => value,
        Err(poisoned) => poisoned.into_inner(),
    };
    if sweep_expired_sessions(&mut guard, started_at_ms) > 0 {
        release_process_memory("sweep_expired_sessions");
    }
    guard.sessions.insert(session_id.clone(), session);

    Ok(ScreenshotSessionDto {
        session_id,
        started_at_ms,
        ttl_ms: SCREENSHOT_SESSION_TTL_MS,
        active_display_id: selected_dto.id.clone(),
        displays: vec![selected_dto],
    })
}

pub fn cancel_session(session_id: &str) -> AppResult<bool> {
    let mut guard = match session_store().lock() {
        Ok(value) => value,
        Err(poisoned) => poisoned.into_inner(),
    };
    let removed = guard.sessions.remove(session_id).is_some();
    if removed {
        release_process_memory("cancel_session");
    }
    Ok(removed)
}

pub fn sweep_expired_sessions_now() -> usize {
    let now = now_ms();
    let mut guard = match session_store().lock() {
        Ok(value) => value,
        Err(poisoned) => poisoned.into_inner(),
    };
    let removed = sweep_expired_sessions(&mut guard, now);
    if removed > 0 {
        release_process_memory("sweep_expired_sessions");
    }
    removed
}

fn encode_png(image: &RgbaImage) -> AppResult<Vec<u8>> {
    use image::ColorType;
    use image::ImageEncoder;
    use image::codecs::png::PngEncoder;

    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);
    let encoder = PngEncoder::new(&mut cursor);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|error| {
            AppError::new("screenshot_encode_failed", "截图编码失败").with_source(error)
        })?;
    Ok(output)
}

fn screenshot_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("screenshots")
}

fn day_bucket() -> String {
    let now = OffsetDateTime::now_utc();
    let format = format_description!("[year]-[month]-[day]");
    now.format(&format)
        .unwrap_or_else(|_| "unknown-date".to_string())
}

fn save_png_file(app_data_dir: &Path, session_id: &str, png: &[u8]) -> AppResult<PathBuf> {
    let root = screenshot_root(app_data_dir);
    let bucket = day_bucket();
    let bucket_dir = root.join(bucket);
    std::fs::create_dir_all(&bucket_dir).map_err(|error| {
        AppError::new("screenshot_archive_create_failed", "创建截图目录失败")
            .with_source(error)
            .with_context("path", bucket_dir.to_string_lossy().to_string())
    })?;

    let timestamp = now_ms();
    let suffix = session_id.chars().take(8).collect::<String>();
    let file_name = format!("shot-{timestamp}-{suffix}.png");
    let file_path = bucket_dir.join(file_name);
    std::fs::write(&file_path, png).map_err(|error| {
        AppError::new("screenshot_archive_write_failed", "保存截图文件失败")
            .with_source(error)
            .with_context("path", file_path.to_string_lossy().to_string())
    })?;
    Ok(file_path)
}

pub fn save_png_file_for_session(
    app_data_dir: &Path,
    session_id: &str,
    png: &[u8],
) -> AppResult<String> {
    let path = save_png_file(app_data_dir, session_id, png)?;
    Ok(path.to_string_lossy().to_string())
}

fn cleanup_archive(root: &Path, max_items: u32, max_total_size_mb: u32) {
    if !root.exists() {
        return;
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            != Some(true)
        {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        files.push((entry.into_path(), metadata.len(), modified));
    }

    files.sort_by_key(|(_, _, modified)| {
        modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis()
    });

    let mut total_size: u64 = files.iter().map(|(_, size, _)| *size).sum();
    let max_size_bytes = u64::from(max_total_size_mb) * 1024 * 1024;
    let mut remaining_items = files.len() as u32;

    for (path, size, _) in files {
        if remaining_items <= max_items && total_size <= max_size_bytes {
            break;
        }
        match std::fs::remove_file(&path) {
            Ok(_) => {
                total_size = total_size.saturating_sub(size);
                remaining_items = remaining_items.saturating_sub(1);
            }
            Err(error) => {
                tracing::warn!(
                    event = "screenshot_archive_cleanup_failed",
                    path = %path.to_string_lossy(),
                    error = error.to_string()
                );
            }
        }
    }
}

pub fn cleanup_saved_archive(app_data_dir: &Path, max_items: u32, max_total_size_mb: u32) {
    cleanup_archive(
        screenshot_root(app_data_dir).as_path(),
        max_items,
        max_total_size_mb,
    );
}

#[derive(Debug, Clone, Copy)]
struct Size2D {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy)]
struct SelectionRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn map_selection_to_image_rect(
    display: Size2D,
    image: Size2D,
    selection: SelectionRect,
) -> AppResult<SelectionRect> {
    let display_width = display.width;
    let display_height = display.height;
    let image_width = image.width;
    let image_height = image.height;
    if display_width == 0 || display_height == 0 || image_width == 0 || image_height == 0 {
        return Err(
            AppError::new("screenshot_selection_invalid", "截图区域无效")
                .with_context("displayWidth", display_width.to_string())
                .with_context("displayHeight", display_height.to_string())
                .with_context("imageWidth", image_width.to_string())
                .with_context("imageHeight", image_height.to_string()),
        );
    }

    let ratio_x = f64::from(image_width) / f64::from(display_width);
    let ratio_y = f64::from(image_height) / f64::from(display_height);

    let mut crop_x = (f64::from(selection.x) * ratio_x).floor() as u32;
    let mut crop_y = (f64::from(selection.y) * ratio_y).floor() as u32;
    crop_x = crop_x.min(image_width.saturating_sub(1));
    crop_y = crop_y.min(image_height.saturating_sub(1));

    let mut crop_right =
        (f64::from(selection.x.saturating_add(selection.width)) * ratio_x).ceil() as u32;
    let mut crop_bottom =
        (f64::from(selection.y.saturating_add(selection.height)) * ratio_y).ceil() as u32;
    crop_right = crop_right.max(crop_x.saturating_add(1)).min(image_width);
    crop_bottom = crop_bottom.max(crop_y.saturating_add(1)).min(image_height);

    let crop_width = crop_right.saturating_sub(crop_x);
    let crop_height = crop_bottom.saturating_sub(crop_y);
    if crop_width == 0 || crop_height == 0 {
        return Err(
            AppError::new("screenshot_selection_invalid", "截图区域无效")
                .with_context("cropWidth", crop_width.to_string())
                .with_context("cropHeight", crop_height.to_string())
                .with_context("cropX", crop_x.to_string())
                .with_context("cropY", crop_y.to_string()),
        );
    }

    Ok(SelectionRect {
        x: crop_x,
        y: crop_y,
        width: crop_width,
        height: crop_height,
    })
}

pub fn commit_selection(
    input: ScreenshotCommitInputDto,
    _app_data_dir: &Path,
    _settings: &SettingsScreenshotDto,
) -> AppResult<ScreenshotCommitImage> {
    let now = now_ms();
    let (session_id, display, image, expired_removed) = {
        let mut guard = match session_store().lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        };
        let expired_removed = sweep_expired_sessions(&mut guard, now);
        let session = guard
            .sessions
            .get_mut(input.session_id.as_str())
            .ok_or_else(|| {
                AppError::new("screenshot_session_not_found", "截图会话不存在或已过期")
                    .with_context("sessionId", input.session_id.clone())
            })?;
        if session.phase == ScreenshotSessionPhase::Committing {
            return Err(
                AppError::new("screenshot_session_busy", "截图会话正在处理中，请稍后重试")
                    .with_context("sessionId", input.session_id.clone()),
            );
        }
        session.phase = ScreenshotSessionPhase::Committing;
        (
            session.id.clone(),
            session.display.dto.clone(),
            Arc::clone(&session.display.image),
            expired_removed,
        )
    };

    let image_width = image.width();
    let image_height = image.height();
    let x = input.x;
    let y = input.y;
    let width = input.width;
    let height = input.height;

    let commit_result = (|| -> AppResult<ScreenshotCommitImage> {
        if width == 0 || height == 0 {
            return Err(
                AppError::new("screenshot_selection_invalid", "截图区域无效")
                    .with_context("width", width.to_string())
                    .with_context("height", height.to_string()),
            );
        }
        if x >= display.width || y >= display.height {
            return Err(
                AppError::new("screenshot_selection_invalid", "截图区域超出显示器范围")
                    .with_context("x", x.to_string())
                    .with_context("y", y.to_string()),
            );
        }
        if x.saturating_add(width) > display.width || y.saturating_add(height) > display.height {
            return Err(
                AppError::new("screenshot_selection_invalid", "截图区域超出显示器范围")
                    .with_context("x", x.to_string())
                    .with_context("y", y.to_string())
                    .with_context("width", width.to_string())
                    .with_context("height", height.to_string())
                    .with_context("displayWidth", display.width.to_string())
                    .with_context("displayHeight", display.height.to_string()),
            );
        }

        let crop = map_selection_to_image_rect(
            Size2D {
                width: display.width,
                height: display.height,
            },
            Size2D {
                width: image_width,
                height: image_height,
            },
            SelectionRect {
                x,
                y,
                width,
                height,
            },
        )?;
        let cropped =
            imageops::crop_imm(image.as_ref(), crop.x, crop.y, crop.width, crop.height).to_image();
        let png_bytes = encode_png(&cropped)?;
        let result = ScreenshotCommitResultDto {
            session_id: session_id.clone(),
            clipboard_accepted: false,
            clipboard_async: false,
            archive_path: None,
            width: crop.width,
            height: crop.height,
            created_at_ms: now,
        };
        Ok(ScreenshotCommitImage {
            result,
            png: png_bytes,
            width: crop.width,
            height: crop.height,
            display_x: display.x,
            display_y: display.y,
            selection_x: x,
            selection_y: y,
        })
    })();

    {
        let mut guard = match session_store().lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        };
        match &commit_result {
            Ok(_) => {
                guard.sessions.remove(session_id.as_str());
            }
            Err(_) => {
                if let Some(session) = guard.sessions.get_mut(session_id.as_str()) {
                    session.phase = ScreenshotSessionPhase::Active;
                }
            }
        }
    }

    if expired_removed > 0 {
        release_process_memory("sweep_expired_sessions");
    }
    release_process_memory("commit_selection");

    commit_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::path::Path;
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

    fn tests_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        match LOCK.get_or_init(|| Mutex::new(())).lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn reset_sessions() {
        let mut guard = match session_store().lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.sessions.clear();
    }

    fn seed_session(session_id: &str, phase: ScreenshotSessionPhase) {
        let started_at_ms = now_ms();
        let mut guard = match session_store().lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.sessions.insert(
            session_id.to_string(),
            ScreenshotSession {
                id: session_id.to_string(),
                started_at_ms,
                expires_at_ms: started_at_ms + SCREENSHOT_SESSION_TTL_MS,
                phase,
                display: CapturedDisplay {
                    dto: ScreenshotDisplayDto {
                        id: "test-display".to_string(),
                        name: "Test Display".to_string(),
                        x: 0,
                        y: 0,
                        width: 120,
                        height: 90,
                        scale_factor: 1.0,
                        primary: true,
                    },
                    image: Arc::new(RgbaImage::from_pixel(120, 90, Rgba([255, 0, 0, 255]))),
                },
            },
        );
    }

    fn session_phase(session_id: &str) -> Option<ScreenshotSessionPhase> {
        let guard = match session_store().lock() {
            Ok(value) => value,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.sessions.get(session_id).map(|session| session.phase)
    }

    #[test]
    fn map_selection_identity_ratio() {
        let rect = map_selection_to_image_rect(
            Size2D {
                width: 1000,
                height: 500,
            },
            Size2D {
                width: 1000,
                height: 500,
            },
            SelectionRect {
                x: 100,
                y: 50,
                width: 200,
                height: 100,
            },
        )
        .unwrap();
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (100, 50, 200, 100)
        );
    }

    #[test]
    fn map_selection_retina_ratio_two() {
        let rect = map_selection_to_image_rect(
            Size2D {
                width: 1728,
                height: 1117,
            },
            Size2D {
                width: 3456,
                height: 2234,
            },
            SelectionRect {
                x: 100,
                y: 50,
                width: 300,
                height: 120,
            },
        )
        .unwrap();
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (200, 100, 600, 240)
        );
    }

    #[test]
    fn map_selection_clamps_at_image_boundary() {
        let rect = map_selection_to_image_rect(
            Size2D {
                width: 1728,
                height: 1117,
            },
            Size2D {
                width: 3456,
                height: 2234,
            },
            SelectionRect {
                x: 1700,
                y: 1100,
                width: 28,
                height: 17,
            },
        )
        .unwrap();
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (3400, 2200, 56, 34)
        );
    }

    #[test]
    fn map_selection_rejects_zero_dimensions() {
        let _lock = tests_lock();
        reset_sessions();
        let error = map_selection_to_image_rect(
            Size2D {
                width: 0,
                height: 1117,
            },
            Size2D {
                width: 3456,
                height: 2234,
            },
            SelectionRect {
                x: 100,
                y: 100,
                width: 10,
                height: 10,
            },
        )
        .expect_err("expected invalid selection");
        assert_eq!(error.code, "screenshot_selection_invalid");
    }

    #[test]
    fn commit_selection_success_removes_session() {
        let _lock = tests_lock();
        reset_sessions();
        let session_id = "test-commit-success";
        seed_session(session_id, ScreenshotSessionPhase::Active);

        let commit = commit_selection(
            ScreenshotCommitInputDto {
                session_id: session_id.to_string(),
                x: 10,
                y: 12,
                width: 30,
                height: 20,
                auto_save: None,
            },
            Path::new("."),
            &SettingsScreenshotDto::default(),
        )
        .expect("commit selection should succeed");

        assert_eq!(commit.result.session_id, session_id);
        assert_eq!(commit.result.width, 30);
        assert_eq!(commit.result.height, 20);
        assert!(session_phase(session_id).is_none());
    }

    #[test]
    fn commit_selection_failure_restores_active_phase() {
        let _lock = tests_lock();
        reset_sessions();
        let session_id = "test-commit-retry";
        seed_session(session_id, ScreenshotSessionPhase::Active);

        let error = commit_selection(
            ScreenshotCommitInputDto {
                session_id: session_id.to_string(),
                x: 10,
                y: 12,
                width: 0,
                height: 20,
                auto_save: None,
            },
            Path::new("."),
            &SettingsScreenshotDto::default(),
        )
        .expect_err("commit should fail for invalid selection");
        assert_eq!(error.code, "screenshot_selection_invalid");
        assert_eq!(
            session_phase(session_id),
            Some(ScreenshotSessionPhase::Active)
        );

        let retry = commit_selection(
            ScreenshotCommitInputDto {
                session_id: session_id.to_string(),
                x: 10,
                y: 12,
                width: 30,
                height: 20,
                auto_save: None,
            },
            Path::new("."),
            &SettingsScreenshotDto::default(),
        )
        .expect("retry commit should succeed");
        assert_eq!(retry.result.session_id, session_id);
        assert!(session_phase(session_id).is_none());
    }

    #[test]
    fn commit_selection_rejects_busy_session() {
        let _lock = tests_lock();
        reset_sessions();
        let session_id = "test-commit-busy";
        seed_session(session_id, ScreenshotSessionPhase::Committing);

        let error = commit_selection(
            ScreenshotCommitInputDto {
                session_id: session_id.to_string(),
                x: 1,
                y: 1,
                width: 10,
                height: 10,
                auto_save: None,
            },
            Path::new("."),
            &SettingsScreenshotDto::default(),
        )
        .expect_err("busy session should reject commit");
        assert_eq!(error.code, "screenshot_session_busy");
        assert_eq!(
            session_phase(session_id),
            Some(ScreenshotSessionPhase::Committing)
        );
    }
}

use crate::app::state::AppState;
use crate::constants::{
    RUNTIME_WORKER_SCREENSHOT, SCREENSHOT_OPERATION_RESULT_EVENT, SCREENSHOT_PIN_WINDOW_LABELS,
    SCREENSHOT_PIN_WINDOW_OPENED_EVENT,
};
use crate::shared::command_response::CommandPayloadContext;
use crate::shared::command_runtime::{run_command_async, run_command_sync};
use crate::shared::request_context::InvokeMeta;
use rtool_app::ScreenshotApplicationService;
use rtool_contracts::models::{
    ScreenshotCancelInputDto, ScreenshotCommitInputDto, ScreenshotCommitResultDto,
    ScreenshotOperationResultPayload, ScreenshotPinResultDto, ScreenshotPinWindowOpenedPayload,
    ScreenshotSessionDto, ScreenshotStartInputDto, SettingsScreenshotDto,
    SettingsScreenshotUpdateInputDto, SettingsUpdateInputDto,
};
use rtool_contracts::{AppError, AppResult, InvokeError};
use rtool_kernel::RuntimeBudget;
use rtool_kernel::RuntimeOrchestrator;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State};
use tokio::sync::Semaphore;

const SCREENSHOT_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "screenshot",
    "截图命令参数无效",
    "截图命令返回序列化失败",
    "未知截图命令",
);

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct StartPayload {
    input: ScreenshotStartInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitPayload {
    input: ScreenshotCommitInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelPayload {
    input: ScreenshotCancelInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateSettingsPayload {
    input: SettingsScreenshotUpdateInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum ScreenshotRequest {
    StartSession(StartPayload),
    CommitSelection(CommitPayload),
    PinSelection(CommitPayload),
    CancelSession(CancelPayload),
    GetSettings,
    UpdateSettings(UpdateSettingsPayload),
}

#[derive(Debug, Clone)]
struct ScreenshotPinSlotState {
    image_path: String,
}

#[derive(Default)]
struct ScreenshotPinStateStore {
    slots: HashMap<String, ScreenshotPinSlotState>,
    sequence: u64,
}

fn pin_state_store() -> &'static Mutex<ScreenshotPinStateStore> {
    static STORE: OnceLock<Mutex<ScreenshotPinStateStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(ScreenshotPinStateStore::default()))
}

fn archive_task_semaphore() -> &'static Arc<Semaphore> {
    static STORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
    STORE.get_or_init(|| {
        Arc::new(Semaphore::new(
            RuntimeBudget::global().screenshot_archive_concurrency,
        ))
    })
}

fn clipboard_task_semaphore() -> &'static Arc<Semaphore> {
    static STORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
    STORE.get_or_init(|| {
        Arc::new(Semaphore::new(
            RuntimeBudget::global().screenshot_clipboard_concurrency,
        ))
    })
}

fn screenshot_pending_jobs() -> &'static AtomicU32 {
    static STORE: OnceLock<AtomicU32> = OnceLock::new();
    STORE.get_or_init(|| AtomicU32::new(0))
}

fn on_screenshot_job_queued(orchestrator: &RuntimeOrchestrator) {
    let pending = screenshot_pending_jobs()
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    orchestrator.mark_running(RUNTIME_WORKER_SCREENSHOT);
    orchestrator.set_queue_depth(
        RUNTIME_WORKER_SCREENSHOT,
        usize::try_from(pending).unwrap_or(usize::MAX),
    );
}

fn on_screenshot_job_finished(orchestrator: &RuntimeOrchestrator) {
    let previous = screenshot_pending_jobs()
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_sub(1))
        })
        .unwrap_or(0);
    let pending = previous.saturating_sub(1);
    orchestrator.set_queue_depth(
        RUNTIME_WORKER_SCREENSHOT,
        usize::try_from(pending).unwrap_or(usize::MAX),
    );
}

fn available_pin_labels(max_instances: u32) -> Vec<&'static str> {
    let limit = max_instances.clamp(
        ScreenshotApplicationService::PIN_MAX_INSTANCES_MIN,
        ScreenshotApplicationService::PIN_MAX_INSTANCES_MAX,
    ) as usize;
    SCREENSHOT_PIN_WINDOW_LABELS[..limit].to_vec()
}

fn choose_pin_window_label(app: &AppHandle, max_instances: u32) -> AppResult<String> {
    let labels = available_pin_labels(max_instances);
    if labels.is_empty() {
        return Err(AppError::new(
            "screenshot_pin_window_unavailable",
            "截图贴图窗口不可用",
        ));
    }

    for label in &labels {
        let window = app.get_webview_window(label).ok_or_else(|| {
            AppError::new("screenshot_pin_window_not_found", "截图贴图窗口未注册")
                .with_context("windowLabel", (*label).to_string())
        })?;
        if window.is_visible().unwrap_or(false) {
            continue;
        }
        return Ok((*label).to_string());
    }

    Err(AppError::new(
        "screenshot_pin_limit_reached",
        "截图贴图数量已达上限，请先关闭部分贴图",
    ))
}

fn screenshot_pin_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir
        .join("clipboard_previews")
        .join("screenshot_pins")
}

fn cleanup_pin_image_file(path: &str) {
    let target = Path::new(path);
    if let Err(error) = std::fs::remove_file(target)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            event = "screenshot_pin_file_cleanup_failed",
            path = path,
            error = error.to_string()
        );
    }
}

fn save_pin_image_file(
    app_data_dir: &Path,
    window_label: &str,
    sequence: u64,
    created_at_ms: i64,
    png: &[u8],
) -> AppResult<String> {
    let root = screenshot_pin_root(app_data_dir);
    std::fs::create_dir_all(&root).map_err(|error| {
        AppError::new("screenshot_pin_dir_create_failed", "创建截图贴图目录失败")
            .with_source(error)
            .with_context("path", root.to_string_lossy().to_string())
    })?;

    let file_name = format!("{window_label}-{created_at_ms}-{sequence}.png");
    let file_path = root.join(file_name);
    std::fs::write(&file_path, png).map_err(|error| {
        AppError::new("screenshot_pin_file_write_failed", "写入截图贴图文件失败")
            .with_source(error)
            .with_context("path", file_path.to_string_lossy().to_string())
    })?;

    Ok(file_path.to_string_lossy().to_string())
}

fn apply_pin_window_geometry(
    app: &AppHandle,
    window_label: &str,
    screen_x: i32,
    screen_y: i32,
    width: u32,
    height: u32,
) -> AppResult<()> {
    let window = app.get_webview_window(window_label).ok_or_else(|| {
        AppError::new("screenshot_pin_window_not_found", "截图贴图窗口未注册")
            .with_context("windowLabel", window_label.to_string())
    })?;

    window.set_always_on_top(true).map_err(|error| {
        AppError::new(
            "screenshot_pin_window_topmost_failed",
            "设置截图贴图窗口置顶失败",
        )
        .with_source(error)
        .with_context("windowLabel", window_label.to_string())
    })?;
    window
        .set_position(LogicalPosition::new(
            f64::from(screen_x),
            f64::from(screen_y),
        ))
        .map_err(|error| {
            AppError::new(
                "screenshot_pin_window_position_failed",
                "设置截图贴图窗口位置失败",
            )
            .with_source(error)
            .with_context("windowLabel", window_label.to_string())
        })?;
    window
        .set_size(LogicalSize::new(f64::from(width), f64::from(height)))
        .map_err(|error| {
            AppError::new(
                "screenshot_pin_window_size_failed",
                "设置截图贴图窗口尺寸失败",
            )
            .with_source(error)
            .with_context("windowLabel", window_label.to_string())
        })?;
    window.show().map_err(|error| {
        AppError::new("screenshot_pin_window_show_failed", "显示截图贴图窗口失败")
            .with_source(error)
            .with_context("windowLabel", window_label.to_string())
    })?;

    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| i64::try_from(value.as_millis()).ok())
        .unwrap_or_default()
}

fn emit_screenshot_operation_result(app: &AppHandle, payload: ScreenshotOperationResultPayload) {
    if let Err(error) = app.emit(SCREENSHOT_OPERATION_RESULT_EVENT, payload) {
        tracing::warn!(
            event = "screenshot_operation_result_emit_failed",
            event_name = SCREENSHOT_OPERATION_RESULT_EVENT,
            error = error.to_string()
        );
    }
}

struct ScreenshotArchiveJob {
    app: AppHandle,
    orchestrator: RuntimeOrchestrator,
    app_data_dir: PathBuf,
    session_id: String,
    png: Arc<Vec<u8>>,
    should_auto_save: bool,
    max_items: u32,
    max_total_size_mb: u32,
}

fn schedule_screenshot_archive_job(job: ScreenshotArchiveJob) {
    let ScreenshotArchiveJob {
        app,
        orchestrator,
        app_data_dir,
        session_id,
        png,
        should_auto_save,
        max_items,
        max_total_size_mb,
    } = job;
    on_screenshot_job_queued(&orchestrator);
    let semaphore = Arc::clone(archive_task_semaphore());
    let orchestrator_for_task = orchestrator.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let Ok(permit) = semaphore.acquire_owned().await else {
            tracing::warn!(event = "screenshot_archive_queue_acquire_failed");
            on_screenshot_job_finished(&orchestrator_for_task);
            return;
        };
        let session_id_for_worker = session_id.clone();
        let app_for_worker = app.clone();
        let worker_result =
            tauri::async_runtime::spawn_blocking(move || -> AppResult<Option<String>> {
                let screenshot_service = ScreenshotApplicationService;
                let archive_path = if should_auto_save {
                    Some(screenshot_service.save_png_file_for_session(
                        app_data_dir.as_path(),
                        session_id_for_worker.as_str(),
                        png.as_slice(),
                    )?)
                } else {
                    None
                };
                screenshot_service.cleanup_saved_archive(
                    app_data_dir.as_path(),
                    max_items,
                    max_total_size_mb,
                );
                Ok(archive_path)
            })
            .await;
        drop(permit);

        match worker_result {
            Ok(Ok(archive_path)) => {
                emit_screenshot_operation_result(
                    &app_for_worker,
                    ScreenshotOperationResultPayload {
                        session_id,
                        operation: "archive".to_string(),
                        phase: "archive".to_string(),
                        ok: true,
                        archive_path,
                        error_code: None,
                        error_message: None,
                        created_at_ms: now_ms(),
                    },
                );
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    event = "screenshot_archive_async_failed",
                    code = error.code.as_str(),
                    message = error.message.as_str()
                );
                emit_screenshot_operation_result(
                    &app_for_worker,
                    ScreenshotOperationResultPayload {
                        session_id,
                        operation: "archive".to_string(),
                        phase: "archive".to_string(),
                        ok: false,
                        archive_path: None,
                        error_code: Some(error.code.clone()),
                        error_message: Some(error.message.clone()),
                        created_at_ms: now_ms(),
                    },
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "screenshot_archive_async_join_failed",
                    error = error.to_string()
                );
                emit_screenshot_operation_result(
                    &app_for_worker,
                    ScreenshotOperationResultPayload {
                        session_id,
                        operation: "archive".to_string(),
                        phase: "archive".to_string(),
                        ok: false,
                        archive_path: None,
                        error_code: Some("screenshot_archive_task_join_failed".to_string()),
                        error_message: Some(error.to_string()),
                        created_at_ms: now_ms(),
                    },
                );
            }
        }
        on_screenshot_job_finished(&orchestrator_for_task);
    });
    std::mem::drop(handle);
}

fn schedule_screenshot_clipboard_write(
    app: AppHandle,
    orchestrator: RuntimeOrchestrator,
    session_id: String,
    operation: &'static str,
    png: Arc<Vec<u8>>,
) {
    on_screenshot_job_queued(&orchestrator);
    let semaphore = Arc::clone(clipboard_task_semaphore());
    let orchestrator_for_task = orchestrator.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let Ok(permit) = semaphore.acquire_owned().await else {
            tracing::warn!(event = "screenshot_clipboard_queue_acquire_failed");
            on_screenshot_job_finished(&orchestrator_for_task);
            return;
        };
        let app_for_worker = app.clone();
        let worker_result = tauri::async_runtime::spawn_blocking(move || {
            let clipboard = app_for_worker.state::<tauri_plugin_clipboard::Clipboard>();
            clipboard.write_image_binary(png.as_ref().clone())
        })
        .await;
        drop(permit);

        match worker_result {
            Ok(Ok(())) => {
                emit_screenshot_operation_result(
                    &app,
                    ScreenshotOperationResultPayload {
                        session_id,
                        operation: operation.to_string(),
                        phase: "clipboard".to_string(),
                        ok: true,
                        archive_path: None,
                        error_code: None,
                        error_message: None,
                        created_at_ms: now_ms(),
                    },
                );
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    event = "screenshot_clipboard_write_failed_async",
                    operation,
                    error
                );
                emit_screenshot_operation_result(
                    &app,
                    ScreenshotOperationResultPayload {
                        session_id,
                        operation: operation.to_string(),
                        phase: "clipboard".to_string(),
                        ok: false,
                        archive_path: None,
                        error_code: Some("screenshot_clipboard_write_failed".to_string()),
                        error_message: Some(error),
                        created_at_ms: now_ms(),
                    },
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "screenshot_clipboard_write_join_failed_async",
                    operation,
                    error = error.to_string()
                );
                emit_screenshot_operation_result(
                    &app,
                    ScreenshotOperationResultPayload {
                        session_id,
                        operation: operation.to_string(),
                        phase: "clipboard".to_string(),
                        ok: false,
                        archive_path: None,
                        error_code: Some("screenshot_clipboard_write_task_join_failed".to_string()),
                        error_message: Some(error.to_string()),
                        created_at_ms: now_ms(),
                    },
                );
            }
        }
        on_screenshot_job_finished(&orchestrator_for_task);
    });
    std::mem::drop(handle);
}

async fn screenshot_start_session(
    request: Option<ScreenshotStartInputDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ScreenshotSessionDto, InvokeError> {
    run_command_async(
        "screenshot_start_session",
        request_id,
        window_label,
        move || async move {
            ScreenshotApplicationService.start_session(request.and_then(|value| value.display_id))
        },
    )
    .await
}

async fn screenshot_commit_selection(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ScreenshotCommitInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ScreenshotCommitResultDto, InvokeError> {
    run_command_async(
        "screenshot_commit_selection",
        request_id,
        window_label,
        move || async move {
            let orchestrator = state.runtime_orchestrator.clone();
            let screenshot_service = state.app_services.screenshot;
            let settings = state.app_services.settings.load_or_init().await?;
            let screenshot_settings = settings.screenshot;
            let should_auto_save = input
                .auto_save
                .unwrap_or(screenshot_settings.auto_save_enabled);
            let app_data_dir = app.path().app_data_dir().map_err(|error| {
                AppError::new("screenshot_app_data_dir_unavailable", "无法访问截图目录")
                    .with_source(error)
            })?;
            let mut commit =
                screenshot_service.commit_selection(input, &app_data_dir, &screenshot_settings)?;
            let session_id = commit.result.session_id.clone();
            let png = Arc::new(commit.png);
            schedule_screenshot_archive_job(ScreenshotArchiveJob {
                app: app.clone(),
                orchestrator: orchestrator.clone(),
                app_data_dir: app_data_dir.clone(),
                session_id: session_id.clone(),
                png: Arc::clone(&png),
                should_auto_save,
                max_items: screenshot_settings.max_items,
                max_total_size_mb: screenshot_settings.max_total_size_mb,
            });
            commit.result.clipboard_accepted = true;
            commit.result.clipboard_async = true;
            schedule_screenshot_clipboard_write(
                app.clone(),
                orchestrator,
                session_id,
                "commit",
                png,
            );
            Ok::<ScreenshotCommitResultDto, AppError>(commit.result)
        },
    )
    .await
}

async fn screenshot_pin_selection(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ScreenshotCommitInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ScreenshotPinResultDto, InvokeError> {
    run_command_async(
        "screenshot_pin_selection",
        request_id,
        window_label,
        move || async move {
            let orchestrator = state.runtime_orchestrator.clone();
            let screenshot_service = state.app_services.screenshot;
            let pin_window_width = input.width;
            let pin_window_height = input.height;
            let settings = state.app_services.settings.load_or_init().await?;
            let screenshot_settings = settings.screenshot;
            let should_auto_save = input
                .auto_save
                .unwrap_or(screenshot_settings.auto_save_enabled);
            let pin_max_instances = screenshot_settings.pin_max_instances;
            let app_data_dir = app.path().app_data_dir().map_err(|error| {
                AppError::new("screenshot_app_data_dir_unavailable", "无法访问截图目录")
                    .with_source(error)
            })?;
            let commit =
                screenshot_service.commit_selection(input, &app_data_dir, &screenshot_settings)?;
            let session_id = commit.result.session_id.clone();
            let png = Arc::new(commit.png);
            schedule_screenshot_archive_job(ScreenshotArchiveJob {
                app: app.clone(),
                orchestrator: orchestrator.clone(),
                app_data_dir: app_data_dir.clone(),
                session_id: session_id.clone(),
                png: Arc::clone(&png),
                should_auto_save,
                max_items: screenshot_settings.max_items,
                max_total_size_mb: screenshot_settings.max_total_size_mb,
            });

            let pinned_window_label = choose_pin_window_label(&app, pin_max_instances)?;
            let (previous_path, sequence) = {
                let mut guard = match pin_state_store().lock() {
                    Ok(value) => value,
                    Err(poisoned) => poisoned.into_inner(),
                };
                guard.sequence = guard.sequence.saturating_add(1);
                let previous = guard
                    .slots
                    .get(pinned_window_label.as_str())
                    .map(|item| item.image_path.clone());
                (previous, guard.sequence)
            };
            if let Some(path) = previous_path {
                cleanup_pin_image_file(path.as_str());
            }

            let image_path = save_pin_image_file(
                &app_data_dir,
                pinned_window_label.as_str(),
                sequence,
                commit.result.created_at_ms,
                png.as_slice(),
            )?;
            let screen_x = commit
                .display_x
                .saturating_add(commit.selection_x.try_into().unwrap_or(i32::MAX));
            let screen_y = commit
                .display_y
                .saturating_add(commit.selection_y.try_into().unwrap_or(i32::MAX));
            apply_pin_window_geometry(
                &app,
                pinned_window_label.as_str(),
                screen_x,
                screen_y,
                pin_window_width,
                pin_window_height,
            )?;
            app.emit(
                SCREENSHOT_PIN_WINDOW_OPENED_EVENT,
                ScreenshotPinWindowOpenedPayload {
                    target_window_label: pinned_window_label.clone(),
                    image_path: image_path.clone(),
                    screen_x,
                    screen_y,
                    width: pin_window_width,
                    height: pin_window_height,
                    created_at_ms: commit.result.created_at_ms,
                },
            )
            .map_err(|error| {
                AppError::new(
                    "screenshot_pin_window_emit_failed",
                    "发送截图贴图窗口事件失败",
                )
                .with_source(error)
                .with_context("windowLabel", pinned_window_label.clone())
            })?;

            {
                let mut guard = match pin_state_store().lock() {
                    Ok(value) => value,
                    Err(poisoned) => poisoned.into_inner(),
                };
                guard.slots.insert(
                    pinned_window_label.clone(),
                    ScreenshotPinSlotState { image_path },
                );
            }
            let result = ScreenshotPinResultDto {
                session_id,
                clipboard_accepted: true,
                clipboard_async: true,
                window_label: pinned_window_label,
                width: pin_window_width,
                height: pin_window_height,
                created_at_ms: commit.result.created_at_ms,
            };
            schedule_screenshot_clipboard_write(
                app.clone(),
                orchestrator,
                result.session_id.clone(),
                "pin",
                png,
            );
            Ok::<ScreenshotPinResultDto, AppError>(result)
        },
    )
    .await
}

fn screenshot_cancel_session(
    input: ScreenshotCancelInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<bool, InvokeError> {
    run_command_sync(
        "screenshot_cancel_session",
        request_id,
        window_label,
        move || ScreenshotApplicationService.cancel_session(input.session_id.as_str()),
    )
}

async fn screenshot_get_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<SettingsScreenshotDto, InvokeError> {
    run_command_async(
        "screenshot_get_settings",
        request_id,
        window_label,
        move || async move {
            Ok::<SettingsScreenshotDto, AppError>(
                state.app_services.settings.load_or_init().await?.screenshot,
            )
        },
    )
    .await
}

async fn screenshot_update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut input: SettingsScreenshotUpdateInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<SettingsScreenshotDto, InvokeError> {
    run_command_async(
        "screenshot_update_settings",
        request_id,
        window_label,
        move || async move {
            if let Some(shortcut) = input.shortcut.take() {
                input.shortcut = Some(
                    crate::platform::native_ui::shortcuts::normalize_screenshot_shortcut(
                        shortcut.as_str(),
                    )?,
                );
            }
            let previous_settings = state.app_services.settings.load_or_init().await?;
            let previous_shortcut = previous_settings.screenshot.shortcut;
            let requested_shortcut = input.shortcut.clone();
            let mut rebound_shortcut: Option<(String, String)> = None;
            if let Some(next_shortcut) = requested_shortcut
                && next_shortcut != previous_shortcut
            {
                crate::platform::native_ui::shortcuts::rebind_screenshot_shortcut(
                    &app,
                    previous_shortcut.as_str(),
                    next_shortcut.as_str(),
                )?;
                rebound_shortcut = Some((previous_shortcut.clone(), next_shortcut));
            }
            let settings = state
                .app_services
                .settings
                .update(SettingsUpdateInputDto {
                    screenshot: Some(input),
                    ..Default::default()
                })
                .await
                .inspect_err(|_error| {
                    if let Some((previous_shortcut, applied_shortcut)) = rebound_shortcut
                        && let Err(rebind_error) =
                            crate::platform::native_ui::shortcuts::rebind_screenshot_shortcut(
                                &app,
                                applied_shortcut.as_str(),
                                previous_shortcut.as_str(),
                            )
                    {
                        tracing::warn!(
                            event = "screenshot_shortcut_rollback_failed",
                            previous_shortcut,
                            applied_shortcut,
                            error = rebind_error.to_string()
                        );
                    }
                })?;
            Ok::<SettingsScreenshotDto, AppError>(settings.screenshot)
        },
    )
    .await
}

pub(crate) async fn handle_screenshot(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ScreenshotRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();

    match request {
        ScreenshotRequest::StartSession(payload) => SCREENSHOT_COMMAND_CONTEXT.serialize(
            "start_session",
            screenshot_start_session(Some(payload.input), request_id, window_label).await?,
        ),
        ScreenshotRequest::CommitSelection(payload) => SCREENSHOT_COMMAND_CONTEXT.serialize(
            "commit_selection",
            screenshot_commit_selection(app, state, payload.input, request_id, window_label)
                .await?,
        ),
        ScreenshotRequest::PinSelection(payload) => SCREENSHOT_COMMAND_CONTEXT.serialize(
            "pin_selection",
            screenshot_pin_selection(app, state, payload.input, request_id, window_label).await?,
        ),
        ScreenshotRequest::CancelSession(payload) => SCREENSHOT_COMMAND_CONTEXT.serialize(
            "cancel_session",
            screenshot_cancel_session(payload.input, request_id, window_label)?,
        ),
        ScreenshotRequest::GetSettings => SCREENSHOT_COMMAND_CONTEXT.serialize(
            "get_settings",
            screenshot_get_settings(state, request_id, window_label).await?,
        ),
        ScreenshotRequest::UpdateSettings(payload) => SCREENSHOT_COMMAND_CONTEXT.serialize(
            "update_settings",
            screenshot_update_settings(app, state, payload.input, request_id, window_label).await?,
        ),
    }
}

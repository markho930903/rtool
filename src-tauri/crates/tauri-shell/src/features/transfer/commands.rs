use std::path::PathBuf;
use std::process::Command;

use tauri::State;

use super::{run_command_async, run_command_sync};
use crate::app::state::AppState;
use anyhow::Context;
use app_core::models::{
    TransferClearHistoryInputDto, TransferHistoryFilterDto, TransferHistoryPageDto,
    TransferPairingCodeDto, TransferPeerDto, TransferSendFilesInputDto, TransferSessionDto,
    TransferSettingsDto, TransferUpdateSettingsInputDto,
};
use app_core::{AppError, AppResult, InvokeError, ResultExt};
use app_transfer::service::TransferService;

fn ensure_transfer_bootstrapped(service: &TransferService) -> AppResult<()> {
    service.ensure_bootstrapped()
}

fn open_path(path: &str) -> AppResult<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "transfer_open_path_invalid",
            "打开目录失败：目录不能为空",
        ));
    }

    let path_buf = PathBuf::from(trimmed);
    if !path_buf.exists() {
        return Err(
            AppError::new("transfer_open_path_not_found", "打开目录失败：目录不存在")
                .with_context("path", path_buf.to_string_lossy().to_string()),
        );
    }

    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(path_buf).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path_buf)
            .status()
    } else {
        Command::new("xdg-open").arg(path_buf).status()
    }
    .with_context(|| format!("failed to invoke system open command: {}", trimmed))
    .with_code("transfer_open_path_failed", "打开目录失败")?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::new("transfer_open_path_failed", "打开目录失败")
            .with_context("status", status.to_string()))
    }
}

#[tauri::command]
pub fn transfer_get_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferSettingsDto, InvokeError> {
    run_command_sync(
        "transfer_get_settings",
        request_id,
        window_label,
        move || Ok::<_, InvokeError>(state.transfer_service.get_settings()),
    )
}

#[tauri::command]
pub async fn transfer_update_settings(
    state: State<'_, AppState>,
    input: TransferUpdateSettingsInputDto,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<TransferSettingsDto, InvokeError> {
    let service = state.transfer_service.clone();
    service.update_settings(input).await.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_generate_pairing_code(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferPairingCodeDto, InvokeError> {
    run_command_sync(
        "transfer_generate_pairing_code",
        request_id,
        window_label,
        move || Ok::<_, InvokeError>(state.transfer_service.generate_pairing_code()),
    )
}

#[tauri::command]
pub async fn transfer_start_discovery(
    state: State<'_, AppState>,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.transfer_service.clone();
    ensure_transfer_bootstrapped(&service)?;
    service.start_discovery().map_err(Into::into)
}

#[tauri::command]
pub fn transfer_stop_discovery(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_sync(
        "transfer_stop_discovery",
        request_id,
        window_label,
        move || {
            state.transfer_service.stop_discovery();
            Ok::<_, InvokeError>(())
        },
    )
}

#[tauri::command]
pub async fn transfer_list_peers(
    state: State<'_, AppState>,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<Vec<TransferPeerDto>, InvokeError> {
    let service = state.transfer_service.clone();
    ensure_transfer_bootstrapped(&service)?;
    service.list_peers().await.map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_send_files(
    state: State<'_, AppState>,
    input: TransferSendFilesInputDto,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<TransferSessionDto, InvokeError> {
    let service = state.transfer_service.clone();
    ensure_transfer_bootstrapped(&service)?;
    service.send_files(input).await.map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_pause_session(
    state: State<'_, AppState>,
    session_id: String,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.transfer_service.clone();
    service
        .pause_session(session_id.as_str())
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_resume_session(
    state: State<'_, AppState>,
    session_id: String,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.transfer_service.clone();
    service
        .resume_session(session_id.as_str())
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_cancel_session(
    state: State<'_, AppState>,
    session_id: String,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.transfer_service.clone();
    service
        .cancel_session(session_id.as_str())
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_retry_session(
    state: State<'_, AppState>,
    session_id: String,
    _request_id: Option<String>,
    _window_label: Option<String>,
) -> Result<TransferSessionDto, InvokeError> {
    let service = state.transfer_service.clone();
    ensure_transfer_bootstrapped(&service)?;
    service
        .retry_session(session_id.as_str())
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_list_history(
    state: State<'_, AppState>,
    filter: Option<TransferHistoryFilterDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferHistoryPageDto, InvokeError> {
    let service = state.transfer_service.clone();
    run_command_async(
        "transfer_list_history",
        request_id,
        window_label,
        move || async move { service.list_history(filter.unwrap_or_default()).await },
    )
    .await
}

#[tauri::command]
pub async fn transfer_clear_history(
    state: State<'_, AppState>,
    input: Option<TransferClearHistoryInputDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.transfer_service.clone();
    run_command_async(
        "transfer_clear_history",
        request_id,
        window_label,
        move || async move { service.clear_history(input.unwrap_or_default()).await },
    )
    .await
}

#[tauri::command]
pub fn transfer_open_download_dir(
    state: State<'_, AppState>,
    path: Option<String>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    run_command_sync(
        "transfer_open_download_dir",
        request_id,
        window_label,
        move || {
            let resolved =
                path.unwrap_or_else(|| state.transfer_service.get_settings().default_download_dir);
            open_path(resolved.as_str())
        },
    )
}

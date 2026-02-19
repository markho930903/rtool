use std::path::PathBuf;
use std::process::Command;

use tauri::State;

use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::app::state::AppState;
use crate::core::models::{
    TransferClearHistoryInputDto, TransferHistoryFilterDto, TransferHistoryPageDto,
    TransferPairingCodeDto, TransferPeerDto, TransferSendFilesInputDto, TransferSessionDto,
    TransferSettingsDto, TransferUpdateSettingsInputDto,
};
use crate::core::{AppError, AppResult, InvokeError, ResultExt};
use anyhow::Context;

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
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_get_settings",
        &request_id,
        window_label.as_deref(),
    );

    let result = Ok(state.transfer_service.get_settings());
    command_end_ok("transfer_get_settings", &request_id, started_at);
    result
}

#[tauri::command]
pub fn transfer_update_settings(
    state: State<'_, AppState>,
    input: TransferUpdateSettingsInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferSettingsDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_update_settings",
        &request_id,
        window_label.as_deref(),
    );

    let result = state.transfer_service.update_settings(input);
    match &result {
        Ok(_) => command_end_ok("transfer_update_settings", &request_id, started_at),
        Err(error) => command_end_error("transfer_update_settings", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_generate_pairing_code(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferPairingCodeDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_generate_pairing_code",
        &request_id,
        window_label.as_deref(),
    );

    let result = Ok(state.transfer_service.generate_pairing_code());
    command_end_ok("transfer_generate_pairing_code", &request_id, started_at);
    result
}

#[tauri::command]
pub async fn transfer_start_discovery(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_start_discovery",
        &request_id,
        window_label.as_deref(),
    );

    state.transfer_service.start_discovery();
    command_end_ok("transfer_start_discovery", &request_id, started_at);
    Ok(())
}

#[tauri::command]
pub fn transfer_stop_discovery(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_stop_discovery",
        &request_id,
        window_label.as_deref(),
    );

    state.transfer_service.stop_discovery();
    command_end_ok("transfer_stop_discovery", &request_id, started_at);
    Ok(())
}

#[tauri::command]
pub async fn transfer_list_peers(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<TransferPeerDto>, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("transfer_list_peers", &request_id, window_label.as_deref());

    let result = state.transfer_service.list_peers().await;
    match &result {
        Ok(_) => command_end_ok("transfer_list_peers", &request_id, started_at),
        Err(error) => command_end_error("transfer_list_peers", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_send_files(
    state: State<'_, AppState>,
    input: TransferSendFilesInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferSessionDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("transfer_send_files", &request_id, window_label.as_deref());

    let result = state.transfer_service.send_files(input).await;
    match &result {
        Ok(_) => command_end_ok("transfer_send_files", &request_id, started_at),
        Err(error) => command_end_error("transfer_send_files", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_pause_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_pause_session",
        &request_id,
        window_label.as_deref(),
    );

    let result = state.transfer_service.pause_session(session_id.as_str());
    match &result {
        Ok(_) => command_end_ok("transfer_pause_session", &request_id, started_at),
        Err(error) => command_end_error("transfer_pause_session", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_resume_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_resume_session",
        &request_id,
        window_label.as_deref(),
    );

    let result = state.transfer_service.resume_session(session_id.as_str());
    match &result {
        Ok(_) => command_end_ok("transfer_resume_session", &request_id, started_at),
        Err(error) => command_end_error("transfer_resume_session", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_cancel_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_cancel_session",
        &request_id,
        window_label.as_deref(),
    );

    let result = state.transfer_service.cancel_session(session_id.as_str());
    match &result {
        Ok(_) => command_end_ok("transfer_cancel_session", &request_id, started_at),
        Err(error) => command_end_error("transfer_cancel_session", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub async fn transfer_retry_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferSessionDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_retry_session",
        &request_id,
        window_label.as_deref(),
    );

    let result = state
        .transfer_service
        .retry_session(session_id.as_str())
        .await;
    match &result {
        Ok(_) => command_end_ok("transfer_retry_session", &request_id, started_at),
        Err(error) => command_end_error("transfer_retry_session", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_list_history(
    state: State<'_, AppState>,
    filter: Option<TransferHistoryFilterDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferHistoryPageDto, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_list_history",
        &request_id,
        window_label.as_deref(),
    );

    let result = state
        .transfer_service
        .list_history(filter.unwrap_or_default());
    match &result {
        Ok(_) => command_end_ok("transfer_list_history", &request_id, started_at),
        Err(error) => command_end_error("transfer_list_history", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_clear_history(
    state: State<'_, AppState>,
    input: Option<TransferClearHistoryInputDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_clear_history",
        &request_id,
        window_label.as_deref(),
    );

    let result = state
        .transfer_service
        .clear_history(input.unwrap_or_default());
    match &result {
        Ok(_) => command_end_ok("transfer_clear_history", &request_id, started_at),
        Err(error) => command_end_error("transfer_clear_history", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn transfer_open_download_dir(
    state: State<'_, AppState>,
    path: Option<String>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "transfer_open_download_dir",
        &request_id,
        window_label.as_deref(),
    );

    let resolved =
        path.unwrap_or_else(|| state.transfer_service.get_settings().default_download_dir);
    let result = open_path(resolved.as_str());
    match &result {
        Ok(_) => command_end_ok("transfer_open_download_dir", &request_id, started_at),
        Err(error) => {
            command_end_error("transfer_open_download_dir", &request_id, started_at, error)
        }
    }
    result.map_err(Into::into)
}

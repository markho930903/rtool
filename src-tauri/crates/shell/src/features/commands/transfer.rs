use std::path::PathBuf;
use std::process::Command;

use tauri::State;

use super::{run_command_async, run_command_sync};
use crate::app::state::AppState;
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use anyhow::Context;
use protocol::models::{
    TransferClearHistoryInputDto, TransferHistoryFilterDto, TransferHistoryPageDto,
    TransferPairingCodeDto, TransferPeerDto, TransferSendFilesInputDto, TransferSessionDto,
    TransferSettingsDto, TransferUpdateSettingsInputDto,
};
use protocol::{AppError, AppResult, InvokeError, ResultExt};
use serde::Deserialize;
use serde_json::Value;

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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferUpdateSettingsPayload {
    input: TransferUpdateSettingsInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferSendFilesPayload {
    input: TransferSendFilesInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferSessionPayload {
    session_id: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct TransferListHistoryPayload {
    filter: Option<TransferHistoryFilterDto>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct TransferClearHistoryPayload {
    input: Option<TransferClearHistoryInputDto>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct TransferOpenDownloadDirPayload {
    path: Option<String>,
}

const TRANSFER_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "transfer",
    "传输命令参数无效",
    "传输命令返回序列化失败",
    "未知传输命令",
);

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
        move || Ok::<_, InvokeError>(state.app_services.transfer.get_settings()),
    )
}

#[tauri::command]
pub async fn transfer_update_settings(
    state: State<'_, AppState>,
    input: TransferUpdateSettingsInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferSettingsDto, InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_update_settings",
        request_id,
        window_label,
        move || async move { service.update_settings(input).await },
    )
    .await
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
        move || Ok::<_, InvokeError>(state.app_services.transfer.generate_pairing_code()),
    )
}

#[tauri::command]
pub async fn transfer_start_discovery(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_start_discovery",
        request_id,
        window_label,
        move || async move { service.start_discovery() },
    )
    .await
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
            state.app_services.transfer.stop_discovery();
            Ok::<_, InvokeError>(())
        },
    )
}

#[tauri::command]
pub async fn transfer_list_peers(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Vec<TransferPeerDto>, InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_list_peers",
        request_id,
        window_label,
        move || async move { service.list_peers().await },
    )
    .await
}

#[tauri::command]
pub async fn transfer_send_files(
    state: State<'_, AppState>,
    input: TransferSendFilesInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferSessionDto, InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_send_files",
        request_id,
        window_label,
        move || async move { service.send_files(input).await },
    )
    .await
}

#[tauri::command]
pub async fn transfer_pause_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_pause_session",
        request_id,
        window_label,
        move || async move { service.pause_session(session_id.as_str()).await },
    )
    .await
}

#[tauri::command]
pub async fn transfer_resume_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_resume_session",
        request_id,
        window_label,
        move || async move { service.resume_session(session_id.as_str()).await },
    )
    .await
}

#[tauri::command]
pub async fn transfer_cancel_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<(), InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_cancel_session",
        request_id,
        window_label,
        move || async move { service.cancel_session(session_id.as_str()).await },
    )
    .await
}

#[tauri::command]
pub async fn transfer_retry_session(
    state: State<'_, AppState>,
    session_id: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferSessionDto, InvokeError> {
    let service = state.app_services.transfer.clone();
    run_command_async(
        "transfer_retry_session",
        request_id,
        window_label,
        move || async move { service.retry_session(session_id.as_str()).await },
    )
    .await
}

#[tauri::command]
pub async fn transfer_list_history(
    state: State<'_, AppState>,
    filter: Option<TransferHistoryFilterDto>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<TransferHistoryPageDto, InvokeError> {
    let service = state.app_services.transfer.clone();
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
    let service = state.app_services.transfer.clone();
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
            let resolved = path.unwrap_or_else(|| {
                state
                    .app_services
                    .transfer
                    .get_settings()
                    .default_download_dir
            });
            open_path(resolved.as_str())
        },
    )
}

#[tauri::command]
pub async fn transfer_handle(
    state: State<'_, AppState>,
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "get_settings" => TRANSFER_COMMAND_CONTEXT.serialize(
            "get_settings",
            transfer_get_settings(state, request_id, window_label)?,
        ),
        "update_settings" => {
            let payload: TransferUpdateSettingsPayload =
                TRANSFER_COMMAND_CONTEXT.parse("update_settings", request.payload)?;
            TRANSFER_COMMAND_CONTEXT.serialize(
                "update_settings",
                transfer_update_settings(state, payload.input, request_id, window_label).await?,
            )
        }
        "generate_pairing_code" => TRANSFER_COMMAND_CONTEXT.serialize(
            "generate_pairing_code",
            transfer_generate_pairing_code(state, request_id, window_label)?,
        ),
        "start_discovery" => {
            transfer_start_discovery(state, request_id, window_label).await?;
            Ok(Value::Null)
        }
        "stop_discovery" => {
            transfer_stop_discovery(state, request_id, window_label)?;
            Ok(Value::Null)
        }
        "list_peers" => TRANSFER_COMMAND_CONTEXT.serialize(
            "list_peers",
            transfer_list_peers(state, request_id, window_label).await?,
        ),
        "send_files" => {
            let payload: TransferSendFilesPayload =
                TRANSFER_COMMAND_CONTEXT.parse("send_files", request.payload)?;
            TRANSFER_COMMAND_CONTEXT.serialize(
                "send_files",
                transfer_send_files(state, payload.input, request_id, window_label).await?,
            )
        }
        "pause_session" => {
            let payload: TransferSessionPayload =
                TRANSFER_COMMAND_CONTEXT.parse("pause_session", request.payload)?;
            transfer_pause_session(state, payload.session_id, request_id, window_label).await?;
            Ok(Value::Null)
        }
        "resume_session" => {
            let payload: TransferSessionPayload =
                TRANSFER_COMMAND_CONTEXT.parse("resume_session", request.payload)?;
            transfer_resume_session(state, payload.session_id, request_id, window_label).await?;
            Ok(Value::Null)
        }
        "cancel_session" => {
            let payload: TransferSessionPayload =
                TRANSFER_COMMAND_CONTEXT.parse("cancel_session", request.payload)?;
            transfer_cancel_session(state, payload.session_id, request_id, window_label).await?;
            Ok(Value::Null)
        }
        "retry_session" => {
            let payload: TransferSessionPayload =
                TRANSFER_COMMAND_CONTEXT.parse("retry_session", request.payload)?;
            TRANSFER_COMMAND_CONTEXT.serialize(
                "retry_session",
                transfer_retry_session(state, payload.session_id, request_id, window_label).await?,
            )
        }
        "list_history" => {
            let payload: TransferListHistoryPayload =
                TRANSFER_COMMAND_CONTEXT.parse("list_history", request.payload)?;
            TRANSFER_COMMAND_CONTEXT.serialize(
                "list_history",
                transfer_list_history(state, payload.filter, request_id, window_label).await?,
            )
        }
        "clear_history" => {
            let payload: TransferClearHistoryPayload =
                TRANSFER_COMMAND_CONTEXT.parse("clear_history", request.payload)?;
            transfer_clear_history(state, payload.input, request_id, window_label).await?;
            Ok(Value::Null)
        }
        "open_download_dir" => {
            let payload: TransferOpenDownloadDirPayload =
                TRANSFER_COMMAND_CONTEXT.parse("open_download_dir", request.payload)?;
            transfer_open_download_dir(state, payload.path, request_id, window_label)?;
            Ok(Value::Null)
        }
        _ => Err(TRANSFER_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

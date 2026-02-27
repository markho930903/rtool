use crate::app::state::AppState;
use crate::command_runtime::{run_command_async, run_command_sync};
use crate::features::clipboard::events::emit_clipboard_sync;
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use protocol::models::{ClipboardSyncPayload, UserSettingsDto, UserSettingsUpdateInputDto};
use protocol::{AppError, InvokeError};
use rtool_i18n::i18n::resolve_locale;
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

const SETTINGS_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "settings",
    "设置命令参数无效",
    "设置命令返回序列化失败",
    "未知设置命令",
);

#[tauri::command]
pub fn app_get_user_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<UserSettingsDto, InvokeError> {
    run_command_sync(
        "app_get_user_settings",
        request_id,
        window_label,
        move || state.app_services.settings.load_or_init(),
    )
}

#[tauri::command]
pub async fn app_update_user_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    input: UserSettingsUpdateInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<UserSettingsDto, InvokeError> {
    run_command_async(
        "app_update_user_settings",
        request_id,
        window_label,
        move || async move {
            let previous_locale = state.locale_snapshot();
            let settings = state.app_services.settings.update(input)?;

            if previous_locale.preference != settings.locale.preference {
                let resolved = resolve_locale(settings.locale.preference.as_str());
                state.update_locale(settings.locale.preference.clone(), resolved.clone());
                crate::apply_locale_to_native_ui(&app, &resolved);
            }

            let clipboard_update = state
                .app_services
                .clipboard
                .apply_user_settings(&settings.clipboard)
                .await?;
            if !clipboard_update.removed_ids.is_empty() {
                emit_clipboard_sync(
                    &app,
                    ClipboardSyncPayload {
                        upsert: Vec::new(),
                        removed_ids: clipboard_update.removed_ids,
                        clear_all: false,
                        reason: Some("user_settings_clipboard_prune".to_string()),
                    },
                );
            }

            Ok::<UserSettingsDto, AppError>(settings)
        },
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSettingsPayload {
    input: UserSettingsUpdateInputDto,
}

#[tauri::command]
pub async fn settings_handle(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "get" => SETTINGS_COMMAND_CONTEXT.serialize(
            "get",
            app_get_user_settings(state, request_id, window_label)?,
        ),
        "update" => {
            let payload: UpdateSettingsPayload =
                SETTINGS_COMMAND_CONTEXT.parse("update", request.payload)?;
            SETTINGS_COMMAND_CONTEXT.serialize(
                "update",
                app_update_user_settings(app, state, payload.input, request_id, window_label).await?,
            )
        }
        _ => Err(SETTINGS_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

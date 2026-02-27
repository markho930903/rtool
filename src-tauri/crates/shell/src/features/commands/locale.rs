use crate::app::state::AppState;
use crate::command_runtime::{run_command_async, run_command_sync};
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use protocol::{AppError, InvokeError};
use rtool_i18n::i18n::{LocaleStateDto, normalize_locale_preference, resolve_locale};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

const LOCALE_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "locale",
    "语言命令参数无效",
    "语言命令返回序列化失败",
    "未知语言命令",
);

#[tauri::command]
pub fn app_get_locale(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleStateDto, InvokeError> {
    run_command_sync("app_get_locale", request_id, window_label, move || {
        let settings = state.app_services.settings.load_or_init()?;
        let preference = normalize_locale_preference(settings.locale.preference.as_str())
            .ok_or_else(|| {
                AppError::new("invalid_locale_preference", "语言偏好无效")
                    .with_context("preference", settings.locale.preference.clone())
            })?;
        let resolved = resolve_locale(&preference);
        let locale_state = state.update_locale(preference, resolved);
        Ok::<_, AppError>(locale_state)
    })
}

#[tauri::command]
pub async fn app_set_locale(
    app: AppHandle,
    state: State<'_, AppState>,
    preference: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleStateDto, InvokeError> {
    run_command_async(
        "app_set_locale",
        request_id,
        window_label,
        move || async move {
            let canonical_preference =
                normalize_locale_preference(&preference).ok_or_else(|| {
                    AppError::new("invalid_locale_preference", "语言偏好无效")
                        .with_context("preference", preference.clone())
                })?;

            state
                .app_services
                .settings
                .update_locale_preference(canonical_preference.as_str())?;
            let resolved = resolve_locale(&canonical_preference);
            let next = state.update_locale(canonical_preference, resolved.clone());
            crate::apply_locale_to_native_ui(&app, &resolved);
            Ok::<LocaleStateDto, AppError>(next)
        },
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetLocalePayload {
    preference: String,
}

#[tauri::command]
pub async fn locale_handle(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "get" => LOCALE_COMMAND_CONTEXT.serialize(
            "get",
            app_get_locale(state, request_id, window_label)?,
        ),
        "set" => {
            let payload: SetLocalePayload = LOCALE_COMMAND_CONTEXT.parse("set", request.payload)?;
            LOCALE_COMMAND_CONTEXT.serialize(
                "set",
                app_set_locale(
                    app,
                    state,
                    payload.preference,
                    request_id,
                    window_label,
                )
                .await?,
            )
        }
        _ => Err(LOCALE_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

use crate::app::state::AppState;
use crate::shared::command_response::CommandPayloadContext;
use crate::shared::command_runtime::run_command_async;
use crate::shared::request_context::InvokeMeta;
use rtool_app::{LocaleApplicationService, LocaleStateDto};
use rtool_contracts::{AppError, InvokeError};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

const LOCALE_SYNC_EVENT: &str = "rtool://settings/locale_sync";

const LOCALE_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "locale",
    "语言命令参数无效",
    "语言命令返回序列化失败",
    "未知语言命令",
);

async fn app_get_locale(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleStateDto, InvokeError> {
    run_command_async(
        "app_get_locale",
        request_id,
        window_label,
        move || async move {
            let locale_service = LocaleApplicationService;
            let settings = state.app_services.settings.load_or_init().await?;
            let preference = locale_service
                .normalize_preference(settings.locale.preference.as_str())
                .ok_or_else(|| {
                    AppError::new("invalid_locale_preference", "语言偏好无效")
                        .with_context("preference", settings.locale.preference.clone())
                })?;
            let resolved = locale_service.resolve(preference.as_str());
            let locale_state = state.update_locale(preference, resolved);
            Ok::<_, AppError>(locale_state)
        },
    )
    .await
}

async fn app_set_locale(
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
            let locale_service = LocaleApplicationService;
            let canonical_preference = locale_service
                .normalize_preference(&preference)
                .ok_or_else(|| {
                    AppError::new("invalid_locale_preference", "语言偏好无效")
                        .with_context("preference", preference.clone())
                })?;

            state
                .app_services
                .settings
                .update_locale_preference(canonical_preference.as_str())
                .await?;
            let resolved = locale_service.resolve(canonical_preference.as_str());
            let next = state.update_locale(canonical_preference, resolved.clone());
            crate::platform::native_ui::apply_locale_to_native_ui(&app, &resolved);
            if let Err(error) = app.emit(LOCALE_SYNC_EVENT, next.clone()) {
                tracing::warn!(event = "locale_sync_emit_failed", detail = %error);
            }
            Ok::<LocaleStateDto, AppError>(next)
        },
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetLocalePayload {
    preference: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum LocaleRequest {
    Get,
    Set(SetLocalePayload),
}

pub(crate) async fn handle_locale(
    app: AppHandle,
    state: State<'_, AppState>,
    request: LocaleRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();

    match request {
        LocaleRequest::Get => LOCALE_COMMAND_CONTEXT.serialize(
            "get",
            app_get_locale(state, request_id, window_label).await?,
        ),
        LocaleRequest::Set(payload) => LOCALE_COMMAND_CONTEXT.serialize(
            "set",
            app_set_locale(app, state, payload.preference, request_id, window_label).await?,
        ),
    }
}

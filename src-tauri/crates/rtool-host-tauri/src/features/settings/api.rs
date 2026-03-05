use crate::app::state::AppState;
use crate::features::clipboard::events::emit_clipboard_sync;
use crate::shared::command_response::CommandPayloadContext;
use crate::shared::command_runtime::run_command_async;
use crate::shared::request_context::InvokeMeta;
use rtool_app::LocaleApplicationService;
use rtool_contracts::models::{ClipboardSyncPayload, SettingsDto, SettingsUpdateInputDto};
use rtool_contracts::{AppError, InvokeError};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

const SETTINGS_SYNC_EVENT: &str = "rtool://settings/sync";

const SETTINGS_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "settings",
    "设置命令参数无效",
    "设置命令返回序列化失败",
    "未知设置命令",
);

fn normalize_screenshot_shortcut_update(
    input: &mut SettingsUpdateInputDto,
) -> Result<(), AppError> {
    let Some(screenshot) = input.screenshot.as_mut() else {
        return Ok(());
    };
    let Some(shortcut) = screenshot.shortcut.take() else {
        return Ok(());
    };
    screenshot.shortcut = Some(
        crate::platform::native_ui::shortcuts::normalize_screenshot_shortcut(shortcut.as_str())?,
    );
    Ok(())
}

async fn app_get_settings(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<SettingsDto, InvokeError> {
    let settings_service = state.app_services.settings.clone();
    run_command_async(
        "app_get_settings",
        request_id,
        window_label,
        move || async move { settings_service.load_or_init().await },
    )
    .await
}

async fn app_update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    input: SettingsUpdateInputDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<SettingsDto, InvokeError> {
    run_command_async(
        "app_update_settings",
        request_id,
        window_label,
        move || async move {
            let mut normalized_input = input;
            normalize_screenshot_shortcut_update(&mut normalized_input)?;

            let previous_locale = state.locale_snapshot();
            let previous_settings = state.app_services.settings.load_or_init().await?;
            let previous_screenshot_shortcut = previous_settings.screenshot.shortcut.clone();
            let requested_screenshot_shortcut = normalized_input
                .screenshot
                .as_ref()
                .and_then(|value| value.shortcut.as_ref())
                .cloned();

            let mut rebound_shortcut: Option<(String, String)> = None;
            if let Some(next_shortcut) = requested_screenshot_shortcut
                && next_shortcut != previous_screenshot_shortcut
            {
                crate::platform::native_ui::shortcuts::rebind_screenshot_shortcut(
                    &app,
                    previous_screenshot_shortcut.as_str(),
                    next_shortcut.as_str(),
                )?;
                rebound_shortcut = Some((previous_screenshot_shortcut.clone(), next_shortcut));
            }

            let settings = match state.app_services.settings.update(normalized_input).await {
                Ok(value) => value,
                Err(error) => {
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
                    return Err(error);
                }
            };
            crate::platform::native_ui::apply_window_chrome(
                &app,
                settings.theme.transparent_window_background,
            );

            if previous_locale.preference != settings.locale.preference {
                let resolved =
                    LocaleApplicationService.resolve(settings.locale.preference.as_str());
                state.update_locale(settings.locale.preference.clone(), resolved.clone());
                crate::platform::native_ui::apply_locale_to_native_ui(&app, &resolved);
            }

            let clipboard_update = state
                .app_services
                .clipboard
                .apply_settings(&settings.clipboard)
                .await?;
            if !clipboard_update.removed_ids.is_empty() {
                emit_clipboard_sync(
                    &app,
                    ClipboardSyncPayload {
                        upsert: Vec::new(),
                        removed_ids: clipboard_update.removed_ids,
                        clear_all: false,
                        reason: Some("settings_clipboard_prune".to_string()),
                    },
                );
            }

            if let Err(error) = app.emit(SETTINGS_SYNC_EVENT, settings.clone()) {
                tracing::warn!(
                    event = "settings_sync_emit_failed",
                    detail = %error
                );
            }

            Ok::<SettingsDto, AppError>(settings)
        },
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateSettingsPayload {
    input: SettingsUpdateInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum SettingsRequest {
    Get,
    Update(UpdateSettingsPayload),
}

pub(crate) async fn handle_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SettingsRequest,
    meta: Option<InvokeMeta>,
) -> Result<Value, InvokeError> {
    let (request_id, window_label) = meta.unwrap_or_default().split();

    match request {
        SettingsRequest::Get => SETTINGS_COMMAND_CONTEXT.serialize(
            "get",
            app_get_settings(state, request_id, window_label).await?,
        ),
        SettingsRequest::Update(payload) => SETTINGS_COMMAND_CONTEXT.serialize(
            "update",
            app_update_settings(app, state, payload.input, request_id, window_label).await?,
        ),
    }
}

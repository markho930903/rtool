use crate::app::state::AppState;
use crate::command_runtime::{run_command_async, run_command_sync};
use crate::features::clipboard::events::emit_clipboard_sync;
use protocol::models::{ClipboardSyncPayload, UserSettingsDto, UserSettingsUpdateInputDto};
use protocol::{AppError, InvokeError};
use rtool_i18n::i18n::resolve_locale;
use tauri::{AppHandle, State};

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

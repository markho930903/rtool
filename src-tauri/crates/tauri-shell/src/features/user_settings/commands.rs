use crate::app::state::AppState;
use crate::command_runtime::{run_command_async, run_command_sync};
use crate::features::clipboard::events::emit_clipboard_sync;
use crate::features::user_settings::store::{load_or_init_user_settings, update_user_settings};
use app_core::i18n::resolve_locale;
use app_core::models::{ClipboardSyncPayload, UserSettingsDto, UserSettingsUpdateInputDto};
use app_core::{AppError, InvokeError};
use app_launcher_app::launcher::service::invalidate_launcher_cache;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn app_get_user_settings(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<UserSettingsDto, InvokeError> {
    run_command_sync(
        "app_get_user_settings",
        request_id,
        window_label,
        load_or_init_user_settings,
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
            let settings = update_user_settings(input)?;

            if previous_locale.preference != settings.locale.preference {
                let resolved = resolve_locale(settings.locale.preference.as_str());
                state.update_locale(settings.locale.preference.clone(), resolved.clone());
                invalidate_launcher_cache();
                crate::apply_locale_to_native_ui(&app, &resolved);
            }

            let clipboard_update = state
                .clipboard_service
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

use crate::app::state::AppState;
use crate::command_runtime::{run_command_async, run_command_sync};
use app_core::i18n::{LocaleStateDto, normalize_locale_preference, resolve_locale};
use app_core::{AppError, InvokeError};
use tauri::{AppHandle, State};

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
            state.app_services.launcher.invalidate_cache();
            crate::apply_locale_to_native_ui(&app, &resolved);
            Ok::<LocaleStateDto, AppError>(next)
        },
    )
    .await
}

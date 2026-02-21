use super::run_command_sync;
use crate::app::launcher_service::invalidate_launcher_cache;
use crate::app::state::AppState;
use crate::core::i18n::{
    APP_LOCALE_PREFERENCE_KEY, LocaleStateDto, normalize_locale_preference, resolve_locale,
};
use crate::core::{AppError, InvokeError};
use crate::infrastructure::db;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn app_get_locale(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleStateDto, InvokeError> {
    run_command_sync("app_get_locale", request_id, window_label, move || {
        Ok::<_, InvokeError>(state.locale_snapshot())
    })
}

#[tauri::command]
pub fn app_set_locale(
    app: AppHandle,
    state: State<'_, AppState>,
    preference: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleStateDto, InvokeError> {
    run_command_sync("app_set_locale", request_id, window_label, move || {
        let canonical_preference = normalize_locale_preference(&preference).ok_or_else(|| {
            AppError::new("invalid_locale_preference", "语言偏好无效")
                .with_context("preference", preference.clone())
        })?;

        db::set_app_setting(
            &state.db_pool,
            APP_LOCALE_PREFERENCE_KEY,
            canonical_preference.as_str(),
        )?;
        let resolved = resolve_locale(&canonical_preference);
        let next = state.update_locale(canonical_preference, resolved.clone());
        invalidate_launcher_cache();
        crate::apply_locale_to_native_ui(&app, &resolved);
        Ok::<LocaleStateDto, AppError>(next)
    })
}

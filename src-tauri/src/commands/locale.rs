use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::app::launcher_service::invalidate_launcher_cache;
use crate::app::state::AppState;
use crate::core::i18n::{
    APP_LOCALE_PREFERENCE_KEY, LocaleStateDto, normalize_locale_preference, resolve_locale,
};
use crate::core::AppError;
use crate::infrastructure::db;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn app_get_locale(
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleStateDto, AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("app_get_locale", &request_id, window_label.as_deref());
    let result = Ok(state.locale_snapshot());
    command_end_ok("app_get_locale", &request_id, started_at);
    result
}

#[tauri::command]
pub fn app_set_locale(
    app: AppHandle,
    state: State<'_, AppState>,
    preference: String,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleStateDto, AppError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("app_set_locale", &request_id, window_label.as_deref());

    let result = (|| -> Result<LocaleStateDto, AppError> {
        let canonical_preference = normalize_locale_preference(&preference).ok_or_else(|| {
            AppError::new("invalid_locale_preference", "语言偏好无效")
                .with_detail(preference.clone())
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
        Ok(next)
    })();

    match &result {
        Ok(_) => command_end_ok("app_set_locale", &request_id, started_at),
        Err(error) => command_end_error("app_set_locale", &request_id, started_at, error),
    }

    result
}

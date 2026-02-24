use crate::app::state::AppState;
use crate::command_runtime::{run_command_async, run_command_sync};
use app_core::i18n::{
    APP_LOCALE_PREFERENCE_KEY, LocaleStateDto, normalize_locale_preference, resolve_locale,
};
use app_core::{AppError, InvokeError};
use app_infra::db;
use app_launcher_app::launcher::service::invalidate_launcher_cache;
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

            db::set_app_setting(
                &state.db_conn,
                APP_LOCALE_PREFERENCE_KEY,
                canonical_preference.as_str(),
            )
            .await?;
            let resolved = resolve_locale(&canonical_preference);
            let next = state.update_locale(canonical_preference, resolved.clone());
            invalidate_launcher_cache();
            crate::apply_locale_to_native_ui(&app, &resolved);
            Ok::<LocaleStateDto, AppError>(next)
        },
    )
    .await
}

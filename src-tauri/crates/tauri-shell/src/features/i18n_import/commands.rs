use super::run_command_sync;
use crate::app::state::AppState;
use app_core::i18n::DEFAULT_RESOLVED_LOCALE;
use app_core::i18n_catalog::{
    ImportLocaleResult, LocaleCatalogList, ReloadLocalesResult, import_locale_file, list_locales,
    reload_overlays,
};
use app_core::{AppError, InvokeError};
use tauri::{AppHandle, State};

fn map_i18n_error(error: anyhow::Error) -> AppError {
    AppError::from_anyhow(error)
        .with_code("i18n_error", "多语言资源操作失败")
        .with_context("domain", "i18n_catalog")
}

#[tauri::command]
pub fn app_list_locales(
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<LocaleCatalogList, InvokeError> {
    run_command_sync("app_list_locales", request_id, window_label, move || {
        list_locales().map_err(map_i18n_error)
    })
}

#[tauri::command]
pub fn app_reload_locales(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ReloadLocalesResult, InvokeError> {
    run_command_sync("app_reload_locales", request_id, window_label, move || {
        let output = reload_overlays().map_err(map_i18n_error)?;
        state.app_services.launcher.invalidate_cache();
        let resolved_locale = state.resolved_locale();
        crate::apply_locale_to_native_ui(&app, &resolved_locale);
        Ok::<ReloadLocalesResult, AppError>(output)
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn app_import_locale_file(
    app: AppHandle,
    state: State<'_, AppState>,
    locale: String,
    namespace: String,
    content: String,
    replace: Option<bool>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ImportLocaleResult, InvokeError> {
    run_command_sync(
        "app_import_locale_file",
        request_id,
        window_label,
        move || {
            let output = import_locale_file(
                &locale,
                &namespace,
                &content,
                replace.unwrap_or(true),
                DEFAULT_RESOLVED_LOCALE,
            )
            .map_err(map_i18n_error)?;

            state.app_services.launcher.invalidate_cache();
            let resolved_locale = state.resolved_locale();
            crate::apply_locale_to_native_ui(&app, &resolved_locale);
            Ok::<ImportLocaleResult, AppError>(output)
        },
    )
}

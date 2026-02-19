use super::{command_end_error, command_end_ok, command_start, normalize_request_id};
use crate::app::launcher_service::invalidate_launcher_cache;
use crate::app::state::AppState;
use crate::core::i18n::DEFAULT_RESOLVED_LOCALE;
use crate::core::i18n_catalog::{
    ImportLocaleResult, LocaleCatalogList, ReloadLocalesResult, import_locale_file, list_locales,
    reload_overlays,
};
use crate::core::{AppError, InvokeError};
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
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("app_list_locales", &request_id, window_label.as_deref());

    let result = list_locales().map_err(map_i18n_error);
    match &result {
        Ok(_) => command_end_ok("app_list_locales", &request_id, started_at),
        Err(error) => command_end_error("app_list_locales", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
pub fn app_reload_locales(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<ReloadLocalesResult, InvokeError> {
    let request_id = normalize_request_id(request_id);
    let started_at = command_start("app_reload_locales", &request_id, window_label.as_deref());

    let result = (|| -> Result<ReloadLocalesResult, AppError> {
        let output = reload_overlays().map_err(map_i18n_error)?;
        invalidate_launcher_cache();
        let resolved_locale = state.resolved_locale();
        crate::apply_locale_to_native_ui(&app, &resolved_locale);
        Ok(output)
    })();

    match &result {
        Ok(_) => command_end_ok("app_reload_locales", &request_id, started_at),
        Err(error) => command_end_error("app_reload_locales", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
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
    let request_id = normalize_request_id(request_id);
    let started_at = command_start(
        "app_import_locale_file",
        &request_id,
        window_label.as_deref(),
    );

    let result = (|| -> Result<ImportLocaleResult, AppError> {
        let output = import_locale_file(
            &locale,
            &namespace,
            &content,
            replace.unwrap_or(true),
            DEFAULT_RESOLVED_LOCALE,
        )
        .map_err(map_i18n_error)?;

        invalidate_launcher_cache();
        let resolved_locale = state.resolved_locale();
        crate::apply_locale_to_native_ui(&app, &resolved_locale);
        Ok(output)
    })();

    match &result {
        Ok(_) => command_end_ok("app_import_locale_file", &request_id, started_at),
        Err(error) => command_end_error("app_import_locale_file", &request_id, started_at, error),
    }
    result.map_err(Into::into)
}

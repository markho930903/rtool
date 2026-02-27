use super::run_command_sync;
use crate::app::state::AppState;
use crate::features::command_payload::{
    CommandPayloadContext, CommandRequestDto,
};
use protocol::{AppError, InvokeError};
use rtool_i18n::i18n::DEFAULT_RESOLVED_LOCALE;
use rtool_i18n::i18n_catalog::{
    ImportLocaleResult, LocaleCatalogList, ReloadLocalesResult, import_locale_file, list_locales,
    reload_overlays,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

const I18N_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "i18n",
    "多语言命令参数无效",
    "多语言命令返回序列化失败",
    "未知多语言命令",
);

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

            let resolved_locale = state.resolved_locale();
            crate::apply_locale_to_native_ui(&app, &resolved_locale);
            Ok::<ImportLocaleResult, AppError>(output)
        },
    )
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct I18nImportPayload {
    locale: String,
    namespace: String,
    content: String,
    replace: Option<bool>,
}

#[tauri::command]
pub fn i18n_import_handle(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CommandRequestDto,
    request_id: Option<String>,
    window_label: Option<String>,
) -> Result<Value, InvokeError> {
    match request.kind.as_str() {
        "list_locales" => I18N_COMMAND_CONTEXT.serialize(
            "list_locales",
            app_list_locales(request_id, window_label)?,
        ),
        "reload_locales" => I18N_COMMAND_CONTEXT.serialize(
            "reload_locales",
            app_reload_locales(app, state, request_id, window_label)?,
        ),
        "import_locale_file" => {
            let payload: I18nImportPayload =
                I18N_COMMAND_CONTEXT.parse("import_locale_file", request.payload)?;
            I18N_COMMAND_CONTEXT.serialize(
                "import_locale_file",
                app_import_locale_file(
                    app,
                    state,
                    payload.locale,
                    payload.namespace,
                    payload.content,
                    payload.replace,
                    request_id,
                    window_label,
                )?,
            )
        }
        _ => Err(I18N_COMMAND_CONTEXT.unknown(request.kind)),
    }
}

pub(crate) fn with_invoke_handler(
    builder: tauri::Builder<tauri::Wry>,
) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        crate::features::app_manager::commands::app_manager_handle,
        crate::features::clipboard::commands::clipboard_handle,
        crate::features::dashboard::commands::dashboard_handle,
        crate::features::launcher::commands::launcher_handle,
        crate::features::locale::commands::locale_handle,
        crate::features::logging::commands::logging_handle,
        crate::features::transfer::commands::transfer_handle,
        crate::features::user_settings::commands::settings_handle,
    ])
}

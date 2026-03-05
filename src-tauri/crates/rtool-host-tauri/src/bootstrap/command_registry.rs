pub(crate) fn with_invoke_handler(
    builder: tauri::Builder<tauri::Wry>,
) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        crate::features::commands::rt_app_manager,
        crate::features::commands::rt_clipboard,
        crate::features::commands::rt_launcher,
        crate::features::commands::rt_locale,
        crate::features::commands::rt_logging,
        crate::features::commands::rt_screenshot,
        crate::features::commands::rt_settings,
    ])
}

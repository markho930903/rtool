#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app_tauri_shell::run(tauri::generate_context!());
}

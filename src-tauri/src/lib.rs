#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    rtool_host_tauri::run(tauri::generate_context!());
}

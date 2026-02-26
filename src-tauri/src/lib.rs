#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    shell::run(tauri::generate_context!());
}

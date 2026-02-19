use active_win_pos_rs::get_active_window;
use image::ImageReader;
use std::error::Error;
use std::io::Cursor;
use std::path::Path;

pub(super) fn current_source_app() -> Option<String> {
    let active_window = get_active_window().ok()?;
    let app_name = active_window.app_name.trim();
    if app_name.is_empty() {
        return None;
    }

    Some(app_name.to_string())
}

pub(super) fn build_image_signature(width: usize, height: usize, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(width as u64).to_le_bytes());
    hasher.update(&(height as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

pub(super) fn read_image_dimensions_from_header(bytes: &[u8]) -> Option<(u32, u32)> {
    let cursor = Cursor::new(bytes);
    let reader = ImageReader::new(cursor).with_guessed_format().ok()?;
    reader.into_dimensions().ok()
}

pub(super) fn save_clipboard_image_preview(
    preview_dir: &Path,
    signature: &str,
    bytes: &[u8],
) -> Result<String, Box<dyn Error>> {
    std::fs::create_dir_all(preview_dir)?;

    let preview_path = preview_dir.join(format!("{}.png", signature));
    std::fs::write(&preview_path, bytes)?;

    Ok(preview_path.to_string_lossy().to_string())
}

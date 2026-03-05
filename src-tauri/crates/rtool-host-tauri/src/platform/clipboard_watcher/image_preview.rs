use image::ImageReader;
use std::error::Error;
use std::io::Cursor;
use std::path::Path;
use xcap::Window;

pub(super) fn current_source_app() -> Option<String> {
    let windows = Window::all().ok()?;
    for window in windows {
        let Ok(is_focused) = window.is_focused() else {
            continue;
        };
        if !is_focused {
            continue;
        }

        let Ok(app_name) = window.app_name() else {
            continue;
        };
        let app_name = app_name.trim();
        if app_name.is_empty() {
            continue;
        }

        return Some(app_name.to_string());
    }

    None
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

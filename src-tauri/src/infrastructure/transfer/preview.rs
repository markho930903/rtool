use std::io::{BufRead, BufReader};
use std::path::Path;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "json", "toml", "yaml", "yml", "csv", "rs", "ts", "tsx", "js", "jsx", "html",
    "css", "xml", "log",
];

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
}

pub fn guess_mime(path: &Path) -> Option<String> {
    let ext = extension(path)?;
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" | "md" | "log" => "text/plain",
        "json" => "application/json",
        "csv" => "text/csv",
        "toml" => "application/toml",
        "yaml" | "yml" => "application/yaml",
        "xml" => "application/xml",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "ts" | "tsx" => "application/typescript",
        _ => "application/octet-stream",
    };
    Some(mime.to_string())
}

pub fn build_preview(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let mime = guess_mime(path);
    let ext = extension(path);

    if let Some(ext_value) = ext.as_deref() {
        if IMAGE_EXTENSIONS.contains(&ext_value) {
            return (
                mime,
                Some("image".to_string()),
                Some(path.to_string_lossy().to_string()),
            );
        }

        if ext_value == "pdf" {
            return (
                mime,
                Some("pdf".to_string()),
                Some(
                    path.file_name()
                        .map(|v| v.to_string_lossy().to_string())
                        .unwrap_or_default(),
                ),
            );
        }

        if TEXT_EXTENSIONS.contains(&ext_value) {
            let preview_text = read_first_lines(path, 5);
            return (mime, Some("text".to_string()), preview_text);
        }
    }

    (mime, None, None)
}

fn read_first_lines(path: &Path, limit: usize) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();

    for line in reader.lines().take(limit) {
        let Ok(value) = line else {
            continue;
        };
        lines.push(value);
    }

    if lines.is_empty() {
        return None;
    }

    Some(lines.join("\n"))
}

use crate::models::ClipboardItemDto;
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or_default()
}

fn hash_to_u64(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn normalize_text_for_key(text: &str) -> String {
    text.trim().to_string()
}

fn extract_image_signature_from_path(path: &str) -> Option<String> {
    let stem = Path::new(path).file_stem()?;
    let value = stem.to_string_lossy().trim().to_string();
    if value.is_empty() {
        return None;
    }
    Some(value)
}

fn extract_image_signature_from_id(id: &str) -> Option<String> {
    let (_, hash) = id.rsplit_once('-')?;
    let value = hash.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn decode_percent_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut index = 0;
    let mut decoded = Vec::with_capacity(bytes.len());

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = &value[index + 1..index + 3];
            if let Ok(parsed) = u8::from_str_radix(hex, 16) {
                decoded.push(parsed);
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&decoded).to_string()
}

fn normalize_path_candidate(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }

    let candidate = if let Some(file_uri) = trimmed.strip_prefix("file://") {
        #[cfg(target_os = "windows")]
        let normalized = file_uri.strip_prefix('/').unwrap_or(file_uri);

        #[cfg(not(target_os = "windows"))]
        let normalized = file_uri;

        decode_percent_component(normalized)
    } else {
        trimmed.to_string()
    };

    if candidate.is_empty() {
        return None;
    }

    let path = Path::new(&candidate);
    if path.exists() {
        return Some(path.to_string_lossy().to_string());
    }

    None
}

pub fn parse_file_paths_from_text(text: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }

    let mut paths = Vec::with_capacity(lines.len());
    for line in lines {
        let path = normalize_path_candidate(line)?;
        paths.push(path);
    }

    Some(paths)
}

pub fn classify_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "text".to_string();
    }

    if parse_file_paths_from_text(trimmed).is_some() {
        return "file".to_string();
    }

    if Regex::new(r"^https?://")
        .ok()
        .is_some_and(|re| re.is_match(trimmed))
    {
        return "link".to_string();
    }

    if Regex::new(r"^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$")
        .ok()
        .is_some_and(|re| re.is_match(trimmed))
    {
        return "color".to_string();
    }

    if trimmed.contains("fn ")
        || trimmed.contains("const ")
        || trimmed.contains("let ")
        || trimmed.contains("class ")
        || trimmed.contains("import ")
    {
        return "code".to_string();
    }

    "text".to_string()
}

pub fn derive_content_key(
    item_type: &str,
    plain_text: &str,
    preview_path: Option<&str>,
    preview_data_url: Option<&str>,
    id: Option<&str>,
) -> String {
    if item_type == "image" {
        if let Some(signature) = preview_path.and_then(extract_image_signature_from_path) {
            return format!("image:{signature}");
        }

        if let Some(signature) = id.and_then(extract_image_signature_from_id) {
            return format!("image:{signature}");
        }

        if let Some(data_url) = preview_data_url {
            return format!("image:data:{}", hash_to_u64(data_url));
        }
    }

    let normalized_text = normalize_text_for_key(plain_text);
    format!("{item_type}:{}", hash_to_u64(normalized_text))
}

pub fn build_clipboard_item(text: String, source_app: Option<String>) -> ClipboardItemDto {
    let created_at = now_millis();
    let item_type = classify_text(&text);
    let content_key = derive_content_key(&item_type, &text, None, None, None);
    let key_hash = hash_to_u64(&content_key);

    let id = format!("clipboard-{}-{}", created_at, key_hash);

    ClipboardItemDto {
        id,
        content_key,
        item_type,
        plain_text: text,
        source_app,
        preview_path: None,
        preview_data_url: None,
        created_at,
        pinned: false,
    }
}

pub fn build_image_clipboard_item(
    width: usize,
    height: usize,
    signature: &str,
    preview_path: Option<String>,
    preview_data_url: Option<String>,
    source_app: Option<String>,
) -> ClipboardItemDto {
    let created_at = now_millis();
    let signature_hash = hash_to_u64(signature);
    let plain_text = format!("[图片] {} x {}", width, height);
    let content_key = format!("image:{signature}");

    ClipboardItemDto {
        id: format!("clipboard-image-{}-{}", created_at, signature_hash),
        content_key,
        item_type: "image".to_string(),
        plain_text,
        source_app,
        preview_path,
        preview_data_url,
        created_at,
        pinned: false,
    }
}

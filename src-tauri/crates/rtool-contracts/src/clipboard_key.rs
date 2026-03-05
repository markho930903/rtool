use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

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

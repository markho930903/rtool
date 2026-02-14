use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const APP_ICON_TTL: Duration = Duration::from_secs(60 * 60 * 24);
const FILE_ICON_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 30);

const FALLBACK_APP_ICON: &str = "i-noto:desktop-computer";
const FALLBACK_FILE_ICON: &str = "i-noto:page-facing-up";

#[derive(Debug, Clone)]
pub struct IconPayload {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiskIconEntry {
    updated_at: u64,
    icon_kind: String,
    icon_value: String,
}

fn icon_memory_cache() -> &'static Mutex<HashMap<String, DiskIconEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, DiskIconEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn resolve_builtin_icon(icon: &str) -> IconPayload {
    IconPayload {
        kind: "iconify".to_string(),
        value: icon.to_string(),
    }
}

pub fn resolve_application_icon(app: &AppHandle, app_path: &Path) -> IconPayload {
    let key = format!("app:{}", app_path.to_string_lossy());

    if let Some(payload) = read_cached_icon(app, &key, APP_ICON_TTL) {
        return payload;
    }

    let generated = try_extract_application_icon(app, app_path).unwrap_or_else(|| IconPayload {
        kind: "iconify".to_string(),
        value: FALLBACK_APP_ICON.to_string(),
    });

    write_cached_icon(app, &key, &generated);
    generated
}

pub fn resolve_file_type_icon(app: &AppHandle, file_path: &Path) -> IconPayload {
    let ext = file_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_else(|| "_none".to_string());

    let key = format!("file-ext:{ext}");
    if let Some(payload) = read_cached_icon(app, &key, FILE_ICON_TTL) {
        return payload;
    }

    let icon = IconPayload {
        kind: "iconify".to_string(),
        value: file_extension_icon(&ext).to_string(),
    };

    write_cached_icon(app, &key, &icon);
    icon
}

fn read_cached_icon(app: &AppHandle, key: &str, ttl: Duration) -> Option<IconPayload> {
    let now = current_timestamp();

    if let Ok(cache) = icon_memory_cache().lock()
        && let Some(entry) = cache.get(key)
        && now.saturating_sub(entry.updated_at) <= ttl.as_secs()
    {
        return Some(IconPayload {
            kind: entry.icon_kind.clone(),
            value: entry.icon_value.clone(),
        });
    }

    let cache_path = icon_cache_file_path(app, key);
    let content = fs::read_to_string(cache_path).ok()?;
    let entry = serde_json::from_str::<DiskIconEntry>(&content).ok()?;
    if now.saturating_sub(entry.updated_at) > ttl.as_secs() {
        return None;
    }

    if let Ok(mut cache) = icon_memory_cache().lock() {
        cache.insert(key.to_string(), entry.clone());
    }

    Some(IconPayload {
        kind: entry.icon_kind,
        value: entry.icon_value,
    })
}

fn write_cached_icon(app: &AppHandle, key: &str, payload: &IconPayload) {
    let entry = DiskIconEntry {
        updated_at: current_timestamp(),
        icon_kind: payload.kind.clone(),
        icon_value: payload.value.clone(),
    };

    if let Ok(mut cache) = icon_memory_cache().lock() {
        cache.insert(key.to_string(), entry.clone());
    }

    let cache_path = icon_cache_file_path(app, key);
    if let Some(parent) = cache_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            tracing::debug!(
                event = "icon_cache_dir_create_failed",
                cache_key = key,
                error = error.to_string()
            );
            return;
        }
    }

    if let Ok(content) = serde_json::to_string(&entry) {
        if let Err(error) = fs::write(&cache_path, content) {
            tracing::debug!(
                event = "icon_cache_write_failed",
                cache_key = key,
                cache_path = %cache_path.to_string_lossy(),
                error = error.to_string()
            );
        }
    } else {
        tracing::debug!(event = "icon_cache_serialize_failed", cache_key = key);
    }
}

fn icon_cache_file_path(app: &AppHandle, key: &str) -> PathBuf {
    let cache_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("launcher_icon_cache");
    cache_dir.join(format!("{}.json", stable_hash(key)))
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn stable_hash(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(target_os = "macos")]
fn try_extract_application_icon(app: &AppHandle, app_path: &Path) -> Option<IconPayload> {
    let extension = app_path.extension()?.to_str()?.to_ascii_lowercase();
    if extension != "app" {
        return None;
    }

    let resources = app_path.join("Contents").join("Resources");
    let icon_file = pick_icns_file(&resources)?;

    let output_png = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("launcher_icon_cache")
        .join(format!("{}.png", stable_hash(&icon_file.to_string_lossy())));

    if !output_png.exists() {
        if let Some(parent) = output_png.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                tracing::debug!(
                    event = "icon_png_cache_dir_create_failed",
                    app_path = %app_path.to_string_lossy(),
                    error = error.to_string()
                );
                return None;
            }
        }

        let status = std::process::Command::new("sips")
            .arg("-s")
            .arg("format")
            .arg("png")
            .arg(&icon_file)
            .arg("--resampleHeightWidth")
            .arg("64")
            .arg("64")
            .arg("--out")
            .arg(&output_png)
            .status()
            .ok()?;

        if !status.success() {
            return None;
        }
    }

    let bytes = fs::read(output_png).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(IconPayload {
        kind: "raster".to_string(),
        value: format!("data:image/png;base64,{encoded}"),
    })
}

#[cfg(target_os = "macos")]
fn pick_icns_file(resources_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(resources_dir).ok()?;
    let mut candidates = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let extension = path.extension().and_then(|value| value.to_str());
        if !matches!(extension, Some(ext) if ext.eq_ignore_ascii_case("icns")) {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();

        let weight = if name.contains("appicon") {
            4
        } else if name.contains("icon") {
            3
        } else {
            1
        };

        candidates.push((weight, path));
    }

    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    candidates.into_iter().next().map(|(_, path)| path)
}

#[cfg(not(target_os = "macos"))]
fn try_extract_application_icon(_app: &AppHandle, _app_path: &Path) -> Option<IconPayload> {
    None
}

fn file_extension_icon(ext: &str) -> &'static str {
    match ext {
        "pdf" => "i-noto:page-facing-up",
        "doc" | "docx" | "rtf" => "i-noto:memo",
        "xls" | "xlsx" | "csv" => "i-noto:bar-chart",
        "ppt" | "pptx" => "i-noto:rolled-up-newspaper",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" => "i-noto:framed-picture",
        "mp4" | "mov" | "mkv" | "avi" | "webm" => "i-noto:film-projector",
        "mp3" | "wav" | "flac" | "aac" | "ogg" => "i-noto:musical-notes",
        "zip" | "rar" | "7z" | "tar" | "gz" => "i-noto:file-folder",
        "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "md" | "txt" => "i-noto:scroll",
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "c" | "cpp" | "h" | "hpp" => {
            "i-noto:desktop-computer"
        }
        "sql" => "i-noto:floppy-disk",
        _ => FALLBACK_FILE_ICON,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_map_common_file_extensions() {
        assert_eq!(file_extension_icon("pdf"), "i-noto:page-facing-up");
        assert_eq!(file_extension_icon("rs"), "i-noto:desktop-computer");
        assert_eq!(file_extension_icon("zip"), "i-noto:file-folder");
        assert_eq!(file_extension_icon("unknown"), FALLBACK_FILE_ICON);
    }
}

use crate::host::LauncherHost;
use base64::Engine as _;
#[cfg(target_os = "macos")]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const APP_ICON_TTL: Duration = Duration::from_secs(60 * 60 * 24);
const APP_ICON_FALLBACK_TTL: Duration = Duration::from_secs(60 * 10);
const FILE_ICON_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 30);
const FALLBACK_APP_ICON: &str = "i-noto:desktop-computer";
const FALLBACK_FILE_ICON: &str = "i-noto:page-facing-up";

#[derive(Debug, Clone)]
pub struct IconPayload {
    pub kind: String,
    pub value: String,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
struct MacIconSource {
    icon_path: PathBuf,
    signature: String,
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

fn ensure_icon_cache_schema_initialized(app: &dyn LauncherHost) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let cache_dir = app
            .app_data_dir()
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("launcher_icon_cache");
        if cache_dir.exists() {
            let _ = fs::remove_dir_all(&cache_dir);
        }
        let _ = fs::create_dir_all(&cache_dir);
    });
}

pub fn resolve_builtin_icon(icon: &str) -> IconPayload {
    IconPayload {
        kind: "iconify".to_string(),
        value: icon.to_string(),
    }
}

pub fn resolve_application_icon(app: &dyn LauncherHost, app_path: &Path) -> IconPayload {
    ensure_icon_cache_schema_initialized(app);
    let app_path_key = app_path.to_string_lossy();

    #[cfg(target_os = "macos")]
    {
        if let Some(source) = resolve_macos_icon_source(app_path) {
            let key = format!("app:{app_path_key}:{}", source.signature);
            if let Some(payload) = read_cached_icon(app, &key, APP_ICON_TTL) {
                return payload;
            }
            if let Some(payload) = render_macos_icon_payload(app, &source) {
                write_cached_icon(app, &key, &payload);
                return payload;
            }
            tracing::debug!(
                event = "app_icon_extract_failed",
                app_path = %app_path.to_string_lossy(),
                icon_path = %source.icon_path.to_string_lossy()
            );
        }
    }

    let fallback_key = format!("app:{app_path_key}:fallback");
    if let Some(payload) = read_cached_icon(app, &fallback_key, APP_ICON_FALLBACK_TTL) {
        return payload;
    }

    let generated = IconPayload {
        kind: "iconify".to_string(),
        value: FALLBACK_APP_ICON.to_string(),
    };

    write_cached_icon(app, &fallback_key, &generated);
    generated
}

pub fn resolve_file_type_icon(app: &dyn LauncherHost, file_path: &Path) -> IconPayload {
    ensure_icon_cache_schema_initialized(app);
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

fn read_cached_icon(app: &dyn LauncherHost, key: &str, ttl: Duration) -> Option<IconPayload> {
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

fn write_cached_icon(app: &dyn LauncherHost, key: &str, payload: &IconPayload) {
    let entry = DiskIconEntry {
        updated_at: current_timestamp(),
        icon_kind: payload.kind.clone(),
        icon_value: payload.value.clone(),
    };

    if let Ok(mut cache) = icon_memory_cache().lock() {
        cache.insert(key.to_string(), entry.clone());
    }

    let cache_path = icon_cache_file_path(app, key);
    if let Some(parent) = cache_path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        tracing::debug!(
            event = "icon_cache_dir_create_failed",
            cache_key = key,
            error = error.to_string()
        );
        return;
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

fn icon_cache_file_path(app: &dyn LauncherHost, key: &str) -> PathBuf {
    let cache_dir = app
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
fn resolve_macos_icon_source(app_path: &Path) -> Option<MacIconSource> {
    let extension = app_path.extension()?.to_str()?.to_ascii_lowercase();
    if extension != "app" {
        return None;
    }

    let resources = app_path.join("Contents").join("Resources");
    if !resources.exists() {
        return None;
    }

    let preferred_names = macos_preferred_icns_names(app_path);
    let icon_path = match_preferred_icns(&resources, preferred_names.as_slice())
        .or_else(|| pick_icns_file(&resources, app_path))?;
    let signature = signature_for_file(icon_path.as_path());

    Some(MacIconSource {
        icon_path,
        signature,
    })
}

#[cfg(target_os = "macos")]
fn render_macos_icon_payload(
    app: &dyn LauncherHost,
    source: &MacIconSource,
) -> Option<IconPayload> {
    let output_png = app
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("launcher_icon_cache")
        .join(format!("{}.png", stable_hash(source.signature.as_str())));

    if !output_png.exists() {
        if let Some(parent) = output_png.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            tracing::debug!(
                event = "icon_png_cache_dir_create_failed",
                icon_path = %source.icon_path.to_string_lossy(),
                error = error.to_string()
            );
            return None;
        }

        let status = std::process::Command::new("sips")
            .arg("-s")
            .arg("format")
            .arg("png")
            .arg(&source.icon_path)
            .arg("--resampleHeightWidth")
            .arg("64")
            .arg("64")
            .arg("--out")
            .arg(&output_png)
            .status()
            .ok()?;

        if !status.success() {
            tracing::debug!(
                event = "icon_sips_convert_failed",
                icon_path = %source.icon_path.to_string_lossy(),
                status = format!("{status}")
            );
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
fn signature_for_file(path: &Path) -> String {
    match fs::metadata(path) {
        Ok(meta) => {
            let modified_secs = meta
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            let content = format!("{}|{}|{modified_secs}", path.to_string_lossy(), meta.len());
            stable_hash(content.as_str())
        }
        Err(_) => stable_hash(path.to_string_lossy().as_ref()),
    }
}

#[cfg(target_os = "macos")]
fn macos_preferred_icns_names(app_path: &Path) -> Vec<String> {
    let info_plist = app_path.join("Contents").join("Info.plist");
    let content = match fs::read_to_string(&info_plist) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };

    let mut candidates = Vec::new();
    candidates.extend(plist_string_values(content.as_str(), "CFBundleIconFile"));
    candidates.extend(plist_string_values(content.as_str(), "CFBundleIconName"));
    candidates.extend(plist_array_string_values(
        content.as_str(),
        "CFBundleIconFiles",
    ));

    let mut seen = HashSet::new();
    let mut names = Vec::new();
    for candidate in candidates {
        let Some(name) = normalize_icns_name(candidate.as_str()) else {
            continue;
        };
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }
    names
}

#[cfg(target_os = "macos")]
fn plist_string_values(content: &str, key: &str) -> Vec<String> {
    let pattern = format!(
        r"<key>{}</key>\s*<string>([^<]+)</string>",
        regex::escape(key)
    );
    let regex = match Regex::new(pattern.as_str()) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    regex
        .captures_iter(content)
        .filter_map(|captures| captures.get(1))
        .map(|capture| capture.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

#[cfg(target_os = "macos")]
fn plist_array_string_values(content: &str, key: &str) -> Vec<String> {
    let pattern = format!(
        r"(?s)<key>{}</key>\s*<array>(.*?)</array>",
        regex::escape(key)
    );
    let array_regex = match Regex::new(pattern.as_str()) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let string_regex = match Regex::new(r"<string>([^<]+)</string>") {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut values = Vec::new();
    for captures in array_regex.captures_iter(content) {
        let Some(body) = captures.get(1) else {
            continue;
        };
        for string_capture in string_regex.captures_iter(body.as_str()) {
            let Some(value) = string_capture.get(1) else {
                continue;
            };
            let normalized = value.as_str().trim();
            if !normalized.is_empty() {
                values.push(normalized.to_string());
            }
        }
    }
    values
}

#[cfg(target_os = "macos")]
fn normalize_icns_name(value: &str) -> Option<String> {
    let file_name = Path::new(value)
        .file_name()
        .and_then(|path| path.to_str())
        .map(str::trim)?;
    if file_name.is_empty() {
        return None;
    }
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".icns") {
        Some(lower)
    } else {
        Some(format!("{lower}.icns"))
    }
}

#[cfg(target_os = "macos")]
fn collect_icns_files(resources_dir: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(resources_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut files = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("icns"))
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        let left_name = left
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        let right_name = right
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        left_name.cmp(&right_name)
    });
    files
}

#[cfg(target_os = "macos")]
fn match_preferred_icns(resources_dir: &Path, preferred_names: &[String]) -> Option<PathBuf> {
    if preferred_names.is_empty() {
        return None;
    }
    let files = collect_icns_files(resources_dir);
    if files.is_empty() {
        return None;
    }
    for preferred in preferred_names {
        let preferred_lower = preferred.to_ascii_lowercase();
        if let Some(found) = files.iter().find(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.to_ascii_lowercase() == preferred_lower)
        }) {
            return Some(found.clone());
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn split_identifier_tokens(value: &str) -> Vec<String> {
    value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

#[cfg(target_os = "macos")]
fn is_language_token(token: &str) -> bool {
    matches!(
        token,
        "javascript"
            | "typescript"
            | "python"
            | "ruby"
            | "java"
            | "go"
            | "c"
            | "cpp"
            | "csharp"
            | "html"
            | "css"
            | "json"
            | "xml"
            | "yaml"
            | "shell"
            | "sql"
            | "markdown"
            | "default"
    )
}

#[cfg(target_os = "macos")]
fn icns_candidate_score(name: &str, app_tokens: &[String]) -> i32 {
    let stem = name.strip_suffix(".icns").unwrap_or(name);
    let tokens = split_identifier_tokens(stem);
    let mut score = 100;

    if stem.contains("appicon") {
        score += 320;
    } else if stem.contains("icon") {
        score += 130;
    }

    let shared = tokens
        .iter()
        .filter(|token| app_tokens.iter().any(|app| app == *token))
        .count();
    score += (shared as i32) * 90;

    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "document" | "default" | "template" | "file" | "doc"
        )
    }) {
        score -= 160;
    }
    if tokens.len() == 1
        && tokens
            .first()
            .is_some_and(|token| is_language_token(token.as_str()))
    {
        score -= 120;
    }
    if matches!(stem, "icon" | "app") {
        score -= 40;
    }

    score
}

#[cfg(target_os = "macos")]
fn pick_icns_file(resources_dir: &Path, app_path: &Path) -> Option<PathBuf> {
    let files = collect_icns_files(resources_dir);
    if files.is_empty() {
        return None;
    }
    let app_tokens = app_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(split_identifier_tokens)
        .unwrap_or_default();

    let mut candidates = files
        .into_iter()
        .map(|path| {
            let lower_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())
                .unwrap_or_default();
            let score = icns_candidate_score(lower_name.as_str(), app_tokens.as_slice());
            (score, lower_name, path)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates.into_iter().next().map(|(_, _, path)| path)
}

// NOTE: Keep i-noto:* mappings in sync with frontend uno.config.ts safelist.
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
        "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "plist" | "md" | "txt" => {
            "i-noto:scroll"
        }
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "c" | "cpp" | "h" | "hpp" => {
            "i-noto:desktop-computer"
        }
        "sql" => "i-noto:floppy-disk",
        _ => FALLBACK_FILE_ICON,
    }
}


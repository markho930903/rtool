use app_clipboard::service::{
    CLIPBOARD_MAX_ITEMS_MAX, CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX,
    CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN,
};
use app_core::i18n::{SYSTEM_LOCALE_PREFERENCE, normalize_locale_preference};
use app_core::models::{
    UserClipboardSettingsDto, UserClipboardSettingsUpdateInputDto, UserGlassProfileDto,
    UserGlassProfileUpdateInputDto, UserLayoutSettingsUpdateInputDto,
    UserLocaleSettingsUpdateInputDto, UserSettingsDto, UserSettingsUpdateInputDto,
    UserThemeGlassSettingsUpdateInputDto, UserThemeSettingsUpdateInputDto,
};
use app_core::{AppError, AppResult};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SETTINGS_DIR_NAME: &str = ".rtool";
const SETTINGS_FILE_NAME: &str = "setting.json";
const SETTINGS_FILE_LOCK_NAME: &str = "setting.json.lock";
const SETTINGS_FILE_LOCK_TIMEOUT_MS: u64 = 2_000;
const SETTINGS_FILE_LOCK_RETRY_MS: u64 = 25;
const SETTINGS_FILE_LOCK_STALE_SECS: u64 = 300;

const GLASS_OPACITY_MIN: u32 = 0;
const GLASS_OPACITY_MAX: u32 = 100;
const GLASS_BLUR_MIN: u32 = 8;
const GLASS_BLUR_MAX: u32 = 40;
const GLASS_SATURATE_MIN: u32 = 100;
const GLASS_SATURATE_MAX: u32 = 220;
const GLASS_BRIGHTNESS_MIN: u32 = 85;
const GLASS_BRIGHTNESS_MAX: u32 = 130;

const DEFAULT_THEME_PREFERENCE: &str = "system";
const DEFAULT_LAYOUT_PREFERENCE: &str = "topbar";

fn settings_lock() -> &'static Mutex<()> {
    static SETTINGS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    SETTINGS_LOCK.get_or_init(|| Mutex::new(()))
}

struct SettingsFileLock {
    path: PathBuf,
    _file: fs::File,
}

impl Drop for SettingsFileLock {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(
                event = "user_settings_lock_release_failed",
                detail = %error,
                lock_path = %self.path.to_string_lossy()
            );
        }
    }
}

fn settings_file_lock_path(path: &Path) -> PathBuf {
    path.with_file_name(SETTINGS_FILE_LOCK_NAME)
}

fn cleanup_stale_settings_file_lock(lock_path: &Path) {
    let metadata = match fs::metadata(lock_path) {
        Ok(value) => value,
        Err(_) => return,
    };
    let modified = match metadata.modified() {
        Ok(value) => value,
        Err(_) => return,
    };
    let age = match SystemTime::now().duration_since(modified) {
        Ok(value) => value,
        Err(_) => Duration::from_secs(0),
    };
    if age < Duration::from_secs(SETTINGS_FILE_LOCK_STALE_SECS) {
        return;
    }

    match fs::remove_file(lock_path) {
        Ok(_) => {
            tracing::warn!(
                event = "user_settings_stale_lock_removed",
                lock_path = %lock_path.to_string_lossy(),
                stale_secs = age.as_secs()
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            tracing::warn!(
                event = "user_settings_stale_lock_cleanup_failed",
                detail = %error,
                lock_path = %lock_path.to_string_lossy()
            );
        }
    }
}

fn acquire_settings_file_lock(path: &Path) -> AppResult<SettingsFileLock> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "user_settings_path_invalid",
            "用户设置路径无效，无法获取文件锁",
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        AppError::new("user_settings_dir_create_failed", "创建用户设置目录失败")
            .with_source(error)
            .with_context("path", parent.to_string_lossy().to_string())
    })?;

    let lock_path = settings_file_lock_path(path);
    let started_at = Instant::now();
    loop {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(file) => {
                return Ok(SettingsFileLock {
                    path: lock_path,
                    _file: file,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup_stale_settings_file_lock(lock_path.as_path());
                if started_at.elapsed() >= Duration::from_millis(SETTINGS_FILE_LOCK_TIMEOUT_MS) {
                    return Err(AppError::new(
                        "user_settings_lock_acquire_failed",
                        "获取用户设置文件锁超时",
                    )
                    .with_context("path", lock_path.to_string_lossy().to_string()));
                }
                thread::sleep(Duration::from_millis(SETTINGS_FILE_LOCK_RETRY_MS));
            }
            Err(error) => {
                return Err(AppError::new(
                    "user_settings_lock_acquire_failed",
                    "获取用户设置文件锁失败",
                )
                .with_source(error)
                .with_context("path", lock_path.to_string_lossy().to_string()));
            }
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        if let Some(value) = env::var_os("USERPROFILE")
            && !value.is_empty()
        {
            return Some(PathBuf::from(value));
        }

        let drive = env::var_os("HOMEDRIVE");
        let path = env::var_os("HOMEPATH");
        if let (Some(drive), Some(path)) = (drive, path) {
            let mut combined = PathBuf::from(drive);
            combined.push(path);
            if !combined.as_os_str().is_empty() {
                return Some(combined);
            }
        }
        return None;
    }

    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn settings_file_path() -> AppResult<PathBuf> {
    let home = home_dir().ok_or_else(|| {
        AppError::new(
            "user_settings_home_dir_unavailable",
            "无法定位用户主目录，读取设置失败",
        )
    })?;
    Ok(home.join(SETTINGS_DIR_NAME).join(SETTINGS_FILE_NAME))
}

fn normalize_theme_preference(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "light" => Some("light"),
        "dark" => Some("dark"),
        "system" => Some("system"),
        _ => None,
    }
}

fn normalize_layout_preference(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "topbar" => Some("topbar"),
        "sidebar" => Some("sidebar"),
        _ => None,
    }
}

fn clamp_profile(profile: &mut UserGlassProfileDto) {
    profile.opacity = profile.opacity.clamp(GLASS_OPACITY_MIN, GLASS_OPACITY_MAX);
    profile.blur = profile.blur.clamp(GLASS_BLUR_MIN, GLASS_BLUR_MAX);
    profile.saturate = profile
        .saturate
        .clamp(GLASS_SATURATE_MIN, GLASS_SATURATE_MAX);
    profile.brightness = profile
        .brightness
        .clamp(GLASS_BRIGHTNESS_MIN, GLASS_BRIGHTNESS_MAX);
}

fn normalize_clipboard_settings(settings: UserClipboardSettingsDto) -> UserClipboardSettingsDto {
    UserClipboardSettingsDto {
        max_items: settings
            .max_items
            .clamp(CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_ITEMS_MAX),
        size_cleanup_enabled: settings.size_cleanup_enabled,
        max_total_size_mb: settings.max_total_size_mb.clamp(
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN,
            CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX,
        ),
    }
}

fn normalize_settings(mut settings: UserSettingsDto) -> UserSettingsDto {
    settings.theme.preference = normalize_theme_preference(settings.theme.preference.as_str())
        .unwrap_or(DEFAULT_THEME_PREFERENCE)
        .to_string();
    settings.layout.preference = normalize_layout_preference(settings.layout.preference.as_str())
        .unwrap_or(DEFAULT_LAYOUT_PREFERENCE)
        .to_string();
    settings.locale.preference = normalize_locale_preference(settings.locale.preference.as_str())
        .unwrap_or_else(|| SYSTEM_LOCALE_PREFERENCE.to_string());

    clamp_profile(&mut settings.theme.glass.light);
    clamp_profile(&mut settings.theme.glass.dark);
    settings.clipboard = normalize_clipboard_settings(settings.clipboard);
    settings
}

fn write_settings_file(path: &Path, settings: &UserSettingsDto) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "user_settings_path_invalid",
            "用户设置路径无效，无法写入配置文件",
        )
    })?;

    fs::create_dir_all(parent).map_err(|error| {
        AppError::new("user_settings_dir_create_failed", "创建用户设置目录失败")
            .with_source(error)
            .with_context("path", parent.to_string_lossy().to_string())
    })?;

    let serialized = serde_json::to_string_pretty(settings).map_err(|error| {
        AppError::new("user_settings_serialize_failed", "序列化用户设置失败").with_source(error)
    })?;

    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, serialized).map_err(|error| {
        AppError::new("user_settings_temp_write_failed", "写入临时设置文件失败")
            .with_source(error)
            .with_context("path", temp_path.to_string_lossy().to_string())
    })?;

    match fs::rename(&temp_path, path) {
        Ok(_) => Ok(()),
        Err(rename_error) => {
            fs::copy(&temp_path, path).map_err(|error| {
                let _ = fs::remove_file(&temp_path);
                AppError::new(
                    "user_settings_atomic_replace_failed",
                    "替换用户设置文件失败",
                )
                .with_source(error)
                .with_context("path", path.to_string_lossy().to_string())
                .with_context("rename_error", rename_error.to_string())
            })?;
            fs::remove_file(&temp_path).map_err(|error| {
                AppError::new("user_settings_temp_cleanup_failed", "清理临时设置文件失败")
                    .with_source(error)
                    .with_context("path", temp_path.to_string_lossy().to_string())
            })?;
            Ok(())
        }
    }
}

fn backup_corrupted_settings_file(path: &Path, content: &str) -> Option<PathBuf> {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let backup_name = format!("{}.bak.{}", SETTINGS_FILE_NAME, suffix);
    let backup_path = path.with_file_name(backup_name);

    match fs::write(&backup_path, content) {
        Ok(_) => Some(backup_path),
        Err(error) => {
            tracing::warn!(
                event = "user_settings_backup_failed",
                detail = %error,
                path = %path.to_string_lossy(),
                backup_path = %backup_path.to_string_lossy()
            );
            None
        }
    }
}

fn load_or_init_unlocked(path: &Path) -> AppResult<UserSettingsDto> {
    if !path.exists() {
        let settings = normalize_settings(UserSettingsDto::default());
        write_settings_file(path, &settings)?;
        return Ok(settings);
    }

    let content = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let settings = normalize_settings(UserSettingsDto::default());
            write_settings_file(path, &settings)?;
            return Ok(settings);
        }
        Err(error) => {
            return Err(
                AppError::new("user_settings_read_failed", "读取用户设置文件失败")
                    .with_source(error)
                    .with_context("path", path.to_string_lossy().to_string()),
            );
        }
    };

    let parsed = serde_json::from_str::<UserSettingsDto>(&content);
    let (settings, force_write) = match parsed {
        Ok(value) => (normalize_settings(value), false),
        Err(error) => {
            let backup = backup_corrupted_settings_file(path, &content);
            if let Some(backup_path) = backup {
                tracing::warn!(
                    event = "user_settings_parse_failed",
                    detail = %error,
                    path = %path.to_string_lossy(),
                    backup_path = %backup_path.to_string_lossy()
                );
            } else {
                tracing::warn!(
                    event = "user_settings_parse_failed",
                    detail = %error,
                    path = %path.to_string_lossy()
                );
            }
            (normalize_settings(UserSettingsDto::default()), true)
        }
    };

    if force_write {
        write_settings_file(path, &settings)?;
        return Ok(settings);
    }

    let normalized_content = serde_json::to_string_pretty(&settings).map_err(|error| {
        AppError::new("user_settings_serialize_failed", "序列化用户设置失败").with_source(error)
    })?;
    if normalized_content != content {
        write_settings_file(path, &settings)?;
    }

    Ok(settings)
}

fn apply_glass_patch(profile: &mut UserGlassProfileDto, input: &UserGlassProfileUpdateInputDto) {
    if let Some(value) = input.opacity {
        profile.opacity = value;
    }
    if let Some(value) = input.blur {
        profile.blur = value;
    }
    if let Some(value) = input.saturate {
        profile.saturate = value;
    }
    if let Some(value) = input.brightness {
        profile.brightness = value;
    }
}

fn apply_theme_patch(
    theme: &mut app_core::models::UserThemeSettingsDto,
    input: &UserThemeSettingsUpdateInputDto,
) -> AppResult<()> {
    if let Some(preference) = &input.preference {
        theme.preference = normalize_theme_preference(preference.as_str())
            .ok_or_else(|| {
                AppError::new("invalid_theme_preference", "主题偏好无效")
                    .with_context("preference", preference.clone())
            })?
            .to_string();
    }

    if let Some(glass) = &input.glass {
        apply_glass_theme_patch(&mut theme.glass, glass);
    }

    Ok(())
}

fn apply_glass_theme_patch(
    glass: &mut app_core::models::UserThemeGlassSettingsDto,
    input: &UserThemeGlassSettingsUpdateInputDto,
) {
    if let Some(light) = &input.light {
        apply_glass_patch(&mut glass.light, light);
    }
    if let Some(dark) = &input.dark {
        apply_glass_patch(&mut glass.dark, dark);
    }
}

fn apply_layout_patch(
    layout: &mut app_core::models::UserLayoutSettingsDto,
    input: &UserLayoutSettingsUpdateInputDto,
) -> AppResult<()> {
    if let Some(preference) = &input.preference {
        layout.preference = normalize_layout_preference(preference.as_str())
            .ok_or_else(|| {
                AppError::new("invalid_layout_preference", "布局偏好无效")
                    .with_context("preference", preference.clone())
            })?
            .to_string();
    }
    Ok(())
}

fn apply_locale_patch(
    locale: &mut app_core::models::UserLocaleSettingsDto,
    input: &UserLocaleSettingsUpdateInputDto,
) -> AppResult<()> {
    if let Some(preference) = &input.preference {
        locale.preference = normalize_locale_preference(preference.as_str()).ok_or_else(|| {
            AppError::new("invalid_locale_preference", "语言偏好无效")
                .with_context("preference", preference.clone())
        })?;
    }
    Ok(())
}

fn apply_clipboard_patch(
    clipboard: &mut UserClipboardSettingsDto,
    input: &UserClipboardSettingsUpdateInputDto,
) {
    if let Some(max_items) = input.max_items {
        clipboard.max_items = max_items;
    }
    if let Some(size_cleanup_enabled) = input.size_cleanup_enabled {
        clipboard.size_cleanup_enabled = size_cleanup_enabled;
    }
    if let Some(max_total_size_mb) = input.max_total_size_mb {
        clipboard.max_total_size_mb = max_total_size_mb;
    }
}

fn apply_update(
    settings: &mut UserSettingsDto,
    input: &UserSettingsUpdateInputDto,
) -> AppResult<()> {
    if let Some(theme) = &input.theme {
        apply_theme_patch(&mut settings.theme, theme)?;
    }
    if let Some(layout) = &input.layout {
        apply_layout_patch(&mut settings.layout, layout)?;
    }
    if let Some(locale) = &input.locale {
        apply_locale_patch(&mut settings.locale, locale)?;
    }
    if let Some(clipboard) = &input.clipboard {
        apply_clipboard_patch(&mut settings.clipboard, clipboard);
    }
    *settings = normalize_settings(settings.clone());
    Ok(())
}

pub fn load_or_init_user_settings() -> AppResult<UserSettingsDto> {
    let guard = settings_lock().lock();
    let _guard = match guard {
        Ok(value) => value,
        Err(poisoned) => poisoned.into_inner(),
    };

    let path = settings_file_path()?;
    let _file_lock = acquire_settings_file_lock(path.as_path())?;
    load_or_init_unlocked(path.as_path())
}

pub fn update_user_settings(input: UserSettingsUpdateInputDto) -> AppResult<UserSettingsDto> {
    let guard = settings_lock().lock();
    let _guard = match guard {
        Ok(value) => value,
        Err(poisoned) => poisoned.into_inner(),
    };

    let path = settings_file_path()?;
    let _file_lock = acquire_settings_file_lock(path.as_path())?;
    let mut settings = load_or_init_unlocked(path.as_path())?;
    apply_update(&mut settings, &input)?;
    write_settings_file(path.as_path(), &settings)?;
    Ok(settings)
}

pub fn update_locale_preference(preference: &str) -> AppResult<UserSettingsDto> {
    let update = UserSettingsUpdateInputDto {
        locale: Some(UserLocaleSettingsUpdateInputDto {
            preference: Some(preference.to_string()),
        }),
        ..Default::default()
    };
    update_user_settings(update)
}

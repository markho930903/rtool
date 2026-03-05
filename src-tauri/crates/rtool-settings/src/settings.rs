use rtool_capture::service::{
    CLIPBOARD_MAX_ITEMS_MAX, CLIPBOARD_MAX_ITEMS_MIN, CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX,
    CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN,
};
use rtool_capture::{
    SCREENSHOT_MAX_ITEMS_MAX, SCREENSHOT_MAX_ITEMS_MIN, SCREENSHOT_MAX_TOTAL_SIZE_MB_MAX,
    SCREENSHOT_MAX_TOTAL_SIZE_MB_MIN, SCREENSHOT_PIN_MAX_INSTANCES_MAX,
    SCREENSHOT_PIN_MAX_INSTANCES_MIN, SCREENSHOT_SHORTCUT_DEFAULT,
};
use rtool_contracts::models::{
    LayoutSettingsUpdateInputDto, LocaleSettingsUpdateInputDto, SettingsClipboardDto,
    SettingsClipboardUpdateInputDto, SettingsDto, SettingsScreenshotDto,
    SettingsScreenshotUpdateInputDto, SettingsUpdateInputDto, ThemeSettingsUpdateInputDto,
};
use rtool_contracts::{AppError, AppResult};
use rtool_data::db::{DbConn, get_app_setting, set_app_setting};
use rtool_kernel::i18n::{SYSTEM_LOCALE_PREFERENCE, normalize_locale_preference};

const APP_SETTINGS_JSON_KEY: &str = "app.settings.v1";
const DEFAULT_THEME_PREFERENCE: &str = "system";
const DEFAULT_LAYOUT_PREFERENCE: &str = "topbar";

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

fn normalize_clipboard_settings(settings: SettingsClipboardDto) -> SettingsClipboardDto {
    SettingsClipboardDto {
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

fn normalize_screenshot_settings(settings: SettingsScreenshotDto) -> SettingsScreenshotDto {
    let shortcut = settings.shortcut.trim();
    SettingsScreenshotDto {
        shortcut: if shortcut.is_empty() {
            SCREENSHOT_SHORTCUT_DEFAULT.to_string()
        } else {
            shortcut.to_string()
        },
        auto_save_enabled: settings.auto_save_enabled,
        max_items: settings
            .max_items
            .clamp(SCREENSHOT_MAX_ITEMS_MIN, SCREENSHOT_MAX_ITEMS_MAX),
        max_total_size_mb: settings.max_total_size_mb.clamp(
            SCREENSHOT_MAX_TOTAL_SIZE_MB_MIN,
            SCREENSHOT_MAX_TOTAL_SIZE_MB_MAX,
        ),
        pin_max_instances: settings.pin_max_instances.clamp(
            SCREENSHOT_PIN_MAX_INSTANCES_MIN,
            SCREENSHOT_PIN_MAX_INSTANCES_MAX,
        ),
    }
}

fn normalize_settings(mut settings: SettingsDto) -> SettingsDto {
    settings.theme.preference = normalize_theme_preference(settings.theme.preference.as_str())
        .unwrap_or(DEFAULT_THEME_PREFERENCE)
        .to_string();
    settings.layout.preference = normalize_layout_preference(settings.layout.preference.as_str())
        .unwrap_or(DEFAULT_LAYOUT_PREFERENCE)
        .to_string();
    settings.locale.preference = normalize_locale_preference(settings.locale.preference.as_str())
        .unwrap_or_else(|| SYSTEM_LOCALE_PREFERENCE.to_string());

    settings.clipboard = normalize_clipboard_settings(settings.clipboard);
    settings.screenshot = normalize_screenshot_settings(settings.screenshot);
    settings
}

fn apply_theme_patch(
    theme: &mut rtool_contracts::models::ThemeSettingsDto,
    input: &ThemeSettingsUpdateInputDto,
) -> AppResult<()> {
    if let Some(preference) = &input.preference {
        theme.preference = normalize_theme_preference(preference.as_str())
            .ok_or_else(|| {
                AppError::new("invalid_theme_preference", "主题偏好无效")
                    .with_context("preference", preference.clone())
            })?
            .to_string();
    }

    if let Some(transparent_window_background) = input.transparent_window_background {
        theme.transparent_window_background = transparent_window_background;
    }

    Ok(())
}

fn apply_layout_patch(
    layout: &mut rtool_contracts::models::LayoutSettingsDto,
    input: &LayoutSettingsUpdateInputDto,
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
    locale: &mut rtool_contracts::models::LocaleSettingsDto,
    input: &LocaleSettingsUpdateInputDto,
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
    clipboard: &mut SettingsClipboardDto,
    input: &SettingsClipboardUpdateInputDto,
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

fn apply_screenshot_patch(
    screenshot: &mut SettingsScreenshotDto,
    input: &SettingsScreenshotUpdateInputDto,
) {
    if let Some(shortcut) = &input.shortcut {
        screenshot.shortcut = shortcut.clone();
    }
    if let Some(auto_save_enabled) = input.auto_save_enabled {
        screenshot.auto_save_enabled = auto_save_enabled;
    }
    if let Some(max_items) = input.max_items {
        screenshot.max_items = max_items;
    }
    if let Some(max_total_size_mb) = input.max_total_size_mb {
        screenshot.max_total_size_mb = max_total_size_mb;
    }
    if let Some(pin_max_instances) = input.pin_max_instances {
        screenshot.pin_max_instances = pin_max_instances;
    }
}

fn apply_update(settings: &mut SettingsDto, input: &SettingsUpdateInputDto) -> AppResult<()> {
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
    if let Some(screenshot) = &input.screenshot {
        apply_screenshot_patch(&mut settings.screenshot, screenshot);
    }
    *settings = normalize_settings(settings.clone());
    Ok(())
}

fn serialize_settings(settings: &SettingsDto) -> AppResult<String> {
    serde_json::to_string(settings).map_err(|error| {
        AppError::new("settings_serialize_failed", "序列化用户设置失败").with_source(error)
    })
}

async fn persist_settings(db_conn: &DbConn, settings: &SettingsDto) -> AppResult<()> {
    let serialized = serialize_settings(settings)?;
    set_app_setting(db_conn, APP_SETTINGS_JSON_KEY, serialized.as_str())
        .await
        .map_err(|error| {
            AppError::new("settings_write_failed", "写入用户设置失败")
                .with_source(error)
                .with_context("key", APP_SETTINGS_JSON_KEY.to_string())
        })
}

pub async fn load_or_init_settings(db_conn: &DbConn) -> AppResult<SettingsDto> {
    let raw = get_app_setting(db_conn, APP_SETTINGS_JSON_KEY)
        .await
        .map_err(|error| {
            AppError::new("settings_read_failed", "读取用户设置失败")
                .with_source(error)
                .with_context("key", APP_SETTINGS_JSON_KEY.to_string())
        })?;

    let Some(raw) = raw else {
        let settings = normalize_settings(SettingsDto::default());
        persist_settings(db_conn, &settings).await?;
        return Ok(settings);
    };

    let parsed = serde_json::from_str::<SettingsDto>(raw.as_str());
    let (settings, force_write) = match parsed {
        Ok(value) => (normalize_settings(value), false),
        Err(error) => {
            tracing::warn!(
                event = "settings_parse_failed",
                detail = %error,
                key = APP_SETTINGS_JSON_KEY
            );
            (normalize_settings(SettingsDto::default()), true)
        }
    };

    let normalized_raw = serialize_settings(&settings)?;
    if force_write || normalized_raw != raw {
        persist_settings(db_conn, &settings).await?;
    }

    Ok(settings)
}

pub async fn update_settings(
    db_conn: &DbConn,
    input: SettingsUpdateInputDto,
) -> AppResult<SettingsDto> {
    let mut settings = load_or_init_settings(db_conn).await?;
    apply_update(&mut settings, &input)?;
    persist_settings(db_conn, &settings).await?;
    Ok(settings)
}

pub async fn update_locale_preference(
    db_conn: &DbConn,
    preference: &str,
) -> AppResult<SettingsDto> {
    let update = SettingsUpdateInputDto {
        locale: Some(LocaleSettingsUpdateInputDto {
            preference: Some(preference.to_string()),
        }),
        ..Default::default()
    };
    update_settings(db_conn, update).await
}

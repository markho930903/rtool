use super::ingest::{normalize_level, sanitize_for_log};
use super::{
    DEFAULT_ALLOW_RAW_VIEW, DEFAULT_HIGH_FREQ_MAX_PER_KEY, DEFAULT_HIGH_FREQ_WINDOW_MS,
    DEFAULT_KEEP_DAYS, DEFAULT_MIN_LEVEL, DEFAULT_REALTIME_ENABLED, SETTING_KEY_ALLOW_RAW_VIEW,
    SETTING_KEY_HIGH_FREQ_MAX_PER_KEY, SETTING_KEY_HIGH_FREQ_WINDOW_MS, SETTING_KEY_KEEP_DAYS,
    SETTING_KEY_MIN_LEVEL, SETTING_KEY_REALTIME_ENABLED,
};
use crate::AppError;
use crate::db::{self, DbConn};
use crate::models::LogConfigDto;

fn bool_setting(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub(super) async fn load_log_config(conn: &DbConn) -> LogConfigDto {
    let keys = [
        SETTING_KEY_MIN_LEVEL,
        SETTING_KEY_KEEP_DAYS,
        SETTING_KEY_REALTIME_ENABLED,
        SETTING_KEY_HIGH_FREQ_WINDOW_MS,
        SETTING_KEY_HIGH_FREQ_MAX_PER_KEY,
        SETTING_KEY_ALLOW_RAW_VIEW,
    ];
    let settings = db::get_app_settings_batch(conn, &keys)
        .await
        .unwrap_or_default();

    LogConfigDto {
        min_level: settings
            .get(SETTING_KEY_MIN_LEVEL)
            .and_then(|value| normalize_level(value).map(ToString::to_string))
            .unwrap_or_else(|| DEFAULT_MIN_LEVEL.to_string()),
        keep_days: settings
            .get(SETTING_KEY_KEEP_DAYS)
            .and_then(|value| value.parse::<u32>().ok())
            .map(|value| value.clamp(1, 90))
            .unwrap_or(DEFAULT_KEEP_DAYS),
        realtime_enabled: settings
            .get(SETTING_KEY_REALTIME_ENABLED)
            .and_then(|value| value.parse::<bool>().ok())
            .unwrap_or(DEFAULT_REALTIME_ENABLED),
        high_freq_window_ms: settings
            .get(SETTING_KEY_HIGH_FREQ_WINDOW_MS)
            .and_then(|value| value.parse::<u32>().ok())
            .map(|value| value.clamp(100, 60_000))
            .unwrap_or(DEFAULT_HIGH_FREQ_WINDOW_MS),
        high_freq_max_per_key: settings
            .get(SETTING_KEY_HIGH_FREQ_MAX_PER_KEY)
            .and_then(|value| value.parse::<u32>().ok())
            .map(|value| value.clamp(1, 200))
            .unwrap_or(DEFAULT_HIGH_FREQ_MAX_PER_KEY),
        allow_raw_view: settings
            .get(SETTING_KEY_ALLOW_RAW_VIEW)
            .and_then(|value| value.parse::<bool>().ok())
            .unwrap_or(DEFAULT_ALLOW_RAW_VIEW),
    }
}

pub(super) async fn persist_log_config(
    conn: &DbConn,
    config: &LogConfigDto,
) -> Result<(), AppError> {
    let keep_days = config.keep_days.to_string();
    let high_freq_window_ms = config.high_freq_window_ms.to_string();
    let high_freq_max_per_key = config.high_freq_max_per_key.to_string();
    let entries = [
        (SETTING_KEY_MIN_LEVEL, config.min_level.as_str()),
        (SETTING_KEY_KEEP_DAYS, keep_days.as_str()),
        (
            SETTING_KEY_REALTIME_ENABLED,
            bool_setting(config.realtime_enabled),
        ),
        (
            SETTING_KEY_HIGH_FREQ_WINDOW_MS,
            high_freq_window_ms.as_str(),
        ),
        (
            SETTING_KEY_HIGH_FREQ_MAX_PER_KEY,
            high_freq_max_per_key.as_str(),
        ),
        (
            SETTING_KEY_ALLOW_RAW_VIEW,
            bool_setting(config.allow_raw_view),
        ),
    ];
    db::set_app_settings_batch(conn, entries.as_slice()).await?;
    Ok(())
}

pub(super) fn clamp_and_normalize_config(
    mut config: LogConfigDto,
) -> Result<LogConfigDto, AppError> {
    let level = normalize_level(&config.min_level).ok_or_else(|| {
        AppError::new("invalid_log_level", "日志级别非法")
            .with_context("level", sanitize_for_log(&config.min_level))
    })?;

    config.min_level = level.to_string();
    config.keep_days = config.keep_days.clamp(1, 90);
    config.high_freq_window_ms = config.high_freq_window_ms.clamp(100, 60_000);
    config.high_freq_max_per_key = config.high_freq_max_per_key.clamp(1, 200);
    Ok(config)
}

pub(super) fn resolve_log_level() -> String {
    let env_level = std::env::var("RTOOL_LOG_LEVEL")
        .ok()
        .map(|value| value.to_ascii_lowercase());
    if let Some(level) = env_level
        && matches!(
            level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        )
    {
        return level;
    }

    if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "info".to_string()
    }
}

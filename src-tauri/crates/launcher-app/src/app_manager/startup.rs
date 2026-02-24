use super::*;

#[cfg(target_os = "windows")]
const WINDOWS_STARTUP_CACHE_TTL: Duration = Duration::from_secs(8);

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct WindowsStartupSnapshot {
    refreshed_at: Option<Instant>,
    user_value_names: HashSet<String>,
    system_value_names: HashSet<String>,
    user_values: Vec<String>,
    system_values: Vec<String>,
}

#[cfg(target_os = "windows")]
impl WindowsStartupSnapshot {
    fn new() -> Self {
        Self {
            refreshed_at: None,
            user_value_names: HashSet::new(),
            system_value_names: HashSet::new(),
            user_values: Vec::new(),
            system_values: Vec::new(),
        }
    }

    fn is_stale(&self) -> bool {
        match self.refreshed_at {
            None => true,
            Some(at) => at.elapsed() >= WINDOWS_STARTUP_CACHE_TTL,
        }
    }
}

pub(super) fn platform_detect_startup_state(
    app_id: &str,
    app_path: &Path,
) -> (bool, AppManagerStartupScope, bool) {
    #[cfg(target_os = "macos")]
    {
        let user_label_enabled = mac_startup_file_path(app_id).is_some_and(|path| path.exists());
        let cache = mac_get_startup_cache_snapshot();
        let target = app_path.to_string_lossy().to_ascii_lowercase();
        let escaped_target = xml_escape(app_path.to_string_lossy().as_ref()).to_ascii_lowercase();
        let user_match = cache
            .user_plist_blobs
            .iter()
            .any(|blob| blob.contains(target.as_str()) || blob.contains(escaped_target.as_str()));
        let system_match = cache
            .system_plist_blobs
            .iter()
            .any(|blob| blob.contains(target.as_str()) || blob.contains(escaped_target.as_str()));

        if system_match {
            return (true, AppManagerStartupScope::System, false);
        }
        if user_label_enabled || user_match {
            return (true, AppManagerStartupScope::User, true);
        }
        (false, AppManagerStartupScope::None, true)
    }
    #[cfg(target_os = "windows")]
    {
        let user_enabled = windows_startup_enabled(app_id)
            || windows_run_registry_contains(
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                app_path,
            );
        let system_enabled = windows_run_registry_contains(
            r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run",
            app_path,
        );
        if system_enabled {
            return (true, AppManagerStartupScope::System, false);
        }
        if user_enabled {
            return (true, AppManagerStartupScope::User, true);
        }
        (false, AppManagerStartupScope::None, true)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app_id;
        let _ = app_path;
        (false, AppManagerStartupScope::None, false)
    }
}

pub(super) fn platform_set_startup(app_id: &str, app_path: &Path, enabled: bool) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        mac_set_startup(app_id, app_path, enabled)
    }
    #[cfg(target_os = "windows")]
    {
        windows_set_startup(app_id, app_path, enabled)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app_id;
        let _ = app_path;
        let _ = enabled;
        Err(app_error(
            AppManagerErrorCode::StartupNotSupported,
            "当前平台暂不支持启动项修改",
        ))
    }
}

#[cfg(target_os = "macos")]
pub(super) fn mac_startup_file_path(app_id: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    let label = startup_label(app_id);
    Some(
        home.join("Library")
            .join("LaunchAgents")
            .join(format!("{label}.plist")),
    )
}

#[cfg(target_os = "macos")]
pub(super) fn mac_get_startup_cache_snapshot() -> MacStartupCache {
    let stale = {
        let cache = mac_startup_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.is_stale()
    };
    if stale {
        let user_blobs = home_dir()
            .map(|home| home.join("Library").join("LaunchAgents"))
            .map(|path| mac_collect_plist_blobs(path.as_path()))
            .unwrap_or_default();
        let mut system_blobs = Vec::new();
        system_blobs.extend(mac_collect_plist_blobs(Path::new("/Library/LaunchAgents")));
        system_blobs.extend(mac_collect_plist_blobs(Path::new("/Library/LaunchDaemons")));

        let mut cache = mac_startup_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.user_plist_blobs = user_blobs;
        cache.system_plist_blobs = system_blobs;
        cache.refreshed_at = Some(Instant::now());
        return cache.clone();
    }

    let cache = mac_startup_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.clone()
}

#[cfg(target_os = "macos")]
pub(super) fn mac_collect_plist_blobs(root: &Path) -> Vec<String> {
    if !root.exists() {
        return Vec::new();
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut blobs = Vec::new();
    for entry in entries.flatten().take(500) {
        let path = entry.path();
        if !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("plist"))
        {
            continue;
        }
        if let Some(content) = mac_read_plist_text(path.as_path()) {
            blobs.push(content.to_ascii_lowercase());
        }
    }
    blobs
}

#[cfg(target_os = "macos")]
pub(super) fn mac_read_plist_text(path: &Path) -> Option<String> {
    if let Ok(content) = fs::read_to_string(path) {
        return Some(content);
    }
    let output = Command::new("plutil")
        .args([
            "-convert",
            "xml1",
            "-o",
            "-",
            path.to_string_lossy().as_ref(),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(target_os = "macos")]
pub(super) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "macos")]
pub(super) fn mac_set_startup(app_id: &str, app_path: &Path, enabled: bool) -> AppResult<()> {
    let startup_path = mac_startup_file_path(app_id).ok_or_else(|| {
        app_error(
            AppManagerErrorCode::StartupPathMissing,
            "无法定位启动项目录，请检查 HOME 环境",
        )
    })?;

    if enabled {
        let parent = startup_path.parent().ok_or_else(|| {
            app_error(AppManagerErrorCode::StartupPathInvalid, "启动项路径无效")
                .with_context("startupFile", startup_path.to_string_lossy().to_string())
        })?;
        fs::create_dir_all(parent)
            .with_context(|| format!("创建启动项目录失败: {}", parent.display()))
            .with_code(
                AppManagerErrorCode::StartupDirCreateFailed.as_str(),
                "创建启动项目录失败",
            )
            .with_ctx("startupDir", parent.display().to_string())?;

        let label = startup_label(app_id);
        let app_str = app_path.to_string_lossy().to_string();
        let program_arguments = if app_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("app"))
        {
            format!(
                "<array><string>/usr/bin/open</string><string>-a</string><string>{}</string></array>",
                xml_escape(app_str.as_str())
            )
        } else {
            format!(
                "<array><string>{}</string></array>",
                xml_escape(app_str.as_str())
            )
        };

        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{}</string>
  <key>ProgramArguments</key>
  {}
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#,
            xml_escape(label.as_str()),
            program_arguments
        );

        fs::write(startup_path.as_path(), plist)
            .with_context(|| format!("写入启动项失败: {}", startup_path.display()))
            .with_code(
                AppManagerErrorCode::StartupWriteFailed.as_str(),
                "写入启动项失败",
            )
            .with_ctx("startupFile", startup_path.display().to_string())
            .with_ctx("appId", app_id.to_string())?;
        return Ok(());
    }

    match fs::remove_file(&startup_path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(
            app_error(AppManagerErrorCode::StartupDeleteFailed, "删除启动项失败")
                .with_source(error)
                .with_context("startupFile", startup_path.display().to_string())
                .with_context("appId", app_id.to_string()),
        ),
    }
}

#[cfg(target_os = "windows")]
pub(super) fn windows_startup_value_name(app_id: &str) -> String {
    format!(
        "RToolStartup_{}",
        stable_hash(app_id).chars().take(10).collect::<String>()
    )
}

#[cfg(target_os = "windows")]
pub(super) fn windows_startup_enabled(app_id: &str) -> bool {
    let snapshot = windows_get_startup_snapshot();
    snapshot
        .user_value_names
        .contains(windows_startup_value_name(app_id).to_ascii_lowercase().as_str())
}

#[cfg(target_os = "windows")]
pub(super) fn windows_run_registry_contains(root: &str, app_path: &Path) -> bool {
    let target = normalize_path_key(app_path.to_string_lossy().as_ref());
    if target.is_empty() {
        return false;
    }

    let snapshot = windows_get_startup_snapshot();
    let values = if root.eq_ignore_ascii_case(r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run")
    {
        snapshot.system_values
    } else {
        snapshot.user_values
    };
    values.iter().any(|value| value.contains(target.as_str()))
}

#[cfg(target_os = "windows")]
pub(super) fn windows_set_startup(app_id: &str, app_path: &Path, enabled: bool) -> AppResult<()> {
    let value_name = windows_startup_value_name(app_id);
    if enabled {
        let command_value = format!("\"{}\"", app_path.to_string_lossy());
        let status = Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                value_name.as_str(),
                "/t",
                "REG_SZ",
                "/d",
                command_value.as_str(),
                "/f",
            ])
            .status()
            .with_context(|| format!("写入注册表启动项失败: {}", value_name))
            .with_code(
                AppManagerErrorCode::StartupUpdateFailed.as_str(),
                "更新启动项失败",
            )
            .with_ctx("valueName", value_name.clone())
            .with_ctx("appPath", app_path.to_string_lossy().to_string())?;
        if !status.success() {
            return Err(
                app_error(AppManagerErrorCode::StartupUpdateFailed, "更新启动项失败")
                    .with_context("status", status.to_string())
                    .with_context("valueName", value_name.clone())
                    .with_context("appPath", app_path.to_string_lossy().to_string()),
            );
        }
        windows_invalidate_startup_snapshot();
        return Ok(());
    }

    let status = Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            value_name.as_str(),
            "/f",
        ])
        .status()
        .with_context(|| format!("删除注册表启动项失败: {}", value_name))
        .with_code(
            AppManagerErrorCode::StartupUpdateFailed.as_str(),
            "更新启动项失败",
        )
        .with_ctx("valueName", value_name.clone())
        .with_ctx("appPath", app_path.to_string_lossy().to_string())?;
    if !status.success() {
        return Err(
            app_error(AppManagerErrorCode::StartupUpdateFailed, "更新启动项失败")
                .with_context("status", status.to_string())
                .with_context("valueName", value_name.clone())
                .with_context("appPath", app_path.to_string_lossy().to_string()),
        );
    }
    windows_invalidate_startup_snapshot();
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_startup_snapshot_cache() -> &'static Mutex<WindowsStartupSnapshot> {
    static CACHE: OnceLock<Mutex<WindowsStartupSnapshot>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(WindowsStartupSnapshot::new()))
}

#[cfg(target_os = "windows")]
fn windows_invalidate_startup_snapshot() {
    let mut cache = windows_startup_snapshot_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.refreshed_at = None;
}

#[cfg(target_os = "windows")]
fn windows_query_run_values(root: &str) -> (HashSet<String>, Vec<String>) {
    let output = match Command::new("reg").args(["query", root]).output() {
        Ok(output) => output,
        Err(_) => return (HashSet::new(), Vec::new()),
    };
    if !output.status.success() {
        return (HashSet::new(), Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut value_names = HashSet::new();
    let mut values = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if let Some((name, value)) = windows_parse_reg_value_line(trimmed) {
            value_names.insert(name.to_ascii_lowercase());
            let normalized = normalize_path_key(value.as_str());
            if !normalized.is_empty() {
                values.push(normalized);
            }
        }
    }
    (value_names, values)
}

#[cfg(target_os = "windows")]
fn windows_get_startup_snapshot() -> WindowsStartupSnapshot {
    let stale = {
        let cache = windows_startup_snapshot_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.is_stale()
    };
    if stale {
        let (user_value_names, user_values) =
            windows_query_run_values(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run");
        let (system_value_names, system_values) =
            windows_query_run_values(r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run");
        let mut cache = windows_startup_snapshot_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.user_value_names = user_value_names;
        cache.system_value_names = system_value_names;
        cache.user_values = user_values;
        cache.system_values = system_values;
        cache.refreshed_at = Some(Instant::now());
        return cache.clone();
    }
    let cache = windows_startup_snapshot_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.clone()
}

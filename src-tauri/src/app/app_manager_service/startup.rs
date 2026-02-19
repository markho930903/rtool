use super::*;

pub(super) fn platform_detect_startup_state(app_id: &str, app_path: &Path) -> (bool, String, bool) {
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
            return (true, "system".to_string(), false);
        }
        if user_label_enabled || user_match {
            return (true, "user".to_string(), true);
        }
        (false, "none".to_string(), true)
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
            return (true, "system".to_string(), false);
        }
        if user_enabled {
            return (true, "user".to_string(), true);
        }
        (false, "none".to_string(), true)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app_id;
        let _ = app_path;
        (false, "none".to_string(), false)
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
        Err(AppError::new(
            "app_manager_startup_not_supported",
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
        AppError::new(
            "app_manager_startup_path_missing",
            "无法定位启动项目录，请检查 HOME 环境",
        )
    })?;

    if enabled {
        let parent = startup_path.parent().ok_or_else(|| {
            AppError::new("app_manager_startup_path_invalid", "启动项路径无效")
                .with_context("startupFile", startup_path.to_string_lossy().to_string())
        })?;
        fs::create_dir_all(parent)
            .with_context(|| format!("创建启动项目录失败: {}", parent.display()))
            .with_code(
                "app_manager_startup_dir_create_failed",
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
            .with_code("app_manager_startup_write_failed", "写入启动项失败")
            .with_ctx("startupFile", startup_path.display().to_string())
            .with_ctx("appId", app_id.to_string())?;
        return Ok(());
    }

    match fs::remove_file(&startup_path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(
            AppError::new("app_manager_startup_delete_failed", "删除启动项失败")
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
    let value_name = windows_startup_value_name(app_id);
    Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            value_name.as_str(),
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub(super) fn windows_run_registry_contains(root: &str, app_path: &Path) -> bool {
    let target = normalize_path_key(app_path.to_string_lossy().as_ref());
    if target.is_empty() {
        return false;
    }

    let output = match Command::new("reg").args(["query", root]).output() {
        Ok(output) => output,
        Err(_) => return false,
    };
    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(|line| normalize_path_key(line))
        .any(|line| line.contains(target.as_str()))
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
            .with_code("app_manager_startup_update_failed", "更新启动项失败")
            .with_ctx("valueName", value_name.clone())
            .with_ctx("appPath", app_path.to_string_lossy().to_string())?;
        if !status.success() {
            return Err(
                AppError::new("app_manager_startup_update_failed", "更新启动项失败")
                    .with_context("status", status.to_string())
                    .with_context("valueName", value_name.clone())
                    .with_context("appPath", app_path.to_string_lossy().to_string()),
            );
        }
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
        .with_code("app_manager_startup_update_failed", "更新启动项失败")
        .with_ctx("valueName", value_name.clone())
        .with_ctx("appPath", app_path.to_string_lossy().to_string())?;
    if !status.success() {
        return Err(
            AppError::new("app_manager_startup_update_failed", "更新启动项失败")
                .with_context("status", status.to_string())
                .with_context("valueName", value_name.clone())
                .with_context("appPath", app_path.to_string_lossy().to_string()),
        );
    }
    Ok(())
}

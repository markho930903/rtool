use super::*;

pub(super) fn platform_uninstall(item: &ManagedAppDto) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        mac_uninstall(item)
    }
    #[cfg(target_os = "windows")]
    {
        windows_uninstall(item)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = item;
        Err(app_error(
            AppManagerErrorCode::UninstallNotSupported,
            "当前平台暂不支持卸载功能",
        ))
    }
}

pub(super) fn platform_open_uninstall_help(item: &ManagedAppDto) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        if item.path.trim().is_empty() {
            return Err(app_error(
                AppManagerErrorCode::OpenHelpInvalid,
                "无有效应用路径",
            ));
        }
        open_with_command(
            "open",
            &["-R", item.path.as_str()],
            AppManagerErrorCode::OpenHelpFailed,
        )
    }
    #[cfg(target_os = "windows")]
    {
        let _ = item;
        open_with_command(
            "cmd",
            &["/C", "start", "", "ms-settings:appsfeatures"],
            AppManagerErrorCode::OpenHelpFailed,
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = item;
        Err(app_error(
            AppManagerErrorCode::OpenHelpNotSupported,
            "当前平台暂不支持该操作",
        ))
    }
}

pub(super) fn platform_open_permission_help(item: &ManagedAppDto) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        if item.path.trim().is_empty() {
            return Err(app_error(
                AppManagerErrorCode::OpenHelpInvalid,
                "无有效应用路径",
            ));
        }

        let mut last_error = None;
        for args in mac_permission_help_open_targets() {
            match open_with_command("open", args, AppManagerErrorCode::OpenHelpFailed) {
                Ok(()) => return Ok(()),
                Err(error) => last_error = Some(error),
            }
        }

        return Err(last_error.unwrap_or_else(|| {
            app_error(
                AppManagerErrorCode::OpenHelpFailed,
                "打开系统权限设置入口失败",
            )
        }));
    }
    #[cfg(target_os = "windows")]
    {
        let _ = item;
        open_with_command(
            "cmd",
            &["/C", "start", "", "ms-settings:privacy"],
            AppManagerErrorCode::OpenHelpFailed,
        )
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = item;
        Err(app_error(
            AppManagerErrorCode::OpenHelpNotSupported,
            "当前平台暂不支持该操作",
        ))
    }
}

#[cfg(target_os = "macos")]
fn mac_permission_help_open_targets() -> &'static [&'static [&'static str]] {
    &[
        &["x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"],
        &["x-apple.systempreferences:com.apple.preference.security"],
        &["-b", "com.apple.systempreferences"],
    ]
}

#[cfg(target_os = "macos")]
pub(super) fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
}

#[cfg(target_os = "macos")]
pub(super) fn mac_uninstall(item: &ManagedAppDto) -> AppResult<()> {
    if item.path.trim().is_empty() {
        return Err(app_error(
            AppManagerErrorCode::UninstallInvalidPath,
            "应用路径为空",
        ));
    }
    if !Path::new(item.path.as_str()).exists() {
        return Err(app_error(
            AppManagerErrorCode::UninstallNotFound,
            "应用路径不存在，无法卸载",
        ));
    }

    let script = format!(
        "tell application \"Finder\" to delete POSIX file \"{}\"",
        applescript_escape(item.path.as_str())
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .with_context(|| format!("调用系统卸载失败: {}", item.path))
        .with_code(
            AppManagerErrorCode::UninstallFailed.as_str(),
            "调用系统卸载失败",
        )
        .with_ctx("appPath", item.path.clone())
        .with_ctx("appName", item.name.clone())?;
    if status.success() {
        return Ok(());
    }

    Err(
        app_error(AppManagerErrorCode::UninstallFailed, "系统卸载执行失败")
            .with_context("status", status.to_string())
            .with_context("appPath", item.path.clone())
            .with_context("appName", item.name.clone()),
    )
}

#[cfg(target_os = "windows")]
pub(super) fn windows_uninstall(item: &ManagedAppDto) -> AppResult<()> {
    let entries = windows_list_uninstall_entries();
    let matched = windows_find_best_uninstall_entry(
        item.name.as_str(),
        Path::new(item.path.as_str()),
        entries.as_slice(),
    );

    if let Some(entry) = matched {
        let command = entry
            .quiet_uninstall_string
            .as_deref()
            .or(entry.uninstall_string.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if let Some(command) = command {
            if windows_execute_uninstall_command(command) {
                return Ok(());
            }

            tracing::warn!(
                event = "app_manager_windows_uninstall_command_failed",
                app_name = item.name.as_str(),
                command = command
            );
        }
    }

    open_with_command(
        "cmd",
        &["/C", "start", "", "ms-settings:appsfeatures"],
        AppManagerErrorCode::UninstallFailed,
    )
}

#[cfg(target_os = "windows")]
pub(super) fn windows_execute_uninstall_command(command: &str) -> bool {
    let direct_status = Command::new("cmd").args(["/C", command]).status();
    if direct_status.as_ref().is_ok_and(|status| status.success()) {
        return true;
    }

    let escaped = windows_powershell_escape(command);
    let script = format!(
        "$cmd='{}'; Start-Process -FilePath 'cmd.exe' -ArgumentList '/C', $cmd -Verb RunAs",
        escaped
    );
    let elevated_status = Command::new("powershell")
        .args(["-NoProfile", "-Command", script.as_str()])
        .status();
    elevated_status
        .as_ref()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    use super::mac_permission_help_open_targets;

    #[test]
    #[cfg(target_os = "macos")]
    fn permission_help_targets_include_full_disk_access_entries() {
        let targets = mac_permission_help_open_targets();
        assert!(!targets.is_empty());
        assert!(targets.iter().any(|args| {
            *args == ["x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles"]
        }));
        assert!(targets
            .iter()
            .any(|args| *args == ["-b", "com.apple.systempreferences"]));
    }
}

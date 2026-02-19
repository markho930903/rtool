use super::*;

fn delete_path_with_mode(path: &Path, delete_mode: &str) -> AppResult<()> {
    match delete_mode {
        "trash" => move_path_to_trash(path),
        "permanent" => {
            if path.is_dir() {
                fs::remove_dir_all(path).map_err(|error| {
                    AppError::new("app_manager_cleanup_delete_failed", "删除目录失败")
                        .with_detail(error.to_string())
                })
            } else {
                fs::remove_file(path).map_err(|error| {
                    AppError::new("app_manager_cleanup_delete_failed", "删除文件失败")
                        .with_detail(error.to_string())
                })
            }
        }
        _ => Err(AppError::new(
            "app_manager_cleanup_mode_invalid",
            "不支持的删除模式",
        )),
    }
}

#[cfg(target_os = "macos")]
fn move_path_to_trash(path: &Path) -> AppResult<()> {
    let path_value = path.to_string_lossy().to_string();
    let script = format!(
        "tell application \"Finder\" to delete POSIX file \"{}\"",
        applescript_escape(path_value.as_str())
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "移入废纸篓失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "移入废纸篓失败")
            .with_detail(format!("status={status}")),
    )
}

#[cfg(target_os = "windows")]
fn move_path_to_trash(path: &Path) -> AppResult<()> {
    let escaped = windows_powershell_escape(path.to_string_lossy().as_ref());
    let script = format!(
        "Add-Type -AssemblyName Microsoft.VisualBasic; \
         $path='{}'; \
         if (Test-Path $path) {{ \
           $item = Get-Item -LiteralPath $path -ErrorAction SilentlyContinue; \
           if ($null -ne $item -and $item.PSIsContainer) {{ \
             [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory($path, 'OnlyErrorDialogs', 'SendToRecycleBin'); \
           }} else {{ \
             [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile($path, 'OnlyErrorDialogs', 'SendToRecycleBin'); \
           }} \
         }}",
        escaped
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", script.as_str()])
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "移入回收站失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "移入回收站失败")
            .with_detail(format!("status={status}")),
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn move_path_to_trash(path: &Path) -> AppResult<()> {
    let _ = path;
    Err(AppError::new(
        "app_manager_cleanup_delete_failed",
        "当前平台不支持移入废纸篓",
    ))
}

#[cfg(target_os = "windows")]
fn windows_registry_key_exists(reg_key: &str) -> bool {
    Command::new("reg")
        .args(["query", reg_key])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn windows_delete_registry_key(reg_key: &str) -> AppResult<()> {
    if !windows_registry_key_exists(reg_key) {
        return Err(AppError::new(
            "app_manager_cleanup_not_found",
            "注册表键不存在",
        ));
    }
    let status = Command::new("reg")
        .args(["delete", reg_key, "/f"])
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "删除注册表键失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "删除注册表键失败")
            .with_detail(format!("status={status}")),
    )
}

#[cfg(target_os = "windows")]
fn windows_delete_registry_value(spec: &str) -> AppResult<()> {
    let (reg_key, value_name) = spec
        .rsplit_once("::")
        .ok_or_else(|| AppError::new("app_manager_cleanup_path_invalid", "注册表值路径格式无效"))?;
    if !windows_registry_value_exists(reg_key, value_name) {
        return Err(AppError::new(
            "app_manager_cleanup_not_found",
            "注册表值不存在",
        ));
    }
    let status = Command::new("reg")
        .args(["delete", reg_key, "/v", value_name, "/f"])
        .status()
        .map_err(|error| {
            AppError::new("app_manager_cleanup_delete_failed", "删除注册表值失败")
                .with_detail(error.to_string())
        })?;
    if status.success() {
        return Ok(());
    }
    Err(
        AppError::new("app_manager_cleanup_delete_failed", "删除注册表值失败")
            .with_detail(format!("status={status}")),
    )
}

fn delete_residue_item(item_kind: &str, item_path: &str, delete_mode: &str) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        match item_kind {
            "registry_key" => return windows_delete_registry_key(item_path),
            "registry_value" => return windows_delete_registry_value(item_path),
            _ => {}
        }
    }

    #[cfg(not(target_os = "windows"))]
    if matches!(item_kind, "registry_key" | "registry_value") {
        return Err(AppError::new(
            "app_manager_cleanup_not_supported",
            "当前平台不支持注册表清理",
        ));
    }

    delete_path_with_mode(Path::new(item_path), delete_mode)
}

pub(super) fn execute_cleanup_plan(
    app: &AppHandle,
    app_item: &ManagedAppDto,
    scan_result: &AppManagerResidueScanResultDto,
    input: AppManagerCleanupInputDto,
) -> AppResult<AppManagerCleanupResultDto> {
    let delete_mode = input.delete_mode.to_ascii_lowercase();
    if delete_mode != "trash" && delete_mode != "permanent" {
        return Err(AppError::new(
            "app_manager_cleanup_mode_invalid",
            "删除模式仅支持 trash 或 permanent",
        ));
    }
    let skip_on_error = input.skip_on_error.unwrap_or(true);
    let mut released_size_bytes = 0u64;
    let mut deleted = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if input.include_main_app {
        if app_item.source == "rtool" {
            skipped.push(AppManagerCleanupItemResultDto {
                item_id: "main-app".to_string(),
                path: app_item.path.clone(),
                kind: "main_app".to_string(),
                status: "skipped".to_string(),
                reason_code: "self_uninstall_forbidden".to_string(),
                message: "当前运行中的应用不可在此流程卸载".to_string(),
                size_bytes: app_item.estimated_size_bytes,
            });
        } else {
            let confirmed_fingerprint = input.confirmed_fingerprint.clone().ok_or_else(|| {
                AppError::new("app_manager_fingerprint_missing", "缺少应用确认指纹")
            })?;
            if confirmed_fingerprint != app_item.fingerprint {
                return Err(AppError::new(
                    "app_manager_fingerprint_mismatch",
                    "应用信息已变化，请刷新后重试",
                ));
            }
            match platform_uninstall(app_item) {
                Ok(_) => {
                    released_size_bytes = released_size_bytes
                        .saturating_add(app_item.estimated_size_bytes.unwrap_or(0));
                    deleted.push(AppManagerCleanupItemResultDto {
                        item_id: "main-app".to_string(),
                        path: app_item.path.clone(),
                        kind: "main_app".to_string(),
                        status: "deleted".to_string(),
                        reason_code: "ok".to_string(),
                        message: "主程序卸载流程已执行".to_string(),
                        size_bytes: app_item.estimated_size_bytes,
                    });
                }
                Err(error) => {
                    let detail = error
                        .detail
                        .as_deref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| error.message.clone());
                    failed.push(AppManagerCleanupItemResultDto {
                        item_id: "main-app".to_string(),
                        path: app_item.path.clone(),
                        kind: "main_app".to_string(),
                        status: "failed".to_string(),
                        reason_code: error.code,
                        message: detail,
                        size_bytes: app_item.estimated_size_bytes,
                    });
                    if !skip_on_error {
                        return Err(AppError::new(
                            "app_manager_cleanup_failed",
                            "主程序卸载失败，已中止清理",
                        ));
                    }
                }
            }
        }
    }

    let selected = input
        .selected_item_ids
        .iter()
        .map(|value| value.as_str())
        .collect::<HashSet<_>>();
    for group in &scan_result.groups {
        for item in &group.items {
            if !selected.contains(item.item_id.as_str()) {
                continue;
            }

            if item
                .readonly_reason_code
                .as_deref()
                .is_some_and(|reason| reason == "managed_by_policy")
            {
                skipped.push(AppManagerCleanupItemResultDto {
                    item_id: item.item_id.clone(),
                    path: item.path.clone(),
                    kind: item.kind.clone(),
                    status: "skipped".to_string(),
                    reason_code: "managed_by_policy".to_string(),
                    message: "系统策略托管项，已跳过".to_string(),
                    size_bytes: Some(item.size_bytes),
                });
                continue;
            }

            let is_registry_item = matches!(item.kind.as_str(), "registry_key" | "registry_value");
            if !is_registry_item && !Path::new(item.path.as_str()).exists() {
                skipped.push(AppManagerCleanupItemResultDto {
                    item_id: item.item_id.clone(),
                    path: item.path.clone(),
                    kind: item.kind.clone(),
                    status: "skipped".to_string(),
                    reason_code: "not_found".to_string(),
                    message: "路径不存在，已跳过".to_string(),
                    size_bytes: Some(item.size_bytes),
                });
                continue;
            }

            match delete_residue_item(item.kind.as_str(), item.path.as_str(), delete_mode.as_str())
            {
                Ok(_) => {
                    released_size_bytes = released_size_bytes.saturating_add(item.size_bytes);
                    deleted.push(AppManagerCleanupItemResultDto {
                        item_id: item.item_id.clone(),
                        path: item.path.clone(),
                        kind: item.kind.clone(),
                        status: "deleted".to_string(),
                        reason_code: "ok".to_string(),
                        message: "删除成功".to_string(),
                        size_bytes: Some(item.size_bytes),
                    });
                }
                Err(error) => {
                    let detail = error
                        .detail
                        .as_deref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| error.message.clone());
                    failed.push(AppManagerCleanupItemResultDto {
                        item_id: item.item_id.clone(),
                        path: item.path.clone(),
                        kind: item.kind.clone(),
                        status: "failed".to_string(),
                        reason_code: error.code,
                        message: detail,
                        size_bytes: Some(item.size_bytes),
                    });
                    if !skip_on_error {
                        return Err(AppError::new(
                            "app_manager_cleanup_failed",
                            "残留清理失败，已按配置中止",
                        ));
                    }
                }
            }
        }
    }

    {
        let mut scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache.remove(app_item.id.as_str());
    }
    let _ = app;
    Ok(AppManagerCleanupResultDto {
        app_id: app_item.id.clone(),
        delete_mode,
        released_size_bytes,
        deleted,
        skipped,
        failed,
    })
}

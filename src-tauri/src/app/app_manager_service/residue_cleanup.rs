use super::*;

fn delete_path_with_mode(path: &Path, delete_mode: AppManagerCleanupDeleteMode) -> AppResult<()> {
    match delete_mode {
        AppManagerCleanupDeleteMode::Trash => move_path_to_trash(path),
        AppManagerCleanupDeleteMode::Permanent => {
            if path.is_dir() {
                fs::remove_dir_all(path)
                    .with_context(|| format!("删除目录失败: {}", path.display()))
                    .with_code(
                        AppManagerErrorCode::CleanupDeleteFailed.as_str(),
                        "删除目录失败",
                    )
                    .with_ctx("path", path.display().to_string())
                    .with_ctx("deleteMode", delete_mode.as_str())
            } else {
                fs::remove_file(path)
                    .with_context(|| format!("删除文件失败: {}", path.display()))
                    .with_code(
                        AppManagerErrorCode::CleanupDeleteFailed.as_str(),
                        "删除文件失败",
                    )
                    .with_ctx("path", path.display().to_string())
                    .with_ctx("deleteMode", delete_mode.as_str())
            }
        }
        AppManagerCleanupDeleteMode::Unknown => Err(app_error(
            AppManagerErrorCode::CleanupModeInvalid,
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
        .with_context(|| format!("调用 osascript 失败: {}", path.display()))
        .with_code(
            AppManagerErrorCode::CleanupDeleteFailed.as_str(),
            "移入废纸篓失败",
        )
        .with_ctx("path", path.display().to_string())
        .with_ctx("deleteMode", "trash")?;
    if status.success() {
        return Ok(());
    }
    Err(
        app_error(AppManagerErrorCode::CleanupDeleteFailed, "移入废纸篓失败")
            .with_context("status", status.to_string())
            .with_context("path", path.display().to_string())
            .with_context("deleteMode", "trash"),
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
        .with_context(|| format!("调用 powershell 失败: {}", path.display()))
        .with_code(
            AppManagerErrorCode::CleanupDeleteFailed.as_str(),
            "移入回收站失败",
        )
        .with_ctx("path", path.display().to_string())
        .with_ctx("deleteMode", "trash")?;
    if status.success() {
        return Ok(());
    }
    Err(
        app_error(AppManagerErrorCode::CleanupDeleteFailed, "移入回收站失败")
            .with_context("status", status.to_string())
            .with_context("path", path.display().to_string())
            .with_context("deleteMode", "trash"),
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn move_path_to_trash(path: &Path) -> AppResult<()> {
    let _ = path;
    Err(app_error(
        AppManagerErrorCode::CleanupDeleteFailed,
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
        return Err(app_error(
            AppManagerErrorCode::CleanupNotFound,
            "注册表键不存在",
        ));
    }
    let status = Command::new("reg")
        .args(["delete", reg_key, "/f"])
        .status()
        .with_context(|| format!("删除注册表键失败: {}", reg_key))
        .with_code(
            AppManagerErrorCode::CleanupDeleteFailed.as_str(),
            "删除注册表键失败",
        )
        .with_ctx("registryKey", reg_key.to_string())?;
    if status.success() {
        return Ok(());
    }
    Err(
        app_error(AppManagerErrorCode::CleanupDeleteFailed, "删除注册表键失败")
            .with_context("status", status.to_string())
            .with_context("registryKey", reg_key.to_string()),
    )
}

#[cfg(target_os = "windows")]
fn windows_delete_registry_value(spec: &str) -> AppResult<()> {
    let (reg_key, value_name) = spec.rsplit_once("::").ok_or_else(|| {
        app_error(
            AppManagerErrorCode::CleanupPathInvalid,
            "注册表值路径格式无效",
        )
    })?;
    if !windows_registry_value_exists(reg_key, value_name) {
        return Err(app_error(
            AppManagerErrorCode::CleanupNotFound,
            "注册表值不存在",
        ));
    }
    let status = Command::new("reg")
        .args(["delete", reg_key, "/v", value_name, "/f"])
        .status()
        .with_context(|| format!("删除注册表值失败: {}::{}", reg_key, value_name))
        .with_code(
            AppManagerErrorCode::CleanupDeleteFailed.as_str(),
            "删除注册表值失败",
        )
        .with_ctx("registryKey", reg_key.to_string())
        .with_ctx("registryValue", value_name.to_string())?;
    if status.success() {
        return Ok(());
    }
    Err(
        app_error(AppManagerErrorCode::CleanupDeleteFailed, "删除注册表值失败")
            .with_context("status", status.to_string())
            .with_context("registryKey", reg_key.to_string())
            .with_context("registryValue", value_name.to_string()),
    )
}

fn delete_residue_item(
    item_kind: AppManagerResidueKind,
    item_path: &str,
    delete_mode: AppManagerCleanupDeleteMode,
) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        match item_kind {
            AppManagerResidueKind::RegistryKey => return windows_delete_registry_key(item_path),
            AppManagerResidueKind::RegistryValue => {
                return windows_delete_registry_value(item_path);
            }
            _ => {}
        }
    }

    #[cfg(not(target_os = "windows"))]
    if matches!(
        item_kind,
        AppManagerResidueKind::RegistryKey | AppManagerResidueKind::RegistryValue
    ) {
        return Err(app_error(
            AppManagerErrorCode::CleanupNotSupported,
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
    let delete_mode = input.delete_mode;
    if matches!(delete_mode, AppManagerCleanupDeleteMode::Unknown) {
        return Err(app_error(
            AppManagerErrorCode::CleanupModeInvalid,
            "删除模式仅支持 trash 或 permanent",
        ));
    }
    let skip_on_error = input.skip_on_error.unwrap_or(true);
    let mut released_size_bytes = 0u64;
    let main_app_size_bytes =
        exact_path_size_bytes(resolve_app_size_path(Path::new(app_item.path.as_str())).as_path())
            .or(app_item.estimated_size_bytes);
    let mut deleted = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    if input.include_main_app {
        if app_item.source == AppManagerSource::Rtool {
            skipped.push(AppManagerCleanupItemResultDto {
                item_id: "main-app".to_string(),
                path: app_item.path.clone(),
                kind: AppManagerResidueKind::MainApp,
                status: AppManagerCleanupStatus::Skipped,
                reason_code: AppManagerCleanupReasonCode::SelfUninstallForbidden,
                message: "当前运行中的应用不可在此流程卸载".to_string(),
                size_bytes: main_app_size_bytes,
            });
        } else {
            let confirmed_fingerprint = input.confirmed_fingerprint.clone().ok_or_else(|| {
                app_error(AppManagerErrorCode::FingerprintMissing, "缺少应用确认指纹")
            })?;
            if confirmed_fingerprint != app_item.fingerprint {
                return Err(app_error(
                    AppManagerErrorCode::FingerprintMismatch,
                    "应用信息已变化，请刷新后重试",
                ));
            }
            match platform_uninstall(app_item) {
                Ok(_) => {
                    released_size_bytes =
                        released_size_bytes.saturating_add(main_app_size_bytes.unwrap_or(0));
                    deleted.push(AppManagerCleanupItemResultDto {
                        item_id: "main-app".to_string(),
                        path: app_item.path.clone(),
                        kind: AppManagerResidueKind::MainApp,
                        status: AppManagerCleanupStatus::Deleted,
                        reason_code: AppManagerCleanupReasonCode::Ok,
                        message: "主程序卸载流程已执行".to_string(),
                        size_bytes: main_app_size_bytes,
                    });
                }
                Err(error) => {
                    let detail = error
                        .causes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| error.message.clone());
                    failed.push(AppManagerCleanupItemResultDto {
                        item_id: "main-app".to_string(),
                        path: app_item.path.clone(),
                        kind: AppManagerResidueKind::MainApp,
                        status: AppManagerCleanupStatus::Failed,
                        reason_code: AppManagerCleanupReasonCode::from_error_code(
                            error.code.as_str(),
                        ),
                        message: detail,
                        size_bytes: main_app_size_bytes,
                    });
                    if !skip_on_error {
                        return Err(app_error(
                            AppManagerErrorCode::CleanupFailed,
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
                .is_some_and(|reason| reason == AppReadonlyReasonCode::ManagedByPolicy)
            {
                skipped.push(AppManagerCleanupItemResultDto {
                    item_id: item.item_id.clone(),
                    path: item.path.clone(),
                    kind: item.kind,
                    status: AppManagerCleanupStatus::Skipped,
                    reason_code: AppManagerCleanupReasonCode::ManagedByPolicy,
                    message: "系统策略托管项，已跳过".to_string(),
                    size_bytes: Some(item.size_bytes),
                });
                continue;
            }

            let is_registry_item = matches!(
                item.kind,
                AppManagerResidueKind::RegistryKey | AppManagerResidueKind::RegistryValue
            );
            if !is_registry_item && !Path::new(item.path.as_str()).exists() {
                skipped.push(AppManagerCleanupItemResultDto {
                    item_id: item.item_id.clone(),
                    path: item.path.clone(),
                    kind: item.kind,
                    status: AppManagerCleanupStatus::Skipped,
                    reason_code: AppManagerCleanupReasonCode::NotFound,
                    message: "路径不存在，已跳过".to_string(),
                    size_bytes: Some(item.size_bytes),
                });
                continue;
            }

            match delete_residue_item(item.kind, item.path.as_str(), delete_mode) {
                Ok(_) => {
                    released_size_bytes = released_size_bytes.saturating_add(item.size_bytes);
                    deleted.push(AppManagerCleanupItemResultDto {
                        item_id: item.item_id.clone(),
                        path: item.path.clone(),
                        kind: item.kind,
                        status: AppManagerCleanupStatus::Deleted,
                        reason_code: AppManagerCleanupReasonCode::Ok,
                        message: "删除成功".to_string(),
                        size_bytes: Some(item.size_bytes),
                    });
                }
                Err(error) => {
                    let detail = error
                        .causes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| error.message.clone());
                    failed.push(AppManagerCleanupItemResultDto {
                        item_id: item.item_id.clone(),
                        path: item.path.clone(),
                        kind: item.kind,
                        status: AppManagerCleanupStatus::Failed,
                        reason_code: AppManagerCleanupReasonCode::from_error_code(
                            error.code.as_str(),
                        ),
                        message: detail,
                        size_bytes: Some(item.size_bytes),
                    });
                    if !skip_on_error {
                        return Err(app_error(
                            AppManagerErrorCode::CleanupFailed,
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

use super::*;

pub fn list_managed_apps(
    app: &AppHandle,
    query: AppManagerQueryDto,
) -> AppResult<AppManagerPageDto> {
    let cache = load_or_refresh_index(app, false)?;
    let normalized_keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let normalized_category = query
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let limit = query
        .limit
        .map(|value| value as usize)
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);
    let offset = query
        .cursor
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    let mut filtered: Vec<ManagedAppDto> = cache
        .items
        .iter()
        .filter(|item| {
            if let Some(category) = normalized_category.as_deref() {
                if category == "rtool" && item.source != "rtool" {
                    return false;
                }
                if category == "application" && item.source != "application" {
                    return false;
                }
                if category == "startup" && !item.startup_enabled {
                    return false;
                }
            }
            if let Some(keyword) = normalized_keyword.as_deref() {
                let name = item.name.to_ascii_lowercase();
                let path = item.path.to_ascii_lowercase();
                let publisher = item
                    .publisher
                    .clone()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if !name.contains(keyword)
                    && !path.contains(keyword)
                    && !publisher.contains(keyword)
                {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    filtered.sort_by(|left, right| {
        right
            .startup_enabled
            .cmp(&left.startup_enabled)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.name.cmp(&right.name))
    });

    let total = filtered.len();
    if offset >= total {
        return Ok(AppManagerPageDto {
            items: Vec::new(),
            next_cursor: None,
            indexed_at: cache.indexed_at,
        });
    }

    let end = offset.saturating_add(limit).min(total);
    let next_cursor = if end < total {
        Some(end.to_string())
    } else {
        None
    };
    let items = filtered[offset..end].to_vec();

    Ok(AppManagerPageDto {
        items,
        next_cursor,
        indexed_at: cache.indexed_at,
    })
}

pub fn refresh_managed_apps_index(app: &AppHandle) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, true)?;
    Ok(make_action_result(
        true,
        "app_manager_refreshed",
        "应用索引已刷新",
        Some(format!("count={}", cache.items.len())),
    ))
}

pub fn set_managed_app_startup(
    app: &AppHandle,
    input: AppManagerStartupUpdateInputDto,
) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    if !item.startup_editable {
        return Err(AppError::new(
            "app_manager_startup_read_only",
            "当前应用启动项为只读，无法修改",
        ));
    }

    platform_set_startup(
        item.id.as_str(),
        Path::new(item.path.as_str()),
        input.enabled,
    )?;
    let _ = load_or_refresh_index(app, true)?;

    let message = if input.enabled {
        "已启用开机启动"
    } else {
        "已关闭开机启动"
    };
    Ok(make_action_result(
        true,
        "app_manager_startup_updated",
        message,
        Some(item.name),
    ))
}

pub fn get_managed_app_detail(
    app: &AppHandle,
    query: AppManagerDetailQueryDto,
) -> AppResult<ManagedAppDetailDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == query.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    Ok(build_app_detail(item))
}

pub fn scan_managed_app_residue(
    app: &AppHandle,
    input: AppManagerResidueScanInputDto,
) -> AppResult<AppManagerResidueScanResultDto> {
    cleanup_stale_scan_cache();
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    let result = build_residue_scan_result(&item);
    {
        let mut scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache.insert(
            item.id.clone(),
            ResidueScanCacheEntry {
                refreshed_at: Instant::now(),
                result: result.clone(),
            },
        );
    }
    Ok(result)
}

pub fn cleanup_managed_app_residue(
    app: &AppHandle,
    input: AppManagerCleanupInputDto,
) -> AppResult<AppManagerCleanupResultDto> {
    cleanup_stale_scan_cache();
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    let scan_result = {
        let scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache
            .get(item.id.as_str())
            .map(|entry| entry.result.clone())
            .unwrap_or_else(|| build_residue_scan_result(&item))
    };

    let result = execute_cleanup_plan(app, &item, &scan_result, input)?;
    let _ = load_or_refresh_index(app, true)?;
    Ok(result)
}

pub fn export_managed_app_scan_result(
    app: &AppHandle,
    input: AppManagerExportScanInputDto,
) -> AppResult<AppManagerExportScanResultDto> {
    cleanup_stale_scan_cache();
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    let scan_result = {
        let scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache
            .get(item.id.as_str())
            .map(|entry| entry.result.clone())
            .unwrap_or_else(|| build_residue_scan_result(&item))
    };
    let detail = build_app_detail(item.clone());

    let export_dir = export_root_dir();
    fs::create_dir_all(&export_dir)
        .with_context(|| format!("创建导出目录失败: {}", export_dir.display()))
        .with_code("app_manager_export_dir_failed", "创建导出目录失败")
        .with_ctx("exportDir", export_dir.display().to_string())?;

    let stem = sanitize_file_stem(item.name.as_str());
    let file_name = format!("{}-{}-scan.json", stem, now_unix_millis());
    let file_path = export_dir.join(file_name);
    let payload = serde_json::json!({
        "exportedAt": now_unix_seconds(),
        "app": item,
        "detail": detail,
        "scanResult": scan_result
    });
    let content = serde_json::to_string_pretty(&payload)
        .with_context(|| format!("序列化导出内容失败: app_id={}", input.app_id))
        .with_code("app_manager_export_serialize_failed", "序列化导出内容失败")
        .with_ctx("appId", input.app_id.clone())?;
    fs::write(&file_path, content)
        .with_context(|| format!("写入导出文件失败: {}", file_path.display()))
        .with_code("app_manager_export_write_failed", "写入导出文件失败")
        .with_ctx("appId", input.app_id.clone())
        .with_ctx("filePath", file_path.display().to_string())?;

    Ok(AppManagerExportScanResultDto {
        app_id: input.app_id,
        file_path: file_path.to_string_lossy().to_string(),
        directory_path: export_dir.to_string_lossy().to_string(),
    })
}

pub fn uninstall_managed_app(
    app: &AppHandle,
    input: AppManagerUninstallInputDto,
) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == input.app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    if item.fingerprint != input.confirmed_fingerprint {
        return Err(AppError::new(
            "app_manager_fingerprint_mismatch",
            "应用信息已变化，请刷新后重试",
        ));
    }

    if !item.uninstall_supported {
        return Err(AppError::new(
            "app_manager_uninstall_unsupported",
            "该应用不支持在当前平台直接卸载",
        ));
    }

    if item.source == "rtool" {
        return Err(AppError::new(
            "app_manager_uninstall_self_forbidden",
            "不支持卸载当前运行中的应用",
        ));
    }

    platform_uninstall(&item)?;
    let _ = load_or_refresh_index(app, true)?;

    Ok(make_action_result(
        true,
        "app_manager_uninstall_started",
        "已触发系统卸载流程",
        Some(item.name),
    ))
}

pub fn open_uninstall_help(
    app: &AppHandle,
    app_id: String,
) -> AppResult<AppManagerActionResultDto> {
    let cache = load_or_refresh_index(app, false)?;
    let item = cache
        .items
        .iter()
        .find(|candidate| candidate.id == app_id)
        .cloned()
        .ok_or_else(|| AppError::new("app_manager_not_found", "应用不存在或索引已过期"))?;

    platform_open_uninstall_help(&item)?;
    Ok(make_action_result(
        true,
        "app_manager_uninstall_help_opened",
        "已打开系统卸载入口",
        Some(item.name),
    ))
}

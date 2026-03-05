use super::*;

fn read_cached_scan_result(cache_key: &str) -> Option<AppManagerResidueScanResultDto> {
    let scan_cache = residue_scan_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    scan_cache.get(cache_key).map(|entry| entry.result.clone())
}

pub fn scan_managed_app_residue(
    app: &dyn LauncherHost,
    input: AppManagerResidueScanInputDto,
) -> AppResult<AppManagerResidueScanResultDto> {
    cleanup_stale_scan_cache();
    let scan_mode = input.mode.unwrap_or(AppManagerResidueScanMode::Deep);
    let item = load_indexed_item(app, input.app_id.as_str())?;

    let cache_key = scan_cache_key(item.id.as_str(), scan_mode);
    if let Some(result) = read_cached_scan_result(cache_key.as_str()) {
        return Ok(result);
    }

    let result = build_residue_scan_result(&item, scan_mode);
    {
        let mut scan_cache = residue_scan_cache()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        scan_cache.insert(
            cache_key,
            ResidueScanCacheEntry {
                refreshed_at: Instant::now(),
                result: result.clone(),
            },
        );
    }
    Ok(result)
}

pub fn cleanup_managed_app_residue(
    app: &dyn LauncherHost,
    input: AppManagerCleanupInputDto,
) -> AppResult<AppManagerCleanupResultDto> {
    cleanup_stale_scan_cache();
    let item = load_indexed_item(app, input.app_id.as_str())?;
    let scan_result = load_or_build_deep_scan(&item);

    let result = execute_cleanup_plan(&item, &scan_result, input)?;
    let _ = load_or_refresh_index(app, true)?;
    Ok(result)
}

pub fn export_managed_app_scan_result(
    app: &dyn LauncherHost,
    input: AppManagerExportScanInputDto,
) -> AppResult<AppManagerExportScanResultDto> {
    cleanup_stale_scan_cache();
    let item = load_indexed_item(app, input.app_id.as_str())?;
    let scan_result = load_or_build_deep_scan(&item);
    let detail = build_app_detail(item.clone());

    let export_dir = export_root_dir();
    fs::create_dir_all(&export_dir)
        .with_context(|| format!("创建导出目录失败: {}", export_dir.display()))
        .with_code(
            AppManagerErrorCode::ExportDirFailed.as_str(),
            "创建导出目录失败",
        )
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
        .with_code(
            AppManagerErrorCode::ExportSerializeFailed.as_str(),
            "序列化导出内容失败",
        )
        .with_ctx("appId", input.app_id.clone())?;
    fs::write(&file_path, content)
        .with_context(|| format!("写入导出文件失败: {}", file_path.display()))
        .with_code(
            AppManagerErrorCode::ExportWriteFailed.as_str(),
            "写入导出文件失败",
        )
        .with_ctx("appId", input.app_id.clone())
        .with_ctx("filePath", file_path.display().to_string())?;

    Ok(AppManagerExportScanResultDto {
        app_id: input.app_id,
        file_path: file_path.to_string_lossy().to_string(),
        directory_path: export_dir.to_string_lossy().to_string(),
    })
}

fn load_or_build_deep_scan(item: &ManagedAppDto) -> AppManagerResidueScanResultDto {
    let deep_key = scan_cache_key(item.id.as_str(), AppManagerResidueScanMode::Deep);
    if let Some(result) = read_cached_scan_result(deep_key.as_str()) {
        return result;
    }

    let result = build_residue_scan_result(item, AppManagerResidueScanMode::Deep);
    let mut scan_cache = residue_scan_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match scan_cache.entry(deep_key) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.get().result.clone(),
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(ResidueScanCacheEntry {
                refreshed_at: Instant::now(),
                result: result.clone(),
            });
            result
        }
    }
}

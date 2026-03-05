use super::*;

pub fn refresh_managed_apps_index(app: &dyn LauncherHost) -> AppResult<AppManagerActionResultDto> {
    let meta = refresh_index_with_meta(app, true)?;
    let cache = meta.cache;
    let detail = format!(
        "count={}, revision={}, changed={}",
        cache.items.len(),
        cache.revision,
        meta.changed_count
    );
    Ok(make_action_result(
        true,
        AppManagerActionCode::AppManagerRefreshed,
        "应用索引已刷新",
        Some(detail),
    ))
}

pub fn poll_managed_apps_auto_refresh(
    app: &dyn LauncherHost,
) -> AppResult<Option<AppManagerIndexUpdatedPayloadDto>> {
    let runtime = app_index_runtime();
    let cache = runtime
        .cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let latest_fingerprint = collect_index_source_fingerprint();
    if !cache.source_fingerprint.is_empty() && latest_fingerprint == cache.source_fingerprint {
        return Ok(None);
    }
    let meta = refresh_index_with_meta(app, true)?;
    if !meta.rebuilt {
        return Ok(None);
    }
    let changed_count = if cache.revision == 0 {
        meta.cache.items.len() as u32
    } else {
        meta.changed_count
    };
    if changed_count == 0 {
        return Ok(None);
    }
    Ok(Some(AppManagerIndexUpdatedPayloadDto {
        revision: meta.cache.revision,
        indexed_at: meta.cache.indexed_at,
        changed_count,
        reason: AppManagerIndexUpdateReason::AutoChange,
    }))
}

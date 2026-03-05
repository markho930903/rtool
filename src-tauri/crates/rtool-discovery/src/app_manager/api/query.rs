use super::*;

fn resolve_single_app_size(item: &ManagedAppDto) -> AppManagerResolvedSizeDto {
    let size_resolution = resolve_managed_app_size_path(item);
    let measured = exact_path_size_bytes(size_resolution.path.as_path())
        .map(|size_bytes| (Some(size_bytes), AppManagerSizeAccuracy::Exact))
        .or_else(|| {
            try_get_path_size_bytes(size_resolution.path.as_path())
                .map(|size_bytes| (Some(size_bytes), AppManagerSizeAccuracy::Estimated))
        });

    let (size_bytes, size_accuracy, size_source, size_computed_at) = match measured {
        Some((size_bytes, size_accuracy)) => (
            size_bytes,
            size_accuracy,
            size_resolution.size_source,
            Some(now_unix_seconds()),
        ),
        None => (
            item.size_bytes,
            item.size_accuracy,
            item.size_source,
            item.size_computed_at,
        ),
    };

    AppManagerResolvedSizeDto {
        app_id: item.id.clone(),
        size_bytes,
        size_accuracy,
        size_source,
        size_computed_at,
    }
}

pub fn list_managed_apps(
    app: &dyn LauncherHost,
    query: AppManagerQueryDto,
) -> AppResult<AppManagerPageDto> {
    let cache = load_or_refresh_index(app, false)?;
    let normalized_keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let normalized_category = query.category;
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

    let mut total = 0usize;
    let mut items = Vec::with_capacity(limit);
    for item in &cache.items {
        if !normalized_category.matches_item(item) {
            continue;
        }
        if !item_matches_keyword(item, normalized_keyword.as_deref()) {
            continue;
        }
        if total >= offset && items.len() < limit {
            items.push(item.clone());
        }
        total = total.saturating_add(1);
    }

    if offset >= total {
        return Ok(AppManagerPageDto {
            items: Vec::new(),
            next_cursor: None,
            total_count: total as u64,
            indexed_at: cache.indexed_at,
            revision: cache.revision,
            index_state: cache.index_state,
        });
    }

    let consumed = offset.saturating_add(items.len());
    let next_cursor = if consumed < total {
        Some(consumed.to_string())
    } else {
        None
    };

    Ok(AppManagerPageDto {
        items,
        next_cursor,
        total_count: total as u64,
        indexed_at: cache.indexed_at,
        revision: cache.revision,
        index_state: cache.index_state,
    })
}

pub fn list_managed_apps_snapshot_meta(
    app: &dyn LauncherHost,
) -> AppResult<AppManagerSnapshotMetaDto> {
    let cache = load_or_refresh_index(app, false)?;
    Ok(AppManagerSnapshotMetaDto {
        indexed_at: cache.indexed_at,
        revision: cache.revision,
        total_count: cache.items.len() as u64,
        index_state: cache.index_state,
    })
}

pub fn resolve_managed_app_sizes(
    app: &dyn LauncherHost,
    input: AppManagerResolveSizesInputDto,
) -> AppResult<AppManagerResolveSizesResultDto> {
    let cache = load_or_refresh_index(app, false)?;
    let wanted = input
        .app_ids
        .iter()
        .map(|value| value.as_str())
        .collect::<HashSet<_>>();
    if wanted.is_empty() {
        return Ok(AppManagerResolveSizesResultDto { items: Vec::new() });
    }

    let mut resolved = Vec::with_capacity(wanted.len());
    for item in cache
        .items
        .iter()
        .filter(|candidate| wanted.contains(candidate.id.as_str()))
    {
        resolved.push(resolve_single_app_size(item));
    }

    if !resolved.is_empty() {
        let resolved_by_id = resolved
            .iter()
            .map(|value| (value.app_id.as_str(), value))
            .collect::<HashMap<_, _>>();
        let runtime = app_index_runtime();
        let mut guard = runtime
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for item in &mut guard.items {
            if let Some(value) = resolved_by_id.get(item.id.as_str()) {
                item.size_bytes = value.size_bytes;
                item.size_accuracy = value.size_accuracy;
                item.size_source = value.size_source;
                item.size_computed_at = value.size_computed_at;
                item.fingerprint = fingerprint_for_app(item);
            }
        }
    }

    Ok(AppManagerResolveSizesResultDto { items: resolved })
}

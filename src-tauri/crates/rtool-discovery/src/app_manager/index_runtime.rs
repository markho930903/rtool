use super::*;

#[derive(Debug, Clone)]
pub(super) struct AppIndexCache {
    pub(super) refreshed_at: Option<Instant>,
    pub(super) indexed_at: i64,
    pub(super) items: Vec<ManagedAppDto>,
    pub(super) revision: u64,
    pub(super) source_fingerprint: String,
    pub(super) building: bool,
    pub(super) index_state: AppManagerIndexState,
    pub(super) last_error: Option<String>,
    pub(super) disk_bootstrapped: bool,
}

#[derive(Debug, Clone)]
pub(super) struct AppIndexRefreshMeta {
    pub(super) cache: AppIndexCache,
    pub(super) changed_count: u32,
    pub(super) rebuilt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedAppIndexCache {
    indexed_at: i64,
    revision: u64,
    source_fingerprint: String,
    items: Vec<ManagedAppDto>,
}

#[derive(Debug, Clone)]
pub(super) struct ResidueScanCacheEntry {
    pub(super) refreshed_at: Instant,
    pub(super) result: AppManagerResidueScanResultDto,
}

pub(super) struct AppIndexRuntime {
    pub(super) cache: Mutex<AppIndexCache>,
    pub(super) condvar: Condvar,
}

pub(super) fn path_is_readonly(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.permissions().readonly())
        .unwrap_or(false)
}

pub(super) fn cleanup_stale_scan_cache() {
    let mut cache = residue_scan_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.retain(|_, entry| entry.refreshed_at.elapsed() <= RESIDUE_SCAN_CACHE_TTL);
}

fn count_item_changes(previous: &[ManagedAppDto], next: &[ManagedAppDto]) -> u32 {
    let mut previous_map = HashMap::new();
    for item in previous {
        previous_map.insert(item.id.as_str(), item.fingerprint.as_str());
    }
    let mut changed = 0u32;
    for item in next {
        let key = item.id.as_str();
        let changed_item = previous_map
            .remove(key)
            .is_none_or(|old_fingerprint| old_fingerprint != item.fingerprint.as_str());
        if changed_item {
            changed = changed.saturating_add(1);
        }
    }
    changed.saturating_add(previous_map.len() as u32)
}

fn compare_managed_app_for_list(left: &ManagedAppDto, right: &ManagedAppDto) -> Ordering {
    right
        .startup_enabled
        .cmp(&left.startup_enabled)
        .then_with(|| left.source.sort_rank().cmp(&right.source.sort_rank()))
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.id.cmp(&right.id))
}

pub(super) fn sort_managed_apps_for_list(items: &mut [ManagedAppDto]) {
    items.sort_by(compare_managed_app_for_list);
}

fn try_bootstrap_index_from_disk(app: &dyn LauncherHost, cache: &mut AppIndexCache) {
    if cache.disk_bootstrapped {
        return;
    }
    cache.disk_bootstrapped = true;
    let Ok(app_data_dir) = app.app_data_dir() else {
        return;
    };
    cleanup_stale_index_cache_files(app_data_dir.as_path());
    let path = app_data_dir.join(INDEX_DISK_CACHE_FILE);
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    let Ok(snapshot) = serde_json::from_str::<PersistedAppIndexCache>(&content) else {
        return;
    };
    cache.items = snapshot.items;
    sort_managed_apps_for_list(cache.items.as_mut_slice());
    cache.indexed_at = snapshot.indexed_at;
    cache.revision = snapshot.revision;
    cache.source_fingerprint = snapshot.source_fingerprint;
    cache.index_state = AppManagerIndexState::Ready;
    cache.last_error = None;
    cache.refreshed_at = Some(Instant::now());
}

fn cleanup_stale_index_cache_files(app_data_dir: &Path) {
    let Ok(entries) = fs::read_dir(app_data_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name == INDEX_DISK_CACHE_FILE {
            continue;
        }
        if file_name.starts_with(INDEX_DISK_CACHE_PREFIX) {
            let _ = fs::remove_file(path);
        }
    }
}

fn persist_index_to_disk(app: &dyn LauncherHost, cache: &AppIndexCache) {
    let Ok(app_data_dir) = app.app_data_dir() else {
        return;
    };
    if fs::create_dir_all(&app_data_dir).is_err() {
        return;
    }
    let path = app_data_dir.join(INDEX_DISK_CACHE_FILE);
    let temp_path = app_data_dir.join(format!("{INDEX_DISK_CACHE_FILE}.tmp"));
    let snapshot = PersistedAppIndexCache {
        indexed_at: cache.indexed_at,
        revision: cache.revision,
        source_fingerprint: cache.source_fingerprint.clone(),
        items: cache.items.clone(),
    };
    let Ok(content) = serde_json::to_vec(&snapshot) else {
        return;
    };
    if fs::write(&temp_path, content).is_err() {
        return;
    }
    let _ = fs::rename(temp_path, path);
}

pub(super) fn refresh_index_with_meta(
    app: &dyn LauncherHost,
    force_refresh: bool,
) -> AppResult<AppIndexRefreshMeta> {
    let runtime = app_index_runtime();
    loop {
        let mut guard = runtime
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        try_bootstrap_index_from_disk(app, &mut guard);
        let stale = force_refresh || guard.is_stale();
        if !stale {
            return Ok(AppIndexRefreshMeta {
                cache: guard.clone(),
                changed_count: 0,
                rebuilt: false,
            });
        }

        let source_fingerprint = collect_index_source_fingerprint();
        let fingerprint_unchanged = !force_refresh
            && !source_fingerprint.is_empty()
            && guard.source_fingerprint == source_fingerprint
            && !guard.items.is_empty();
        if fingerprint_unchanged {
            guard.refreshed_at = Some(Instant::now());
            return Ok(AppIndexRefreshMeta {
                cache: guard.clone(),
                changed_count: 0,
                rebuilt: false,
            });
        }

        if guard.building {
            guard = runtime
                .condvar
                .wait(guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            continue;
        }

        guard.building = true;
        guard.index_state = AppManagerIndexState::Building;
        let previous_items = guard.items.clone();
        drop(guard);

        let rebuild_result = build_app_index(app);
        let indexed_at = now_unix_seconds();
        let mut guard = runtime
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.building = false;
        match rebuild_result {
            Ok(items) => {
                let changed_count = count_item_changes(previous_items.as_slice(), items.as_slice());
                let changed = changed_count > 0;
                guard.items = items;
                guard.indexed_at = indexed_at;
                guard.refreshed_at = Some(Instant::now());
                guard.source_fingerprint = source_fingerprint;
                guard.index_state = AppManagerIndexState::Ready;
                guard.last_error = None;
                if changed || guard.revision == 0 {
                    guard.revision = guard.revision.saturating_add(1);
                }
                let cache_snapshot = guard.clone();
                runtime.condvar.notify_all();
                persist_index_to_disk(app, &cache_snapshot);
                return Ok(AppIndexRefreshMeta {
                    cache: cache_snapshot,
                    changed_count,
                    rebuilt: true,
                });
            }
            Err(error) => {
                guard.refreshed_at = Some(Instant::now());
                guard.index_state = AppManagerIndexState::Degraded;
                guard.last_error = Some(error.to_string());
                let cache_snapshot = guard.clone();
                runtime.condvar.notify_all();
                if cache_snapshot.items.is_empty() {
                    return Err(error);
                }
                return Ok(AppIndexRefreshMeta {
                    cache: cache_snapshot,
                    changed_count: 0,
                    rebuilt: false,
                });
            }
        }
    }
}

pub(super) fn load_or_refresh_index(
    app: &dyn LauncherHost,
    force_refresh: bool,
) -> AppResult<AppIndexCache> {
    refresh_index_with_meta(app, force_refresh).map(|value| value.cache)
}

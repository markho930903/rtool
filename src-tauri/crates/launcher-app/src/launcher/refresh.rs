use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TruncationLogLevel {
    Info,
    Warn,
}

static INDEXER_STARTED: OnceLock<AtomicBool> = OnceLock::new();
static INDEXER_STOPPED: OnceLock<AtomicBool> = OnceLock::new();
static INDEX_BUILDING: OnceLock<AtomicBool> = OnceLock::new();
static INDEX_REBUILD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn indexer_started_flag() -> &'static AtomicBool {
    INDEXER_STARTED.get_or_init(|| AtomicBool::new(false))
}

fn indexer_stopped_flag() -> &'static AtomicBool {
    INDEXER_STOPPED.get_or_init(|| AtomicBool::new(false))
}

fn index_building_flag() -> &'static AtomicBool {
    INDEX_BUILDING.get_or_init(|| AtomicBool::new(false))
}

fn index_rebuild_lock() -> &'static Mutex<()> {
    INDEX_REBUILD_LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) fn classify_truncation_log_level(
    settings: &LauncherSearchSettingsRecord,
) -> TruncationLogLevel {
    if is_default_scope_profile(settings) {
        return TruncationLogLevel::Info;
    }
    TruncationLogLevel::Warn
}

fn log_scan_truncation(
    settings: &LauncherSearchSettingsRecord,
    reason: RefreshReason,
    root: &str,
    effective_max_items: usize,
    indexed_items: usize,
) {
    let configured_max_items_per_root = settings.max_items_per_root as usize;
    let max_total_items = settings.max_total_items as usize;
    match classify_truncation_log_level(settings) {
        TruncationLogLevel::Info => {
            tracing::info!(
                event = "launcher_index_scan_truncated_expected",
                root,
                effective_max_items,
                configured_max_items_per_root,
                max_total_items,
                indexed_items,
                reason = reason.as_str()
            );
        }
        TruncationLogLevel::Warn => {
            tracing::warn!(
                event = "launcher_index_scan_truncated_unexpected",
                root,
                effective_max_items,
                configured_max_items_per_root,
                max_total_items,
                indexed_items,
                reason = reason.as_str()
            );
        }
    }
}

pub fn start_background_indexer(db_conn: DbConn) {
    let started = indexer_started_flag();
    let stopped = indexer_stopped_flag();
    if started.swap(true, Ordering::SeqCst) {
        tracing::info!(event = "launcher_indexer_start_skipped_running");
        return;
    }

    stopped.store(false, Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        struct StartedFlagReset<'a> {
            flag: &'a AtomicBool,
        }

        impl Drop for StartedFlagReset<'_> {
            fn drop(&mut self) {
                self.flag.store(false, Ordering::SeqCst);
            }
        }

        let _started_flag_reset = StartedFlagReset { flag: started };
        index_building_flag().store(true, Ordering::SeqCst);
        let initial_result = refresh_index(&db_conn, RefreshReason::Startup).await;
        index_building_flag().store(false, Ordering::SeqCst);
        if let Err(error) = initial_result {
            let error_text = error.to_string();
            let _ = write_meta(&db_conn, INDEX_READY_KEY, "0").await;
            let _ = write_meta(&db_conn, INDEX_LAST_ERROR_KEY, error_text.as_str()).await;
            tracing::warn!(
                event = "launcher_index_initial_build_failed",
                error = error_text
            );
        }

        loop {
            if wait_for_next_refresh(&db_conn, stopped).await {
                break;
            }
            if let Err(error) = refresh_index(&db_conn, RefreshReason::Periodic).await {
                let error_text = error.to_string();
                let _ = write_meta(&db_conn, INDEX_LAST_ERROR_KEY, error_text.as_str()).await;
                tracing::warn!(
                    event = "launcher_index_periodic_refresh_failed",
                    error = error_text
                );
            }
        }
    });

    tracing::info!(event = "launcher_indexer_started");
}

pub fn stop_background_indexer() {
    let stopped = indexer_stopped_flag();
    let started = indexer_started_flag();
    if !started.load(Ordering::SeqCst) {
        return;
    }
    stopped.store(true, Ordering::SeqCst);
}

pub fn get_indexer_runtime_status() -> LauncherRuntimeStatusDto {
    LauncherRuntimeStatusDto {
        started: indexer_started_flag().load(Ordering::SeqCst),
        building: index_building_flag().load(Ordering::SeqCst),
    }
}

pub async fn get_index_status_async(db_conn: &DbConn) -> AppResult<LauncherIndexStatusDto> {
    let ready = read_meta(db_conn, INDEX_READY_KEY)
        .await?
        .as_deref()
        .map(is_truthy_flag)
        .unwrap_or(false);
    let indexed_items = read_meta(db_conn, INDEX_LAST_ITEM_COUNT_KEY)
        .await?
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let indexed_roots = read_meta(db_conn, INDEX_LAST_ROOT_COUNT_KEY)
        .await?
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let last_build_ms = read_meta(db_conn, INDEX_LAST_BUILD_MS_KEY)
        .await?
        .and_then(|value| value.parse().ok());
    let last_duration_ms = read_meta(db_conn, INDEX_LAST_DURATION_MS_KEY)
        .await?
        .and_then(|value| value.parse::<u64>().ok());
    let last_error = read_meta(db_conn, INDEX_LAST_ERROR_KEY)
        .await?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let index_version = read_meta(db_conn, INDEX_VERSION_KEY)
        .await?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| INDEX_VERSION_VALUE.to_string());
    let truncated = read_meta(db_conn, INDEX_LAST_TRUNCATED_KEY)
        .await?
        .as_deref()
        .map(is_truthy_flag)
        .unwrap_or(false);

    let settings = load_or_init_settings(db_conn).await?;
    Ok(LauncherIndexStatusDto {
        ready,
        building: index_building_flag().load(Ordering::SeqCst),
        indexed_items,
        indexed_roots,
        last_build_ms,
        last_duration_ms,
        last_error,
        refresh_interval_secs: settings.refresh_interval_secs,
        index_version,
        truncated,
    })
}

pub async fn rebuild_index_now_async(db_conn: &DbConn) -> AppResult<LauncherRebuildResultDto> {
    let started_at = Instant::now();
    refresh_index(db_conn, RefreshReason::Manual).await?;
    let duration_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
    let status = get_index_status_async(db_conn).await?;
    Ok(LauncherRebuildResultDto {
        success: status.ready,
        duration_ms,
        indexed_items: status.indexed_items,
        indexed_roots: status.indexed_roots,
        truncated: status.truncated,
        ready: status.ready,
    })
}

async fn refresh_index(db_conn: &DbConn, reason: RefreshReason) -> AppResult<()> {
    let _lock_guard = index_rebuild_lock().lock().await;
    index_building_flag().store(true, Ordering::SeqCst);

    let started_at = Instant::now();
    let result = refresh_index_inner(db_conn, reason, started_at).await;
    index_building_flag().store(false, Ordering::SeqCst);
    if let Err(error) = &result {
        let error_text = error.to_string();
        if matches!(reason, RefreshReason::Startup) {
            let _ = write_meta(db_conn, INDEX_READY_KEY, "0").await;
        }
        let _ = write_meta(db_conn, INDEX_LAST_ERROR_KEY, error_text.as_str()).await;
    }
    result
}

async fn refresh_index_inner(
    db_conn: &DbConn,
    reason: RefreshReason,
    started_at: Instant,
) -> AppResult<()> {
    if matches!(reason, RefreshReason::Startup) {
        write_meta(db_conn, INDEX_READY_KEY, "0").await?;
    }

    let settings = load_or_init_settings(db_conn).await?;
    let exclusion_rules = build_exclusion_rules(settings.exclude_patterns.as_slice());
    let scan_token = now_unix_millis().to_string();
    write_meta(db_conn, INDEX_VERSION_KEY, INDEX_VERSION_VALUE).await?;

    let single_system_root_scope = has_single_system_root_scope(&settings);
    let mut indexed_items: usize = 0;
    let mut indexed_roots: u32 = 0;
    let mut truncated = false;
    let mut remaining_total = settings.max_total_items as usize;

    for root in &settings.roots {
        if remaining_total == 0 {
            truncated = true;
            break;
        }

        let root_path = PathBuf::from(root);
        if !root_path.exists() {
            continue;
        }

        let configured_max_items_per_root = settings.max_items_per_root as usize;
        let effective_max_items_per_root = resolve_effective_max_items_per_root(
            configured_max_items_per_root,
            remaining_total,
            single_system_root_scope,
        );
        let effective_max_items = effective_max_items_per_root
            .max(1)
            .min(remaining_total.max(1));

        indexed_roots += 1;
        let scan_root_path = root_path.clone();
        let scan_root = root.clone();
        let scan_rules = exclusion_rules.clone();
        let scan_max_depth = settings.max_scan_depth as usize;
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            scan_index_root_with_rules(
                scan_root_path.as_path(),
                scan_max_depth,
                effective_max_items_per_root,
                remaining_total,
                scan_rules.as_slice(),
                scan_root.as_str(),
                reason,
            )
        })
        .await
        .map_err(|error| {
            AppError::new("launcher_index_scan_join_failed", "启动器索引扫描任务失败")
                .with_source(error)
        })?;

        indexed_items = indexed_items.saturating_add(outcome.entries.len());
        remaining_total = remaining_total.saturating_sub(outcome.entries.len());
        truncated |= outcome.truncated;
        if outcome.truncated {
            log_scan_truncation(
                &settings,
                reason,
                root.as_str(),
                effective_max_items,
                indexed_items,
            );
        }

        upsert_entries_batched(db_conn, outcome.entries.as_slice(), scan_token.as_str()).await?;
        delete_stale_entries_for_root(db_conn, root.as_str(), scan_token.as_str()).await?;
    }

    purge_removed_roots(db_conn, settings.roots.as_slice()).await?;

    let duration_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
    write_meta(db_conn, INDEX_READY_KEY, "1").await?;
    write_meta(
        db_conn,
        INDEX_LAST_BUILD_MS_KEY,
        now_unix_millis().to_string().as_str(),
    )
    .await?;
    write_meta(
        db_conn,
        INDEX_LAST_DURATION_MS_KEY,
        duration_ms.to_string().as_str(),
    )
    .await?;
    write_meta(
        db_conn,
        INDEX_LAST_ITEM_COUNT_KEY,
        indexed_items.to_string().as_str(),
    )
    .await?;
    write_meta(
        db_conn,
        INDEX_LAST_ROOT_COUNT_KEY,
        indexed_roots.to_string().as_str(),
    )
    .await?;
    write_meta(
        db_conn,
        INDEX_LAST_TRUNCATED_KEY,
        if truncated { "1" } else { "0" },
    )
    .await?;
    write_meta(db_conn, INDEX_LAST_ERROR_KEY, "").await?;

    tracing::info!(
        event = "launcher_index_refresh_finished",
        reason = reason.as_str(),
        indexed_items,
        indexed_roots,
        truncated,
        duration_ms
    );
    Ok(())
}

async fn wait_for_next_refresh(db_conn: &DbConn, stopped: &AtomicBool) -> bool {
    let refresh_interval_secs = load_or_init_settings(db_conn)
        .await
        .map(|value| value.refresh_interval_secs)
        .unwrap_or(DEFAULT_REFRESH_INTERVAL_SECS);
    let target_ms = i64::from(refresh_interval_secs).saturating_mul(1000);
    let poll_ms = 1_000_i64;
    let mut elapsed = 0_i64;
    while elapsed < target_ms {
        if stopped.load(Ordering::SeqCst) {
            return true;
        }
        sleep(Duration::from_millis(poll_ms as u64)).await;
        elapsed += poll_ms;
    }
    stopped.load(Ordering::SeqCst)
}

fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

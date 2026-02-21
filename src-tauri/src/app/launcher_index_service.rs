use crate::app::icon_service::{resolve_builtin_icon, resolve_file_type_icon};
use crate::core::AppResult;
use crate::core::i18n::t;
use crate::core::models::{LauncherActionDto, LauncherItemDto};
use crate::infrastructure::db::DbPool;
use rusqlite::{OptionalExtension, params};
use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const INDEX_READY_KEY: &str = "launcher.index.ready";
const INDEX_LAST_BUILD_MS_KEY: &str = "launcher.index.lastBuildMs";
const INDEX_VERSION_KEY: &str = "launcher.index.version";
const INDEX_LAST_ERROR_KEY: &str = "launcher.index.lastError";
const INDEX_VERSION_VALUE: &str = "1";
const INDEX_SCAN_DEPTH: usize = 8;
const INDEX_MAX_ITEMS_PER_ROOT: usize = 20_000;
const BACKGROUND_REFRESH_INTERVAL: Duration = Duration::from_secs(300);
const BACKGROUND_REFRESH_POLL_INTERVAL: Duration = Duration::from_secs(1);
const QUERY_OVERSCAN_FACTOR: usize = 4;
static INDEXER_STARTED: OnceLock<AtomicBool> = OnceLock::new();
static INDEXER_STOPPED: OnceLock<AtomicBool> = OnceLock::new();

fn indexer_started_flag() -> &'static AtomicBool {
    INDEXER_STARTED.get_or_init(|| AtomicBool::new(false))
}

fn indexer_stopped_flag() -> &'static AtomicBool {
    INDEXER_STOPPED.get_or_init(|| AtomicBool::new(false))
}

#[derive(Debug, Clone)]
struct LauncherIndexEntry {
    path: String,
    kind: IndexedEntryKind,
    name: String,
    parent: String,
    ext: Option<String>,
    mtime: Option<i64>,
    size: Option<i64>,
    source_root: String,
    searchable_text: String,
}

#[derive(Debug, Clone)]
pub struct IndexedSearchResult {
    pub items: Vec<LauncherItemDto>,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexedEntryKind {
    Directory,
    File,
}

impl IndexedEntryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::File => "file",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("directory") {
            return Some(Self::Directory);
        }
        if value.eq_ignore_ascii_case("file") {
            return Some(Self::File);
        }
        None
    }
}

pub fn start_background_indexer(db_pool: DbPool) {
    let started = indexer_started_flag();
    let stopped = indexer_stopped_flag();
    if started.swap(true, Ordering::SeqCst) {
        return;
    }
    stopped.store(false, Ordering::SeqCst);

    let spawn_result = thread::Builder::new()
        .name("launcher-indexer".to_string())
        .spawn(move || {
            if let Err(error) = refresh_index(&db_pool, true) {
                let _ = write_meta(&db_pool, INDEX_READY_KEY, "0");
                let _ = write_meta(&db_pool, INDEX_LAST_ERROR_KEY, error.to_string().as_str());
                tracing::warn!(
                    event = "launcher_index_initial_build_failed",
                    error = error.to_string()
                );
            }

            loop {
                if wait_for_next_refresh(stopped) {
                    break;
                }
                if let Err(error) = refresh_index(&db_pool, false) {
                    let _ = write_meta(&db_pool, INDEX_LAST_ERROR_KEY, error.to_string().as_str());
                    tracing::warn!(
                        event = "launcher_index_incremental_refresh_failed",
                        error = error.to_string()
                    );
                }
            }
        });

    if let Err(error) = spawn_result {
        started.store(false, Ordering::SeqCst);
        tracing::error!(
            event = "launcher_indexer_spawn_failed",
            error = error.to_string()
        );
        return;
    }

    tracing::info!(event = "launcher_indexer_started");
}

pub fn stop_background_indexer() {
    let started = indexer_started_flag();
    let stopped = indexer_stopped_flag();
    if !started.load(Ordering::SeqCst) {
        return;
    }
    stopped.store(true, Ordering::SeqCst);
    started.store(false, Ordering::SeqCst);
}

pub fn search_indexed_items(
    app: &AppHandle,
    db_pool: &DbPool,
    normalized_query: &str,
    locale: &str,
    limit: usize,
) -> AppResult<IndexedSearchResult> {
    let ready = read_index_ready(db_pool)?;
    let limit = limit.max(1);
    let candidate_limit = (limit * QUERY_OVERSCAN_FACTOR).clamp(limit, 800);
    let pattern = if normalized_query.is_empty() {
        String::new()
    } else {
        format!("%{}%", escape_like_pattern(normalized_query))
    };

    let conn = db_pool.get()?;
    let mut statement = conn.prepare(
        r#"
        SELECT path, kind, name, parent
        FROM launcher_index_entries
        WHERE (?1 = '' OR searchable_text LIKE ?2 ESCAPE '\')
        ORDER BY
            CASE kind
                WHEN 'directory' THEN 0
                WHEN 'file' THEN 1
                ELSE 2
            END ASC,
            name COLLATE NOCASE ASC,
            path COLLATE NOCASE ASC
        LIMIT ?3
        "#,
    )?;

    let mut rows = statement.query(params![normalized_query, pattern, candidate_limit as i64])?;
    let mut items = Vec::new();
    while let Some(row) = rows.next()? {
        let path: String = row.get(0)?;
        let kind_raw: String = row.get(1)?;
        let mut title: String = row.get(2)?;
        let subtitle: String = row.get(3)?;
        if title.trim().is_empty() {
            title = path.clone();
        }

        let Some(kind) = IndexedEntryKind::from_db(&kind_raw) else {
            tracing::warn!(
                event = "launcher_index_unknown_entry_kind",
                kind = kind_raw.as_str(),
                path = path.as_str()
            );
            continue;
        };
        let subtitle = if subtitle.trim().is_empty() {
            path.clone()
        } else {
            subtitle
        };

        let item = match kind {
            IndexedEntryKind::Directory => {
                let icon = resolve_builtin_icon("i-noto:file-folder");
                LauncherItemDto {
                    id: stable_id("dir", path.as_str()),
                    title,
                    subtitle,
                    category: "directory".to_string(),
                    source: Some(t(locale, "launcher.source.directory")),
                    shortcut: None,
                    score: 0,
                    icon_kind: icon.kind,
                    icon_value: icon.value,
                    action: LauncherActionDto::OpenDirectory { path },
                }
            }
            IndexedEntryKind::File => {
                let path_buf = PathBuf::from(path.as_str());
                let icon = resolve_file_type_icon(app, path_buf.as_path());
                LauncherItemDto {
                    id: stable_id("file", path.as_str()),
                    title,
                    subtitle,
                    category: "file".to_string(),
                    source: Some(t(locale, "launcher.source.file")),
                    shortcut: None,
                    score: 0,
                    icon_kind: icon.kind,
                    icon_value: icon.value,
                    action: LauncherActionDto::OpenFile { path },
                }
            }
        };

        items.push(item);
    }

    Ok(IndexedSearchResult { items, ready })
}

fn refresh_index(db_pool: &DbPool, startup_build: bool) -> AppResult<()> {
    if startup_build {
        write_meta(db_pool, INDEX_READY_KEY, "0")?;
    }

    let roots = launcher_index_roots();
    let roots_count = roots.len();
    let scan_token = now_unix_millis().to_string();
    let mut conn = db_pool.get()?;
    let transaction = conn.transaction()?;

    write_meta_tx(&transaction, INDEX_VERSION_KEY, INDEX_VERSION_VALUE)?;
    for root in roots {
        let entries = scan_index_root(
            root.as_path(),
            INDEX_SCAN_DEPTH,
            INDEX_MAX_ITEMS_PER_ROOT,
            root.to_string_lossy().as_ref(),
        );
        upsert_entries(&transaction, entries.as_slice(), scan_token.as_str())?;
        transaction.execute(
            "DELETE FROM launcher_index_entries
             WHERE source_root = ?1
               AND COALESCE(scan_token, '') <> ?2",
            params![root.to_string_lossy().to_string(), scan_token],
        )?;
    }

    write_meta_tx(&transaction, INDEX_READY_KEY, "1")?;
    write_meta_tx(
        &transaction,
        INDEX_LAST_BUILD_MS_KEY,
        now_unix_millis().to_string().as_str(),
    )?;
    write_meta_tx(&transaction, INDEX_LAST_ERROR_KEY, "")?;
    transaction.commit()?;

    tracing::debug!(
        event = "launcher_index_refresh_finished",
        startup_build,
        roots_count
    );
    Ok(())
}

fn wait_for_next_refresh(stopped: &AtomicBool) -> bool {
    let target_ms = i64::try_from(BACKGROUND_REFRESH_INTERVAL.as_millis()).unwrap_or(i64::MAX);
    let poll_ms = i64::try_from(BACKGROUND_REFRESH_POLL_INTERVAL.as_millis()).unwrap_or(1000);
    let mut elapsed = 0_i64;
    while elapsed < target_ms {
        if stopped.load(Ordering::SeqCst) {
            return true;
        }
        thread::sleep(BACKGROUND_REFRESH_POLL_INTERVAL);
        elapsed += poll_ms;
    }
    stopped.load(Ordering::SeqCst)
}

fn upsert_entries(
    transaction: &rusqlite::Transaction<'_>,
    entries: &[LauncherIndexEntry],
    scan_token: &str,
) -> AppResult<()> {
    for entry in entries {
        transaction.execute(
            r#"
            INSERT INTO launcher_index_entries (
                path,
                kind,
                name,
                parent,
                ext,
                mtime,
                size,
                source_root,
                searchable_text,
                scan_token
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(path) DO UPDATE SET
                kind = excluded.kind,
                name = excluded.name,
                parent = excluded.parent,
                ext = excluded.ext,
                mtime = excluded.mtime,
                size = excluded.size,
                source_root = excluded.source_root,
                searchable_text = excluded.searchable_text,
                scan_token = excluded.scan_token
            "#,
            params![
                entry.path,
                entry.kind.as_str(),
                entry.name,
                entry.parent,
                entry.ext,
                entry.mtime,
                entry.size,
                entry.source_root,
                entry.searchable_text,
                scan_token,
            ],
        )?;
    }
    Ok(())
}

fn scan_index_root(
    root: &Path,
    max_depth: usize,
    max_items: usize,
    source_root: &str,
) -> Vec<LauncherIndexEntry> {
    if !root.exists() {
        return Vec::new();
    }

    let mut queue = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0usize));

    let mut entries = Vec::new();
    while let Some((current_dir, depth)) = queue.pop_front() {
        if entries.len() >= max_items {
            tracing::warn!(
                event = "launcher_index_scan_truncated",
                root = %root.to_string_lossy(),
                max_items
            );
            break;
        }

        let dir_entries = match fs::read_dir(&current_dir) {
            Ok(dir_entries) => dir_entries,
            Err(error) => {
                tracing::debug!(
                    event = "launcher_index_scan_read_dir_failed",
                    dir = %current_dir.to_string_lossy(),
                    error = error.to_string()
                );
                continue;
            }
        };

        for dir_entry in dir_entries.flatten() {
            if entries.len() >= max_items {
                break;
            }

            let path = dir_entry.path();
            if is_hidden(path.as_path()) {
                continue;
            }

            let file_type = match dir_entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    tracing::debug!(
                        event = "launcher_index_scan_file_type_failed",
                        path = %path.to_string_lossy(),
                        error = error.to_string()
                    );
                    continue;
                }
            };

            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                if let Some(entry) =
                    build_index_entry(path.as_path(), IndexedEntryKind::Directory, source_root)
                {
                    entries.push(entry);
                }
                if depth < max_depth {
                    queue.push_back((path, depth + 1));
                }
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            if let Some(entry) =
                build_index_entry(path.as_path(), IndexedEntryKind::File, source_root)
            {
                entries.push(entry);
            }
        }
    }

    entries
}

fn build_index_entry(
    path: &Path,
    kind: IndexedEntryKind,
    source_root: &str,
) -> Option<LauncherIndexEntry> {
    let path_value = path.to_string_lossy().to_string();
    if path_value.trim().is_empty() {
        return None;
    }

    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| path_value.clone());
    let parent = path
        .parent()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path_value.clone());
    let ext = if matches!(kind, IndexedEntryKind::File) {
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
    } else {
        None
    };

    let metadata = fs::metadata(path).ok();
    let mtime = metadata
        .as_ref()
        .and_then(|value| value.modified().ok())
        .and_then(system_time_to_unix_millis);
    let size = metadata
        .as_ref()
        .map(|value| value.len())
        .and_then(|value| i64::try_from(value).ok());

    let searchable_text = normalize_query(
        format!(
            "{} {} {} {}",
            name,
            parent,
            path_value,
            ext.clone().unwrap_or_default()
        )
        .as_str(),
    );

    Some(LauncherIndexEntry {
        path: path_value,
        kind,
        name,
        parent,
        ext,
        mtime,
        size,
        source_root: source_root.to_string(),
        searchable_text,
    })
}

fn read_index_ready(db_pool: &DbPool) -> AppResult<bool> {
    let value = read_meta(db_pool, INDEX_READY_KEY)?;
    Ok(value
        .as_deref()
        .map(|value| matches!(value, "1" | "true" | "TRUE"))
        .unwrap_or(false))
}

fn read_meta(db_pool: &DbPool, key: &str) -> AppResult<Option<String>> {
    let conn = db_pool.get()?;
    conn.query_row(
        "SELECT value FROM launcher_index_meta WHERE key = ?1 LIMIT 1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(Into::into)
}

fn write_meta(db_pool: &DbPool, key: &str, value: &str) -> AppResult<()> {
    let conn = db_pool.get()?;
    conn.execute(
        "INSERT INTO launcher_index_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn write_meta_tx(transaction: &rusqlite::Transaction<'_>, key: &str, value: &str) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO launcher_index_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn launcher_index_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = home_dir() {
        roots.push(home.join("Desktop"));
        roots.push(home.join("Documents"));
        roots.push(home.join("Downloads"));
    }
    roots
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn stable_id(prefix: &str, input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{prefix}.{:x}", hasher.finish())
}

fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

fn system_time_to_unix_millis(value: SystemTime) -> Option<i64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn normalize_query(value: &str) -> String {
    value.trim().to_lowercase()
}

fn escape_like_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
#[path = "../../tests/app/launcher_index_service_tests.rs"]
mod tests;

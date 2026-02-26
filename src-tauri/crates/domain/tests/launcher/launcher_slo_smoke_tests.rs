use super::*;
use crate::host::{AppPackageInfo, LauncherHost, LauncherWindow};
use foundation::db::{DbConn, init_db, open_db};
use foundation::models::ClipboardWindowModeAppliedDto;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

struct SmokeHost {
    locale: String,
    app_data_dir: PathBuf,
}

struct SmokeWindow;

impl LauncherWindow for SmokeWindow {
    fn show(&self) -> foundation::AppResult<()> {
        Ok(())
    }

    fn set_focus(&self) -> foundation::AppResult<()> {
        Ok(())
    }
}

impl LauncherHost for SmokeHost {
    fn emit(&self, _event: &str, _payload: Value) -> foundation::AppResult<()> {
        Ok(())
    }

    fn get_webview_window(&self, _label: &str) -> Option<Box<dyn LauncherWindow>> {
        Some(Box::new(SmokeWindow))
    }

    fn app_data_dir(&self) -> foundation::AppResult<PathBuf> {
        Ok(self.app_data_dir.clone())
    }

    fn package_info(&self) -> AppPackageInfo {
        AppPackageInfo {
            name: "rtool-tests".to_string(),
            version: "0.0.0".to_string(),
        }
    }

    fn resolved_locale(&self) -> Option<String> {
        Some(self.locale.clone())
    }

    fn apply_clipboard_window_mode(
        &self,
        compact: bool,
        _source: &str,
    ) -> foundation::AppResult<ClipboardWindowModeAppliedDto> {
        Ok(ClipboardWindowModeAppliedDto {
            compact,
            applied_width_logical: 480.0,
            applied_height_logical: 360.0,
            scale_factor: 1.0,
        })
    }
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rtool-{prefix}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn unique_temp_db_path(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_millis();
    std::env::temp_dir().join(format!("rtool-{prefix}-{}-{now}.db", std::process::id()))
}

async fn setup_temp_db(prefix: &str) -> (DbConn, PathBuf) {
    let path = unique_temp_db_path(prefix);
    let conn = open_db(path.as_path()).await.expect("open db");
    init_db(&conn).await.expect("init db");
    (conn, path)
}

fn percentile(samples: &[u64], ratio: f64) -> u64 {
    if samples.is_empty() {
        return 0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = ((sorted.len() as f64) * ratio).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len().saturating_sub(1));
    sorted[index]
}

async fn seed_fixture_entries(db_conn: &DbConn, root: &Path, count: usize) {
    let root_text = root.to_string_lossy().to_string();
    let tx = db_conn.transaction().await.expect("create tx");
    for index in 0..count {
        let path = root.join(format!("alpha-tool-{index}.txt"));
        let path_text = path.to_string_lossy().to_string();
        let name = format!("alpha-tool-{index}");
        let searchable_text =
            normalize_query(format!("{name} {root_text} {path_text} txt file").as_str());
        tx.execute(
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
            ) VALUES (?1, 'file', ?2, ?3, 'txt', NULL, NULL, ?4, ?5, 'smoke')
            "#,
            (
                path_text.as_str(),
                name.as_str(),
                root_text.as_str(),
                root_text.as_str(),
                searchable_text.as_str(),
            ),
        )
        .await
        .expect("insert fixture row");
    }
    tx.commit().await.expect("commit fixture");
    write_meta(db_conn, INDEX_READY_KEY, "1")
        .await
        .expect("mark index ready");
}

#[tokio::test]
#[ignore = "SLO smoke test (manual run): cargo test -p launcher-app launcher_search_index_slo_smoke_should_meet_latency_targets -- --ignored --nocapture"]
async fn launcher_search_index_slo_smoke_should_meet_latency_targets() {
    let fixture_dir = create_temp_dir("launcher-slo-fixture");
    let icon_cache_dir = create_temp_dir("launcher-slo-icons");
    let (db_conn, db_path) = setup_temp_db("launcher-slo").await;

    seed_fixture_entries(&db_conn, fixture_dir.as_path(), 3_000).await;

    let host = SmokeHost {
        locale: "en-US".to_string(),
        app_data_dir: icon_cache_dir.clone(),
    };

    for _ in 0..10 {
        let warmup = search_indexed_items_async(&host, &db_conn, "alpha", "en-US", 60)
            .await
            .expect("warmup search");
        assert!(warmup.ready);
    }

    let mut durations = Vec::new();
    for _ in 0..200 {
        let started_at = Instant::now();
        let result = search_indexed_items_async(&host, &db_conn, "alpha", "en-US", 60)
            .await
            .expect("search");
        let elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        assert!(result.ready);
        assert!(!result.items.is_empty());
        durations.push(elapsed_ms);
    }

    let p95 = percentile(durations.as_slice(), 0.95);
    let p99 = percentile(durations.as_slice(), 0.99);
    eprintln!("launcher indexed search smoke latency: p95={p95}ms p99={p99}ms");
    assert!(
        p95 <= 120,
        "launcher indexed search p95 exceeded SLO: {p95}ms > 120ms"
    );
    assert!(
        p99 <= 250,
        "launcher indexed search p99 exceeded SLO: {p99}ms > 250ms"
    );

    let _ = fs::remove_file(db_path);
    let _ = fs::remove_dir_all(fixture_dir);
    let _ = fs::remove_dir_all(icon_cache_dir);
}

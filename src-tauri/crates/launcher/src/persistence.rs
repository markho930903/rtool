use super::*;

pub(super) async fn read_index_ready(db_conn: &DbConn) -> AppResult<bool> {
    let value = read_meta(db_conn, INDEX_READY_KEY).await?;
    Ok(value.as_deref().map(is_truthy_flag).unwrap_or(false))
}

pub(super) async fn read_meta(db_conn: &DbConn, key: &str) -> DbResult<Option<String>> {
    let mut rows = db_conn
        .query(
            "SELECT value FROM launcher_index_meta WHERE key = ?1 LIMIT 1",
            [key],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        return Ok(Some(row.get::<String>(0)?));
    }
    Ok(None)
}

pub(super) async fn write_meta(db_conn: &DbConn, key: &str, value: &str) -> DbResult<()> {
    db_conn
        .execute(
            "INSERT INTO launcher_index_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            (key, value),
        )
        .await?;
    Ok(())
}

pub(super) async fn upsert_entries_batched(
    db_conn: &DbConn,
    entries: &[LauncherIndexEntry],
    scan_token: &str,
) -> DbResult<()> {
    const UPSERT_BATCH_SIZE: usize = 2_000;
    for chunk in entries.chunks(UPSERT_BATCH_SIZE) {
        let transaction = db_conn.transaction().await?;
        for entry in chunk {
            transaction
                .execute(
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
                    (
                        entry.path.as_str(),
                        entry.kind.as_str(),
                        entry.name.as_str(),
                        entry.parent.as_str(),
                        entry.ext.as_deref(),
                        entry.mtime,
                        entry.size,
                        entry.source_root.as_str(),
                        entry.searchable_text.as_str(),
                        scan_token,
                    ),
                )
                .await?;
        }
        transaction.commit().await?;
    }
    Ok(())
}

pub(super) async fn delete_stale_entries_for_root(
    db_conn: &DbConn,
    root: &str,
    scan_token: &str,
) -> DbResult<()> {
    db_conn
        .execute(
            "DELETE FROM launcher_index_entries
         WHERE source_root = ?1
           AND COALESCE(scan_token, '') <> ?2",
            (root, scan_token),
        )
        .await?;
    Ok(())
}

pub(super) async fn purge_removed_roots(db_conn: &DbConn, roots: &[String]) -> DbResult<()> {
    if roots.is_empty() {
        db_conn
            .execute("DELETE FROM launcher_index_entries", ())
            .await?;
        return Ok(());
    }

    let placeholders = (1..=roots.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql =
        format!("DELETE FROM launcher_index_entries WHERE source_root NOT IN ({placeholders})");
    let root_refs = roots.iter().map(String::as_str).collect::<Vec<_>>();
    db_conn
        .execute(sql.as_str(), params_from_iter(root_refs))
        .await?;
    Ok(())
}

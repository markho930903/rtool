use super::*;

#[derive(Debug, Clone)]
pub struct IndexedSearchResult {
    pub items: Vec<LauncherItemDto>,
    pub ready: bool,
}

pub async fn search_indexed_items_async(
    app: &dyn LauncherHost,
    db_conn: &DbConn,
    normalized_query: &str,
    locale: &str,
    limit: usize,
) -> AppResult<IndexedSearchResult> {
    let ready = read_index_ready(db_conn).await?;
    let limit = limit.max(1);
    let candidate_limit = (limit * QUERY_OVERSCAN_FACTOR).clamp(limit, MAX_QUERY_CANDIDATE_LIMIT);
    let rows = if normalized_query.is_empty() {
        query_index_rows_default(db_conn, candidate_limit as i64).await?
    } else {
        let fts_query = build_fts_query(normalized_query);
        if let Some(fts_query) = fts_query {
            match query_index_rows_fts(db_conn, fts_query.as_str(), candidate_limit as i64).await {
                Ok(rows) => rows,
                Err(error) => {
                    tracing::warn!(
                        event = "launcher_index_fts_query_failed",
                        query = normalized_query,
                        error = error.to_string()
                    );
                    query_index_rows_like(db_conn, normalized_query, candidate_limit as i64).await?
                }
            }
        } else {
            query_index_rows_like(db_conn, normalized_query, candidate_limit as i64).await?
        }
    };

    let mut items = Vec::new();
    for (path, kind_raw, name, parent) in rows {
        let Some(kind) = IndexedEntryKind::from_db(kind_raw.as_str()) else {
            tracing::warn!(
                event = "launcher_index_unknown_entry_kind",
                kind = kind_raw.as_str(),
                path = path.as_str()
            );
            continue;
        };

        let title = if name.trim().is_empty() {
            path.clone()
        } else {
            name
        };
        let subtitle = if parent.trim().is_empty() {
            path.clone()
        } else {
            parent
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

async fn query_index_rows_default(
    db_conn: &DbConn,
    limit: i64,
) -> DbResult<Vec<(String, String, String, String)>> {
    let mut rows = db_conn
        .query(
            r#"
        SELECT path, kind, name, parent
        FROM launcher_index_entries
        ORDER BY
            CASE kind
                WHEN 'directory' THEN 0
                WHEN 'file' THEN 1
                ELSE 2
            END ASC,
            name COLLATE NOCASE ASC,
            path COLLATE NOCASE ASC
        LIMIT ?1
        "#,
            [limit],
        )
        .await?;
    let mut values = Vec::new();
    while let Some(row) = rows.next().await? {
        values.push((
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<String>(2)?,
            row.get::<String>(3)?,
        ));
    }
    Ok(values)
}

async fn query_index_rows_fts(
    db_conn: &DbConn,
    fts_query: &str,
    limit: i64,
) -> DbResult<Vec<(String, String, String, String)>> {
    let mut rows = db_conn
        .query(
            r#"
        SELECT e.path, e.kind, e.name, e.parent
        FROM launcher_index_entries_fts f
        JOIN launcher_index_entries e ON e.rowid = f.rowid
        WHERE launcher_index_entries_fts MATCH ?1
        ORDER BY
            CASE e.kind
                WHEN 'directory' THEN 0
                WHEN 'file' THEN 1
                ELSE 2
            END ASC,
            bm25(launcher_index_entries_fts) ASC,
            e.name COLLATE NOCASE ASC,
            e.path COLLATE NOCASE ASC
        LIMIT ?2
        "#,
            (fts_query, limit),
        )
        .await?;
    let mut values = Vec::new();
    while let Some(row) = rows.next().await? {
        values.push((
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<String>(2)?,
            row.get::<String>(3)?,
        ));
    }
    Ok(values)
}

async fn query_index_rows_like(
    db_conn: &DbConn,
    normalized_query: &str,
    limit: i64,
) -> DbResult<Vec<(String, String, String, String)>> {
    let pattern = format!("%{}%", escape_like_pattern(normalized_query));
    let mut rows = db_conn
        .query(
            r#"
        SELECT path, kind, name, parent
        FROM launcher_index_entries
        WHERE searchable_text LIKE ?1 ESCAPE '\\'
        ORDER BY
            CASE kind
                WHEN 'directory' THEN 0
                WHEN 'file' THEN 1
                ELSE 2
            END ASC,
            name COLLATE NOCASE ASC,
            path COLLATE NOCASE ASC
        LIMIT ?2
        "#,
            (pattern.as_str(), limit),
        )
        .await?;
    let mut values = Vec::new();
    while let Some(row) = rows.next().await? {
        values.push((
            row.get::<String>(0)?,
            row.get::<String>(1)?,
            row.get::<String>(2)?,
            row.get::<String>(3)?,
        ));
    }
    Ok(values)
}

fn build_fts_query(normalized_query: &str) -> Option<String> {
    let terms = normalized_query
        .split_whitespace()
        .map(sanitize_fts_token)
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{term}\"*"))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return None;
    }
    Some(terms.join(" AND "))
}

fn sanitize_fts_token(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .collect::<String>()
}

fn stable_id(prefix: &str, input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{prefix}.{:x}", hasher.finish())
}

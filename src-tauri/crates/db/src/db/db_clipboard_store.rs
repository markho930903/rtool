use super::{CLIPBOARD_LIST_LIMIT_MAX, DbConn, PrunedClipboardItem};
use crate::AppError;
use crate::db_error::DbResult;
use crate::models::{ClipboardFilterDto, ClipboardItemDto};
use libsql::{Row, params};
use std::io::ErrorKind;

fn map_clipboard_item_row(row: &Row) -> DbResult<ClipboardItemDto> {
    Ok(ClipboardItemDto {
        id: row.get(0)?,
        content_key: row.get(1)?,
        item_type: row.get(2)?,
        plain_text: row.get(3)?,
        source_app: row.get(4)?,
        preview_path: row.get(5)?,
        preview_data_url: row.get(6)?,
        created_at: row.get(7)?,
        pinned: row.get::<i64>(8)? == 1,
    })
}

pub async fn insert_clipboard_item(
    conn: &DbConn,
    item: &ClipboardItemDto,
) -> DbResult<ClipboardItemDto> {
    conn.execute(
        "INSERT INTO clipboard_items (id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(content_key) DO UPDATE SET
             item_type = excluded.item_type,
             plain_text = excluded.plain_text,
             source_app = excluded.source_app,
             preview_path = COALESCE(excluded.preview_path, clipboard_items.preview_path),
             preview_data_url = COALESCE(excluded.preview_data_url, clipboard_items.preview_data_url),
             created_at = excluded.created_at",
        params![
            item.id.as_str(),
            item.content_key.as_str(),
            item.item_type.as_str(),
            item.plain_text.as_str(),
            item.source_app.as_deref(),
            item.preview_path.as_deref(),
            item.preview_data_url.as_deref(),
            item.created_at,
            if item.pinned { 1 } else { 0 },
        ],
    )
    .await?;

    let mut rows = conn
        .query(
            "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
             FROM clipboard_items
             WHERE content_key = ?1
             LIMIT 1",
            params![item.content_key.as_str()],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        return map_clipboard_item_row(&row);
    }

    Err(AppError::new("clipboard_upsert_not_found", "写入剪贴板记录后读取失败").into())
}

pub async fn list_clipboard_items(
    conn: &DbConn,
    filter: &ClipboardFilterDto,
) -> DbResult<Vec<ClipboardItemDto>> {
    let limit = filter
        .limit
        .unwrap_or(100)
        .clamp(1, CLIPBOARD_LIST_LIMIT_MAX) as i64;
    let query = filter.query.clone().unwrap_or_default();

    let mut rows = conn
        .query(
            "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
             FROM clipboard_items
             WHERE (?1 = '' OR item_type = ?1)
               AND (?2 = '' OR plain_text LIKE ?3)
               AND (?4 = 0 OR pinned = 1)
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?5",
            params![
                filter.item_type.clone().unwrap_or_default(),
                query,
                format!("%{}%", filter.query.clone().unwrap_or_default()),
                if filter.only_pinned.unwrap_or(false) { 1 } else { 0 },
                limit,
            ],
        )
        .await?;

    let mut items = Vec::new();
    while let Some(row) = rows.next().await? {
        items.push(map_clipboard_item_row(&row)?);
    }

    Ok(items)
}

pub async fn get_clipboard_item(conn: &DbConn, id: &str) -> DbResult<Option<ClipboardItemDto>> {
    let mut rows = conn
        .query(
            "SELECT id, content_key, item_type, plain_text, source_app, preview_path, preview_data_url, created_at, pinned
             FROM clipboard_items
             WHERE id = ?1
             LIMIT 1",
            params![id],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        return Ok(Some(map_clipboard_item_row(&row)?));
    }

    Ok(None)
}

pub async fn pin_clipboard_item(conn: &DbConn, id: &str, pinned: bool) -> DbResult<()> {
    conn.execute(
        "UPDATE clipboard_items SET pinned = ?1 WHERE id = ?2",
        params![if pinned { 1 } else { 0 }, id],
    )
    .await?;
    Ok(())
}

pub async fn touch_clipboard_item(
    conn: &DbConn,
    id: &str,
    created_at: i64,
) -> DbResult<Option<ClipboardItemDto>> {
    conn.execute(
        "UPDATE clipboard_items SET created_at = ?1 WHERE id = ?2",
        params![created_at, id],
    )
    .await?;

    get_clipboard_item(conn, id).await
}

pub async fn delete_clipboard_item(conn: &DbConn, id: &str) -> DbResult<Option<String>> {
    let mut rows = conn
        .query(
            "SELECT preview_path FROM clipboard_items WHERE id = ?1 LIMIT 1",
            params![id],
        )
        .await?;
    let preview_path = if let Some(row) = rows.next().await? {
        row.get::<Option<String>>(0)?
    } else {
        None
    };

    conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])
        .await?;
    Ok(preview_path)
}

pub async fn clear_all_clipboard_items(conn: &DbConn) -> DbResult<Vec<String>> {
    let mut rows = conn
        .query("SELECT preview_path FROM clipboard_items", ())
        .await?;
    let mut preview_paths = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Some(path) = row.get::<Option<String>>(0)? {
            preview_paths.push(path);
        }
    }

    conn.execute("DELETE FROM clipboard_items", ()).await?;
    Ok(preview_paths)
}

fn preview_file_size_bytes(preview_path: Option<&str>) -> u64 {
    let Some(path) = preview_path else {
        return 0;
    };
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return 0;
    }

    match std::fs::metadata(trimmed) {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                tracing::warn!(
                    event = "clipboard_preview_size_missing",
                    preview_path = trimmed
                );
            } else {
                tracing::warn!(
                    event = "clipboard_preview_size_failed",
                    preview_path = trimmed,
                    error = error.to_string()
                );
            }
            0
        }
    }
}

fn clipboard_row_size_bytes(
    plain_text: &str,
    preview_data_url: Option<&str>,
    preview_path: Option<&str>,
) -> u64 {
    plain_text.len() as u64
        + preview_data_url
            .map(|value| value.len() as u64)
            .unwrap_or(0)
        + preview_file_size_bytes(preview_path)
}

pub async fn prune_clipboard_items(
    conn: &DbConn,
    max_items: u32,
    max_total_size_bytes: Option<u64>,
) -> DbResult<Vec<PrunedClipboardItem>> {
    let transaction = conn.transaction().await?;

    if max_total_size_bytes.is_none() {
        let mut rows = transaction
            .query("SELECT COUNT(*) FROM clipboard_items", ())
            .await?;
        let total = if let Some(row) = rows.next().await? {
            row.get::<i64>(0)?
        } else {
            0
        };

        let overflow = total - i64::from(max_items);
        if overflow <= 0 {
            transaction.commit().await?;
            return Ok(Vec::new());
        }

        let mut to_remove = Vec::new();
        let mut rows = transaction
            .query(
                "SELECT id, preview_path
                 FROM clipboard_items
                 ORDER BY pinned ASC, created_at ASC, id ASC
                 LIMIT ?1",
                params![overflow],
            )
            .await?;

        while let Some(row) = rows.next().await? {
            to_remove.push(PrunedClipboardItem {
                id: row.get::<String>(0)?,
                preview_path: row.get::<Option<String>>(1)?,
            });
        }

        for item in &to_remove {
            transaction
                .execute(
                    "DELETE FROM clipboard_items WHERE id = ?1",
                    params![item.id.as_str()],
                )
                .await?;
        }

        transaction.commit().await?;
        return Ok(to_remove);
    }

    let size_limit = max_total_size_bytes.unwrap_or(u64::MAX);
    let mut total_count: u64 = 0;
    let mut total_size: u64 = 0;
    let mut candidates = Vec::new();

    let mut rows = transaction
        .query(
            "SELECT id, preview_path, plain_text, preview_data_url
             FROM clipboard_items
             ORDER BY pinned ASC, created_at ASC, id ASC",
            (),
        )
        .await?;

    while let Some(row) = rows.next().await? {
        let id = row.get::<String>(0)?;
        let preview_path = row.get::<Option<String>>(1)?;
        let plain_text = row.get::<String>(2)?;
        let preview_data_url = row.get::<Option<String>>(3)?;
        let size_bytes = clipboard_row_size_bytes(
            plain_text.as_str(),
            preview_data_url.as_deref(),
            preview_path.as_deref(),
        );

        total_count += 1;
        total_size = total_size.saturating_add(size_bytes);
        candidates.push((id, preview_path, size_bytes));
    }

    if total_count <= u64::from(max_items) && total_size <= size_limit {
        transaction.commit().await?;
        return Ok(Vec::new());
    }

    let mut to_remove = Vec::new();
    for (id, preview_path, size_bytes) in candidates {
        if total_count <= u64::from(max_items) && total_size <= size_limit {
            break;
        }
        total_count = total_count.saturating_sub(1);
        total_size = total_size.saturating_sub(size_bytes);
        to_remove.push(PrunedClipboardItem { id, preview_path });
    }

    for item in &to_remove {
        transaction
            .execute(
                "DELETE FROM clipboard_items WHERE id = ?1",
                params![item.id.as_str()],
            )
            .await?;
    }

    transaction.commit().await?;
    Ok(to_remove)
}

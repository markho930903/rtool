use super::logging_ingest::{normalize_level, sanitize_for_log};
use super::logging_store::row_to_log_entry;
use super::{QUERY_LIMIT_DEFAULT, QUERY_LIMIT_MAX};
use crate::db_error::DbAppError;
use app_core::AppError;
use app_core::models::{LogPageDto, LogQueryDto};
use libsql::{Value as LibsqlValue, params_from_iter};

pub(crate) fn build_log_fts_query(keyword: &str) -> Option<String> {
    let normalized = sanitize_for_log(keyword);
    let tokens = normalized
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
                .to_string()
        })
        .filter(|token| !token.is_empty())
        .take(8)
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return None;
    }

    Some(
        tokens
            .into_iter()
            .map(|token| format!("{token}*"))
            .collect::<Vec<_>>()
            .join(" AND "),
    )
}

pub(super) async fn query_log_entries(
    center: &super::LogCenter,
    query: LogQueryDto,
) -> Result<LogPageDto, AppError> {
    let limit = query.limit.clamp(1, QUERY_LIMIT_MAX);
    let mut sql = String::from(
        "SELECT id, timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count FROM log_entries WHERE 1=1",
    );
    let mut params = Vec::<LibsqlValue>::new();

    if let Some(cursor) = query.cursor.as_deref() {
        let cursor_id = cursor.parse::<i64>().map_err(|_| {
            AppError::new("invalid_cursor", "日志分页游标非法")
                .with_context("cursor", sanitize_for_log(cursor))
        })?;
        sql.push_str(" AND id < ?");
        params.push(LibsqlValue::Integer(cursor_id));
    }

    if let Some(levels) = query.levels.as_ref().filter(|levels| !levels.is_empty()) {
        let mut normalized_levels = Vec::new();
        for level in levels {
            let normalized = normalize_level(level).ok_or_else(|| {
                AppError::new("invalid_log_level", "日志级别非法")
                    .with_context("level", sanitize_for_log(level))
            })?;
            normalized_levels.push(normalized.to_string());
        }

        sql.push_str(" AND level IN (");
        for (index, value) in normalized_levels.iter().enumerate() {
            if index > 0 {
                sql.push_str(", ");
            }
            sql.push('?');
            params.push(LibsqlValue::Text(value.clone()));
        }
        sql.push(')');
    }

    if let Some(scope) = query
        .scope
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        sql.push_str(" AND scope = ?");
        params.push(LibsqlValue::Text(sanitize_for_log(scope)));
    }

    if let Some(request_id) = query
        .request_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        sql.push_str(" AND request_id = ?");
        params.push(LibsqlValue::Text(sanitize_for_log(request_id)));
    }

    if let Some(window_label) = query
        .window_label
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        sql.push_str(" AND window_label = ?");
        params.push(LibsqlValue::Text(sanitize_for_log(window_label)));
    }

    if let Some(keyword) = query
        .keyword
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        if let Some(fts_query) = build_log_fts_query(keyword) {
            sql.push_str(
                " AND id IN (SELECT rowid FROM log_entries_fts WHERE log_entries_fts MATCH ?)",
            );
            params.push(LibsqlValue::Text(fts_query));
        } else {
            sql.push_str(" AND (message LIKE ? OR metadata LIKE ? OR event LIKE ?)");
            let pattern = format!("%{}%", sanitize_for_log(keyword));
            params.push(LibsqlValue::Text(pattern.clone()));
            params.push(LibsqlValue::Text(pattern.clone()));
            params.push(LibsqlValue::Text(pattern));
        }
    }

    if let Some(start_at) = query.start_at {
        sql.push_str(" AND timestamp >= ?");
        params.push(LibsqlValue::Integer(start_at));
    }

    if let Some(end_at) = query.end_at {
        sql.push_str(" AND timestamp <= ?");
        params.push(LibsqlValue::Integer(end_at));
    }

    sql.push_str(" ORDER BY id DESC LIMIT ?");
    params.push(LibsqlValue::Integer(i64::from(limit) + 1));

    let mut rows = center
        .db_conn
        .query(sql.as_str(), params_from_iter(params))
        .await
        .map_err(DbAppError::from)?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().await.map_err(DbAppError::from)? {
        items.push(row_to_log_entry(&row)?);
    }

    let page_size = usize::try_from(limit).unwrap_or(QUERY_LIMIT_DEFAULT as usize);
    let next_cursor = if items.len() > page_size {
        let marker = items
            .get(page_size)
            .map(|value| value.id)
            .unwrap_or_default();
        items.truncate(page_size);
        Some(marker.to_string())
    } else {
        None
    };

    Ok(LogPageDto { items, next_cursor })
}

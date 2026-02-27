use super::ingest::{now_millis, sanitize_for_log};
use super::{HighFrequencyWindow, RecordLogInput};
use crate::AppError;
use crate::db::DbConn;
use crate::db_error::DbResult;
use crate::models::LogEntryDto;
use libsql::{Row, Rows, params};
use serde_json::Value;

const AGGREGATED_EVENT: &str = "high_frequency_aggregated";

pub(super) async fn cleanup_expired_log_entries(conn: &DbConn, keep_days: u32) -> DbResult<()> {
    let keep_ms = i64::from(keep_days) * 24 * 60 * 60 * 1000;
    let cutoff = now_millis().saturating_sub(keep_ms);
    conn.execute(
        "DELETE FROM log_entries WHERE timestamp < ?1",
        params![cutoff],
    )
    .await?;
    Ok(())
}

fn parse_metadata_value(metadata: Option<String>) -> Option<Value> {
    metadata.and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
}

fn serialize_metadata_value(metadata: &Option<Value>) -> Option<String> {
    metadata
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok())
}

fn aggregated_message(key: &str) -> String {
    format!("{AGGREGATED_EVENT} key={}", sanitize_for_log(key))
}

pub(super) fn row_to_log_entry(row: &Row) -> DbResult<LogEntryDto> {
    let aggregated_count: Option<i64> = row.get(10)?;
    Ok(LogEntryDto {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        level: row.get(2)?,
        scope: row.get(3)?,
        event: row.get(4)?,
        request_id: row.get(5)?,
        window_label: row.get(6)?,
        message: row.get(7)?,
        metadata: parse_metadata_value(row.get(8)?),
        raw_ref: row.get(9)?,
        aggregated_count: aggregated_count.and_then(|value| u32::try_from(value).ok()),
    })
}

async fn next_log_entry(
    rows: &mut Rows,
    error_code: &'static str,
    message: &'static str,
) -> DbResult<LogEntryDto> {
    if let Some(row) = rows.next().await? {
        return Ok(row_to_log_entry(&row)?);
    }

    Err(AppError::new(error_code, message).into())
}

pub(super) async fn save_log_entry(
    conn: &DbConn,
    input: &RecordLogInput,
    timestamp: i64,
) -> DbResult<LogEntryDto> {
    let metadata = serialize_metadata_value(&input.metadata);
    let mut rows = conn
        .query(
            "INSERT INTO log_entries (timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL)
         RETURNING id, timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count",
            params![
                timestamp,
                input.level.as_str(),
                input.scope.as_str(),
                input.event.as_str(),
                input.request_id.as_str(),
                input.window_label.as_deref(),
                input.message.as_str(),
                metadata,
                input.raw_ref.as_deref()
            ],
        )
        .await?;

    next_log_entry(&mut rows, "log_insert_missing", "写入日志后未返回记录").await
}

pub(super) async fn upsert_aggregated_log(
    conn: &DbConn,
    key: &str,
    input: &RecordLogInput,
    timestamp: i64,
    window: &mut HighFrequencyWindow,
) -> DbResult<LogEntryDto> {
    if let Some(row_id) = window.aggregated_row_id {
        window.aggregated_count = window.aggregated_count.saturating_add(1);
        let mut rows = conn
            .query(
                "UPDATE log_entries
             SET timestamp = ?1,
                 aggregated_count = ?2,
                 message = ?3
             WHERE id = ?4
             RETURNING id, timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count",
                params![
                    timestamp,
                    i64::from(window.aggregated_count),
                    aggregated_message(key),
                    row_id
                ],
            )
            .await?;

        return next_log_entry(&mut rows, "log_aggregate_missing", "聚合日志记录不存在").await;
    }

    window.aggregated_count = 1;
    let aggregated_message_text = aggregated_message(key);
    let mut rows = conn
        .query(
            "INSERT INTO log_entries (timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, ?8)
         RETURNING id, timestamp, level, scope, event, request_id, window_label, message, metadata, raw_ref, aggregated_count",
            params![
                timestamp,
                input.level.as_str(),
                input.scope.as_str(),
                AGGREGATED_EVENT,
                input.request_id.as_str(),
                input.window_label.as_deref(),
                aggregated_message_text.as_str(),
                i64::from(window.aggregated_count)
            ],
        )
        .await?;

    let entry = next_log_entry(
        &mut rows,
        "log_aggregate_insert_missing",
        "写入聚合日志后未返回记录",
    )
    .await?;
    window.aggregated_row_id = Some(entry.id);
    Ok(entry)
}

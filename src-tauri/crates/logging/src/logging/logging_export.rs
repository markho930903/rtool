use super::logging_ingest::now_millis;
use super::logging_query;
use super::{EXPORT_FLUSH_EVERY_PAGES, EXPORT_THROTTLE_SLEEP_MS};
use crate::models::LogQueryDto;
use crate::{AppError, ResultExt};
use anyhow::Context;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::time::sleep;

pub(super) async fn export_log_entries(
    center: &super::LogCenter,
    query: LogQueryDto,
    output_path: Option<String>,
) -> Result<String, AppError> {
    let mut cursor = query.cursor.clone();
    let mut page_count = 0u32;

    let target_path = output_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            center
                .log_dir
                .join(format!("rtool-log-export-{}.jsonl", now_millis()))
        });
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建日志导出目录失败: {}", parent.display()))
            .with_code("log_export_dir_create_failed", "创建日志导出目录失败")
            .with_ctx("outputDir", parent.display().to_string())?;
    }

    let file = File::create(&target_path)
        .await
        .with_context(|| format!("创建日志导出文件失败: {}", target_path.display()))
        .with_code("log_export_file_create_failed", "创建日志导出文件失败")
        .with_ctx("targetPath", target_path.display().to_string())?;
    let mut writer = BufWriter::new(file);

    loop {
        let mut next_query = query.clone();
        next_query.cursor = cursor.clone();
        next_query.limit = super::QUERY_LIMIT_MAX;

        let page = logging_query::query_log_entries(center, next_query).await?;
        for item in &page.items {
            let line = serde_json::to_string(item)
                .with_context(|| format!("序列化日志导出内容失败: entryId={}", item.id))
                .with_code("log_export_serialize_failed", "序列化日志导出内容失败")
                .with_ctx("entryId", item.id.to_string())?;
            writer
                .write_all(line.as_bytes())
                .await
                .with_context(|| format!("写入日志导出文件失败: {}", target_path.display()))
                .with_code("log_export_write_failed", "写入日志导出文件失败")
                .with_ctx("targetPath", target_path.display().to_string())?;
            writer
                .write_all(b"\n")
                .await
                .with_context(|| format!("写入日志导出文件失败: {}", target_path.display()))
                .with_code("log_export_write_failed", "写入日志导出文件失败")
                .with_ctx("targetPath", target_path.display().to_string())?;
        }

        page_count = page_count.saturating_add(1);
        if page_count.is_multiple_of(EXPORT_FLUSH_EVERY_PAGES) {
            writer
                .flush()
                .await
                .with_context(|| format!("刷新日志导出文件失败: {}", target_path.display()))
                .with_code("log_export_flush_failed", "刷新日志导出文件失败")
                .with_ctx("targetPath", target_path.display().to_string())?;
            sleep(Duration::from_millis(EXPORT_THROTTLE_SLEEP_MS)).await;
        }

        if page.next_cursor.is_none() {
            break;
        }
        cursor = page.next_cursor;
    }

    writer
        .flush()
        .await
        .with_context(|| format!("刷新日志导出文件失败: {}", target_path.display()))
        .with_code("log_export_flush_failed", "刷新日志导出文件失败")
        .with_ctx("targetPath", target_path.display().to_string())?;

    Ok(target_path.to_string_lossy().to_string())
}

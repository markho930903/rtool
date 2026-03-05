use anyhow::Context;
use rtool_contracts::{AppError, AppResult, ResultExt};
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn reveal_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Err(
            AppError::new("app_manager_reveal_not_found", "定位失败：目标路径不存在")
                .with_context("path", path.to_string_lossy().to_string()),
        );
    }

    let target = path.to_path_buf();
    let command_result = if cfg!(target_os = "macos") {
        Command::new("open").arg("-R").arg(&target).status()
    } else if cfg!(target_os = "windows") {
        Command::new("explorer")
            .arg(format!("/select,{}", target.to_string_lossy()))
            .status()
    } else {
        let fallback = if target.is_dir() {
            target.clone()
        } else {
            target
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| path.to_path_buf())
        };
        Command::new("xdg-open").arg(fallback).status()
    };

    let status = command_result
        .with_context(|| {
            format!(
                "failed to launch file manager for {}",
                target.to_string_lossy()
            )
        })
        .with_code(
            "app_manager_reveal_failed",
            "定位失败：无法启动系统文件管理器",
        )?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            "app_manager_reveal_failed",
            "定位失败：系统文件管理器调用异常",
        )
        .with_context("path", target.to_string_lossy().to_string())
        .with_context("status", status.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::reveal_path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reveal_path_returns_not_found_for_missing_path() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let missing = std::env::temp_dir().join(format!(
            "rtool-reveal-missing-{}-{nanos}",
            std::process::id()
        ));
        let missing_display = missing.to_string_lossy().to_string();

        let error = reveal_path(&missing).expect_err("missing path should fail");
        assert_eq!(error.code, "app_manager_reveal_not_found");
        assert!(
            error
                .context
                .iter()
                .any(|item| item.key == "path" && item.value == missing_display)
        );
    }
}

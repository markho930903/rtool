use super::image_preview::{
    build_image_signature, current_source_app, read_image_dimensions_from_header,
    save_clipboard_image_preview,
};
use crate::features::clipboard::events::emit_clipboard_sync;
use domain::service::ClipboardService;
use foundation::models::ClipboardSyncPayload;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

pub(super) struct ClipboardProcessor<R: Runtime> {
    app_handle: AppHandle<R>,
    service: ClipboardService,
    preview_dir: Option<PathBuf>,
    last_seen: String,
    last_image_signature: String,
}

impl<R: Runtime> ClipboardProcessor<R> {
    pub(super) fn new(app_handle: AppHandle<R>, service: ClipboardService) -> Self {
        let preview_dir = match app_handle.path().app_data_dir() {
            Ok(value) => Some(value.join("clipboard_previews")),
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_preview_dir_resolve_failed",
                    error = error.to_string()
                );
                None
            }
        };

        Self {
            app_handle,
            service,
            preview_dir,
            last_seen: String::new(),
            last_image_signature: String::new(),
        }
    }

    async fn handle_text(&mut self, text: String, source_app: Option<String>) -> bool {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() || trimmed == self.last_seen {
            return true;
        }

        self.last_seen = trimmed.clone();
        self.last_image_signature.clear();

        match self.service.save_text(trimmed, source_app).await {
            Ok(result) => {
                emit_clipboard_sync(
                    &self.app_handle,
                    ClipboardSyncPayload {
                        upsert: vec![result.item],
                        removed_ids: result.removed_ids,
                        clear_all: false,
                        reason: Some("watcher_save_text".to_string()),
                    },
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_text_save_failed",
                    error_code = error.code.as_str(),
                    error_detail = error
                        .causes
                        .first()
                        .map(String::as_str)
                        .map(foundation::logging::sanitize_for_log)
                        .unwrap_or_default()
                );
            }
        }
        true
    }

    async fn handle_files(&mut self, files_uris: Vec<String>, source_app: Option<String>) -> bool {
        let normalized_files: Vec<String> = files_uris
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if normalized_files.is_empty() {
            return false;
        }

        let serialized = normalized_files.join("\n");
        if serialized == self.last_seen {
            return true;
        }

        self.last_seen = serialized.clone();
        self.last_image_signature.clear();

        match self.service.save_text(serialized, source_app).await {
            Ok(result) => {
                emit_clipboard_sync(
                    &self.app_handle,
                    ClipboardSyncPayload {
                        upsert: vec![result.item],
                        removed_ids: result.removed_ids,
                        clear_all: false,
                        reason: Some("watcher_save_files".to_string()),
                    },
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_files_save_failed",
                    error_code = error.code.as_str(),
                    error_detail = error
                        .causes
                        .first()
                        .map(String::as_str)
                        .map(foundation::logging::sanitize_for_log)
                        .unwrap_or_default()
                );
            }
        }
        true
    }

    async fn handle_image(&mut self, png_bytes: &[u8], source_app: Option<String>) {
        let (width_u32, height_u32) =
            if let Some(dimensions) = read_image_dimensions_from_header(png_bytes) {
                dimensions
            } else {
                match image::load_from_memory(png_bytes) {
                    Ok(decoded) => (decoded.width(), decoded.height()),
                    Err(error) => {
                        tracing::warn!(
                            event = "clipboard_image_decode_failed",
                            error = error.to_string()
                        );
                        return;
                    }
                }
            };
        let width = width_u32 as usize;
        let height = height_u32 as usize;
        let signature = build_image_signature(width, height, png_bytes);
        if signature == self.last_image_signature {
            return;
        }

        if let Err(error) = self.service.ensure_disk_space_for_new_item() {
            tracing::warn!(
                event = "clipboard_image_skip_low_disk",
                error_code = error.code.as_str(),
                error_detail = error
                    .causes
                    .first()
                    .map(String::as_str)
                    .map(foundation::logging::sanitize_for_log)
                    .unwrap_or_default()
            );
            return;
        }

        self.last_image_signature = signature.clone();
        self.last_seen.clear();

        let preview_path = self.preview_dir.as_ref().and_then(|dir| {
            match save_clipboard_image_preview(dir, &signature, png_bytes) {
                Ok(path) => Some(path),
                Err(error) => {
                    tracing::warn!(
                        event = "clipboard_preview_save_failed",
                        signature = %signature,
                        error = error.to_string()
                    );
                    None
                }
            }
        });

        let item = foundation::clipboard::build_image_clipboard_item(
            width,
            height,
            &signature,
            preview_path,
            None,
            source_app,
        );

        match self.service.save_item(item).await {
            Ok(result) => {
                emit_clipboard_sync(
                    &self.app_handle,
                    ClipboardSyncPayload {
                        upsert: vec![result.item],
                        removed_ids: result.removed_ids,
                        clear_all: false,
                        reason: Some("watcher_save_image".to_string()),
                    },
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "clipboard_image_save_failed",
                    error_code = error.code.as_str(),
                    error_detail = error
                        .causes
                        .first()
                        .map(String::as_str)
                        .map(foundation::logging::sanitize_for_log)
                        .unwrap_or_default()
                );
            }
        }
    }

    pub(super) async fn handle_update_event(&mut self) {
        let source_app = current_source_app();
        let files_uris_result = {
            let clipboard = self.app_handle.state::<tauri_plugin_clipboard::Clipboard>();
            clipboard.read_files_uris()
        };
        if let Ok(files_uris) = files_uris_result
            && self.handle_files(files_uris, source_app.clone()).await
        {
            return;
        }

        let image_binary_result = {
            let clipboard = self.app_handle.state::<tauri_plugin_clipboard::Clipboard>();
            clipboard.read_image_binary()
        };
        if let Ok(image_binary) = image_binary_result {
            self.handle_image(&image_binary, source_app.clone()).await;
            return;
        }

        let text_result = {
            let clipboard = self.app_handle.state::<tauri_plugin_clipboard::Clipboard>();
            clipboard.read_text()
        };
        if let Ok(text) = text_result {
            self.handle_text(text, source_app).await;
        }
    }
}

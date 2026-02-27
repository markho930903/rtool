#[path = "clipboard_service.rs"]
mod clipboard_service;

pub mod service {
    pub use super::clipboard_service::{
        CLIPBOARD_MAX_ITEMS_DEFAULT, CLIPBOARD_MAX_ITEMS_MAX, CLIPBOARD_MAX_ITEMS_MIN,
        CLIPBOARD_MAX_TOTAL_SIZE_MB_DEFAULT, CLIPBOARD_MAX_TOTAL_SIZE_MB_MAX,
        CLIPBOARD_MAX_TOTAL_SIZE_MB_MIN, CLIPBOARD_MIN_FREE_DISK_BYTES,
        CLIPBOARD_SIZE_CLEANUP_ENABLED_DEFAULT, ClipboardSaveResult, ClipboardService,
        ClipboardSettingsUpdateResult,
    };
}

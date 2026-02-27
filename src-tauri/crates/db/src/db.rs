use libsql::Connection;

pub const CLIPBOARD_MAX_ITEMS_KEY: &str = "clipboard.maxItems";
pub const CLIPBOARD_SIZE_CLEANUP_ENABLED_KEY: &str = "clipboard.sizeCleanupEnabled";
pub const CLIPBOARD_MAX_TOTAL_SIZE_MB_KEY: &str = "clipboard.maxTotalSizeMb";
pub(crate) const CLIPBOARD_LIST_LIMIT_MAX: u32 = 10_000;

#[derive(Debug, Clone)]
pub struct PrunedClipboardItem {
    pub id: String,
    pub preview_path: Option<String>,
}

pub type DbConn = Connection;

#[path = "db_bootstrap.rs"]
mod db_bootstrap;
#[path = "db_clipboard_store.rs"]
mod db_clipboard_store;
#[path = "db_settings_store.rs"]
mod db_settings_store;

pub use db_bootstrap::{init_db, open_db};
pub use db_clipboard_store::{
    clear_all_clipboard_items, delete_clipboard_item, get_clipboard_item, insert_clipboard_item,
    list_clipboard_items, pin_clipboard_item, prune_clipboard_items, touch_clipboard_item,
};
pub use db_settings_store::{
    delete_app_settings, get_app_setting, get_app_settings_batch, set_app_setting,
    set_app_settings_batch,
};


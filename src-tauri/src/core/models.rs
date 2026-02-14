use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteItemDto {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResultDto {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LauncherActionDto {
    OpenBuiltinRoute {
        route: String,
    },
    OpenBuiltinTool {
        #[serde(rename = "toolId")]
        tool_id: String,
    },
    OpenBuiltinWindow {
        #[serde(rename = "windowLabel")]
        window_label: String,
    },
    OpenFile {
        path: String,
    },
    OpenApplication {
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherItemDto {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    pub score: i32,
    pub icon_kind: String,
    pub icon_value: String,
    pub action: LauncherActionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardFilterDto {
    pub query: Option<String>,
    pub item_type: Option<String>,
    pub only_pinned: Option<bool>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItemDto {
    pub id: String,
    pub content_key: String,
    pub item_type: String,
    pub plain_text: String,
    pub source_app: Option<String>,
    pub preview_path: Option<String>,
    pub preview_data_url: Option<String>,
    pub created_at: i64,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSettingsDto {
    pub max_items: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardWindowOpenedPayload {
    pub compact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardWindowModeAppliedDto {
    pub compact: bool,
    pub applied_width_logical: f64,
    pub applied_height_logical: f64,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSyncPayload {
    pub upsert: Vec<ClipboardItemDto>,
    pub removed_ids: Vec<String>,
    pub clear_all: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRuntimeInfoDto {
    pub app_name: String,
    pub app_version: String,
    pub build_mode: String,
    pub uptime_seconds: u64,
    pub process_memory_bytes: Option<u64>,
    pub database_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfoDto {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub kernel_version: Option<String>,
    pub arch: Option<String>,
    pub host_name: Option<String>,
    pub cpu_brand: Option<String>,
    pub cpu_cores: Option<u32>,
    pub total_memory_bytes: Option<u64>,
    pub used_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshotDto {
    pub sampled_at: i64,
    pub app: AppRuntimeInfoDto,
    pub system: SystemInfoDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LogQueryDto {
    pub cursor: Option<String>,
    pub limit: u32,
    pub levels: Option<Vec<String>>,
    pub scope: Option<String>,
    pub request_id: Option<String>,
    pub window_label: Option<String>,
    pub keyword: Option<String>,
    pub start_at: Option<i64>,
    pub end_at: Option<i64>,
}

impl Default for LogQueryDto {
    fn default() -> Self {
        Self {
            cursor: None,
            limit: 100,
            levels: None,
            scope: None,
            request_id: None,
            window_label: None,
            keyword: None,
            start_at: None,
            end_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntryDto {
    pub id: i64,
    pub timestamp: i64,
    pub level: String,
    pub scope: String,
    pub event: String,
    pub request_id: String,
    pub window_label: Option<String>,
    pub message: String,
    pub metadata: Option<Value>,
    pub raw_ref: Option<String>,
    pub aggregated_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPageDto {
    pub items: Vec<LogEntryDto>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogConfigDto {
    pub min_level: String,
    pub keep_days: u32,
    pub realtime_enabled: bool,
    pub high_freq_window_ms: u32,
    pub high_freq_max_per_key: u32,
    pub allow_raw_view: bool,
}

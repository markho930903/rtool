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
#[serde(rename_all = "camelCase", default)]
pub struct AppManagerQueryDto {
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

impl Default for AppManagerQueryDto {
    fn default() -> Self {
        Self {
            keyword: None,
            category: None,
            limit: Some(100),
            cursor: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerCapabilitiesDto {
    pub startup: bool,
    pub uninstall: bool,
    pub residue_scan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerIdentityDto {
    pub primary_id: String,
    pub aliases: Vec<String>,
    pub identity_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAppDto {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_or_app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    pub platform: String,
    pub source: String,
    pub icon_kind: String,
    pub icon_value: String,
    pub estimated_size_bytes: Option<u64>,
    pub startup_enabled: bool,
    pub startup_scope: String,
    pub startup_editable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason_code: Option<String>,
    pub uninstall_supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uninstall_kind: Option<String>,
    pub capabilities: AppManagerCapabilitiesDto,
    pub identity: AppManagerIdentityDto,
    pub risk_level: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerPageDto {
    pub items: Vec<ManagedAppDto>,
    pub next_cursor: Option<String>,
    pub indexed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerStartupUpdateInputDto {
    pub app_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerUninstallInputDto {
    pub app_id: String,
    pub confirmed_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerDetailQueryDto {
    pub app_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRelatedRootDto {
    pub id: String,
    pub label: String,
    pub path: String,
    pub path_type: String,
    pub scope: String,
    pub kind: String,
    pub exists: bool,
    pub readonly: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSizeSummaryDto {
    pub app_bytes: Option<u64>,
    pub residue_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedAppDetailDto {
    pub app: ManagedAppDto,
    pub install_path: String,
    pub related_roots: Vec<AppRelatedRootDto>,
    pub size_summary: AppSizeSummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResidueScanInputDto {
    pub app_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResidueItemDto {
    pub item_id: String,
    pub path: String,
    pub path_type: String,
    pub kind: String,
    pub scope: String,
    pub size_bytes: u64,
    pub match_reason: String,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub risk_level: String,
    pub recommended: bool,
    pub readonly: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResidueGroupDto {
    pub group_id: String,
    pub label: String,
    pub scope: String,
    pub kind: String,
    pub total_size_bytes: u64,
    pub items: Vec<AppManagerResidueItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerScanWarningDto {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResidueScanResultDto {
    pub app_id: String,
    pub total_size_bytes: u64,
    pub groups: Vec<AppManagerResidueGroupDto>,
    pub warnings: Vec<AppManagerScanWarningDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerCleanupInputDto {
    pub app_id: String,
    pub selected_item_ids: Vec<String>,
    pub delete_mode: String,
    pub include_main_app: bool,
    pub skip_on_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerCleanupItemResultDto {
    pub item_id: String,
    pub path: String,
    pub kind: String,
    pub status: String,
    pub reason_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerCleanupResultDto {
    pub app_id: String,
    pub delete_mode: String,
    pub released_size_bytes: u64,
    pub deleted: Vec<AppManagerCleanupItemResultDto>,
    pub skipped: Vec<AppManagerCleanupItemResultDto>,
    pub failed: Vec<AppManagerCleanupItemResultDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerExportScanInputDto {
    pub app_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerExportScanResultDto {
    pub app_id: String,
    pub file_path: String,
    pub directory_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerActionResultDto {
    pub ok: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
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
    pub size_cleanup_enabled: bool,
    pub max_total_size_mb: u32,
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
pub struct TransferSettingsDto {
    pub default_download_dir: String,
    pub max_parallel_files: u32,
    pub max_inflight_chunks: u32,
    pub chunk_size_kb: u32,
    pub auto_cleanup_days: u32,
    pub resume_enabled: bool,
    pub discovery_enabled: bool,
    pub pairing_required: bool,
    pub pipeline_v2_enabled: bool,
    pub codec_v2_enabled: bool,
    pub db_flush_interval_ms: u32,
    pub event_emit_interval_ms: u32,
    pub ack_batch_size: u32,
    pub ack_flush_interval_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TransferUpdateSettingsInputDto {
    pub default_download_dir: Option<String>,
    pub max_parallel_files: Option<u32>,
    pub max_inflight_chunks: Option<u32>,
    pub chunk_size_kb: Option<u32>,
    pub auto_cleanup_days: Option<u32>,
    pub resume_enabled: Option<bool>,
    pub discovery_enabled: Option<bool>,
    pub pairing_required: Option<bool>,
}

impl Default for TransferUpdateSettingsInputDto {
    fn default() -> Self {
        Self {
            default_download_dir: None,
            max_parallel_files: None,
            max_inflight_chunks: None,
            chunk_size_kb: None,
            auto_cleanup_days: None,
            resume_enabled: None,
            discovery_enabled: None,
            pairing_required: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferPeerDto {
    pub device_id: String,
    pub display_name: String,
    pub address: String,
    pub listen_port: u16,
    pub last_seen_at: i64,
    pub paired_at: Option<i64>,
    pub trust_level: String,
    pub failed_attempts: u32,
    pub blocked_until: Option<i64>,
    pub pairing_required: bool,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferPairingCodeDto {
    pub code: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferFileInputDto {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress_folder: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferSendFilesInputDto {
    pub peer_device_id: String,
    pub pair_code: String,
    pub files: Vec<TransferFileInputDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferFileDto {
    pub id: String,
    pub session_id: String,
    pub relative_path: String,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
    pub size_bytes: u64,
    pub transferred_bytes: u64,
    pub chunk_size: u32,
    pub chunk_count: u32,
    pub status: String,
    pub blake3: Option<String>,
    pub mime_type: Option<String>,
    pub preview_kind: Option<String>,
    pub preview_data: Option<String>,
    pub is_folder_archive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferSessionDto {
    pub id: String,
    pub direction: String,
    pub peer_device_id: String,
    pub peer_name: String,
    pub status: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub avg_speed_bps: u64,
    pub save_dir: String,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub cleanup_after_at: Option<i64>,
    pub files: Vec<TransferFileDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgressSnapshotDto {
    pub session: TransferSessionDto,
    pub active_file_id: Option<String>,
    pub speed_bps: u64,
    pub eta_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inflight_chunks: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retransmit_chunks: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TransferHistoryFilterDto {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub status: Option<String>,
    pub peer_device_id: Option<String>,
}

impl Default for TransferHistoryFilterDto {
    fn default() -> Self {
        Self {
            cursor: None,
            limit: Some(30),
            status: None,
            peer_device_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferHistoryPageDto {
    pub items: Vec<TransferSessionDto>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TransferClearHistoryInputDto {
    pub all: Option<bool>,
    pub older_than_days: Option<u32>,
}

impl Default for TransferClearHistoryInputDto {
    fn default() -> Self {
        Self {
            all: Some(false),
            older_than_days: Some(30),
        }
    }
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

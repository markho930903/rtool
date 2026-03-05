use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResultDto {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsDto {
    pub theme: ThemeSettingsDto,
    pub layout: LayoutSettingsDto,
    pub locale: LocaleSettingsDto,
    pub clipboard: SettingsClipboardDto,
    pub screenshot: SettingsScreenshotDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemeSettingsDto {
    pub preference: String,
    pub transparent_window_background: bool,
}

impl Default for ThemeSettingsDto {
    fn default() -> Self {
        Self {
            preference: "system".to_string(),
            transparent_window_background: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSettingsDto {
    pub preference: String,
}

impl Default for LayoutSettingsDto {
    fn default() -> Self {
        Self {
            preference: "topbar".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleSettingsDto {
    pub preference: String,
}

impl Default for LocaleSettingsDto {
    fn default() -> Self {
        Self {
            preference: "system".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsUpdateInputDto {
    pub theme: Option<ThemeSettingsUpdateInputDto>,
    pub layout: Option<LayoutSettingsUpdateInputDto>,
    pub locale: Option<LocaleSettingsUpdateInputDto>,
    pub clipboard: Option<SettingsClipboardUpdateInputDto>,
    pub screenshot: Option<SettingsScreenshotUpdateInputDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemeSettingsUpdateInputDto {
    pub preference: Option<String>,
    pub transparent_window_background: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LayoutSettingsUpdateInputDto {
    pub preference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LocaleSettingsUpdateInputDto {
    pub preference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsClipboardDto {
    pub max_items: u32,
    pub size_cleanup_enabled: bool,
    pub max_total_size_mb: u32,
}

impl Default for SettingsClipboardDto {
    fn default() -> Self {
        Self {
            max_items: 1000,
            size_cleanup_enabled: true,
            max_total_size_mb: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsClipboardUpdateInputDto {
    pub max_items: Option<u32>,
    pub size_cleanup_enabled: Option<bool>,
    pub max_total_size_mb: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsScreenshotDto {
    pub shortcut: String,
    pub auto_save_enabled: bool,
    pub max_items: u32,
    pub max_total_size_mb: u32,
    pub pin_max_instances: u32,
}

impl Default for SettingsScreenshotDto {
    fn default() -> Self {
        Self {
            shortcut: "Alt+Shift+S".to_string(),
            auto_save_enabled: true,
            max_items: 300,
            max_total_size_mb: 2048,
            pin_max_instances: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsScreenshotUpdateInputDto {
    pub shortcut: Option<String>,
    pub auto_save_enabled: Option<bool>,
    pub max_items: Option<u32>,
    pub max_total_size_mb: Option<u32>,
    pub pin_max_instances: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
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
    OpenDirectory {
        path: String,
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
pub struct LauncherSearchSettingsDto {
    pub roots: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_scan_depth: u32,
    pub max_items_per_root: u32,
    pub max_total_items: u32,
    pub refresh_interval_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherUpdateSearchSettingsInputDto {
    pub roots: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub max_scan_depth: Option<u32>,
    pub max_items_per_root: Option<u32>,
    pub max_total_items: Option<u32>,
    pub refresh_interval_secs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherIndexStatusDto {
    pub ready: bool,
    pub building: bool,
    pub indexed_items: u64,
    pub indexed_roots: u32,
    pub last_build_ms: Option<i64>,
    pub last_duration_ms: Option<u64>,
    pub last_error: Option<String>,
    pub refresh_interval_secs: u32,
    pub index_version: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherRebuildResultDto {
    pub success: bool,
    pub duration_ms: u64,
    pub indexed_items: u64,
    pub indexed_roots: u32,
    pub truncated: bool,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppManagerQueryDto {
    pub keyword: Option<String>,
    pub category: AppManagerCategory,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

impl Default for AppManagerQueryDto {
    fn default() -> Self {
        Self {
            keyword: None,
            category: AppManagerCategory::All,
            limit: Some(100),
            cursor: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerSnapshotMetaDto {
    pub indexed_at: i64,
    pub revision: u64,
    pub total_count: u64,
    pub index_state: AppManagerIndexState,
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
    pub identity_source: AppManagerIdentitySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppReadonlyReasonCode {
    PermissionDenied,
    ManagedByPolicy,
    FeatureDisabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerScope {
    User,
    System,
}

impl AppManagerScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerPathType {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerStartupScope {
    User,
    System,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerResidueKind {
    Install,
    AppSupport,
    Cache,
    Preferences,
    Logs,
    Startup,
    AppScript,
    Container,
    GroupContainer,
    SavedState,
    WebkitData,
    LaunchAgent,
    LaunchDaemon,
    HelperTool,
    AppData,
    RegistryKey,
    RegistryValue,
    MainApp,
}

impl AppManagerResidueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::AppSupport => "app_support",
            Self::Cache => "cache",
            Self::Preferences => "preferences",
            Self::Logs => "logs",
            Self::Startup => "startup",
            Self::AppScript => "app_script",
            Self::Container => "container",
            Self::GroupContainer => "group_container",
            Self::SavedState => "saved_state",
            Self::WebkitData => "webkit_data",
            Self::LaunchAgent => "launch_agent",
            Self::LaunchDaemon => "launch_daemon",
            Self::HelperTool => "helper_tool",
            Self::AppData => "app_data",
            Self::RegistryKey => "registry_key",
            Self::RegistryValue => "registry_value",
            Self::MainApp => "main_app",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerResidueConfidence {
    Exact,
    High,
    Medium,
}

impl AppManagerResidueConfidence {
    pub fn rank(self) -> u8 {
        match self {
            Self::Exact => 3,
            Self::High => 2,
            Self::Medium => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerIdentitySource {
    BundleId,
    Registry,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerSource {
    Rtool,
    Application,
}

impl AppManagerSource {
    pub fn sort_rank(self) -> u8 {
        match self {
            Self::Application => 0,
            Self::Rtool => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerSizeAccuracy {
    Exact,
    Estimated,
}

impl AppManagerSizeAccuracy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Estimated => "estimated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerSizeSource {
    AppBundle,
    ParentDirectory,
    #[default]
    Path,
    RegistryEstimated,
}

impl AppManagerSizeSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AppBundle => "app_bundle",
            Self::ParentDirectory => "parent_directory",
            Self::Path => "path",
            Self::RegistryEstimated => "registry_estimated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerIndexState {
    Ready,
    Building,
    Degraded,
}

impl AppManagerIndexState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Building => "building",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerIndexUpdateReason {
    Manual,
    AutoChange,
    Startup,
}

impl AppManagerIndexUpdateReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::AutoChange => "auto_change",
            Self::Startup => "startup",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerCategory {
    #[default]
    All,
    Rtool,
    Application,
    Startup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerPlatform {
    Macos,
    Windows,
    Linux,
}

impl AppManagerPlatform {
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::Macos
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            unreachable!("unsupported target platform")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerIconKind {
    Raster,
    Iconify,
}

impl AppManagerIconKind {
    pub fn from_raw(value: &str) -> Self {
        if value.eq_ignore_ascii_case("raster") {
            return Self::Raster;
        }
        if value.eq_ignore_ascii_case("iconify") {
            return Self::Iconify;
        }
        Self::Iconify
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerUninstallKind {
    FinderTrash,
    RegistryCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerResidueMatchReason {
    RelatedRoot,
    BundleId,
    ExtensionBundle,
    EntitlementGroup,
    IdentifierPattern,
    KeywordToken,
    StartupLabel,
    StartupShortcut,
    UninstallRegistry,
    StartupRegistry,
    RunRegistry,
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
    pub platform: AppManagerPlatform,
    pub source: AppManagerSource,
    pub icon_kind: AppManagerIconKind,
    pub icon_value: String,
    pub size_bytes: Option<u64>,
    pub size_accuracy: AppManagerSizeAccuracy,
    #[serde(default)]
    pub size_source: AppManagerSizeSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_computed_at: Option<i64>,
    pub startup_enabled: bool,
    pub startup_scope: AppManagerStartupScope,
    pub startup_editable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason_code: Option<AppReadonlyReasonCode>,
    pub uninstall_supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uninstall_kind: Option<AppManagerUninstallKind>,
    pub capabilities: AppManagerCapabilitiesDto,
    pub identity: AppManagerIdentityDto,
    pub risk_level: AppManagerRiskLevel,
    pub fingerprint: String,
}

impl AppManagerCategory {
    pub fn matches_item(self, item: &ManagedAppDto) -> bool {
        match self {
            Self::All => true,
            Self::Rtool => matches!(item.source, AppManagerSource::Rtool),
            Self::Application => matches!(item.source, AppManagerSource::Application),
            Self::Startup => item.startup_enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerPageDto {
    pub items: Vec<ManagedAppDto>,
    pub next_cursor: Option<String>,
    pub total_count: u64,
    pub indexed_at: i64,
    pub revision: u64,
    pub index_state: AppManagerIndexState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerIndexUpdatedPayloadDto {
    pub revision: u64,
    pub indexed_at: i64,
    pub changed_count: u32,
    pub reason: AppManagerIndexUpdateReason,
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
    pub path_type: AppManagerPathType,
    pub scope: AppManagerScope,
    pub kind: AppManagerResidueKind,
    pub exists: bool,
    pub readonly: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason_code: Option<AppReadonlyReasonCode>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<AppManagerResidueScanMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerResidueScanMode {
    Quick,
    Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResolveSizesInputDto {
    pub app_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResolvedSizeDto {
    pub app_id: String,
    pub size_bytes: Option<u64>,
    pub size_accuracy: AppManagerSizeAccuracy,
    pub size_source: AppManagerSizeSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_computed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResolveSizesResultDto {
    pub items: Vec<AppManagerResolvedSizeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResidueItemDto {
    pub item_id: String,
    pub path: String,
    pub path_type: AppManagerPathType,
    pub kind: AppManagerResidueKind,
    pub scope: AppManagerScope,
    pub size_bytes: u64,
    pub match_reason: AppManagerResidueMatchReason,
    pub confidence: AppManagerResidueConfidence,
    pub evidence: Vec<String>,
    pub risk_level: AppManagerRiskLevel,
    pub recommended: bool,
    pub readonly: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly_reason_code: Option<AppReadonlyReasonCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResidueGroupDto {
    pub group_id: String,
    pub label: String,
    pub scope: AppManagerScope,
    pub kind: AppManagerResidueKind,
    pub total_size_bytes: u64,
    pub items: Vec<AppManagerResidueItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerScanWarningDto {
    pub code: AppManagerScanWarningCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail_code: Option<AppManagerScanWarningDetailCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerScanWarningCode {
    AppManagerSizeMetadataReadFailed,
    AppManagerSizeEstimateTruncated,
    AppManagerSizeReadDirFailed,
    AppManagerSizeReadDirEntryFailed,
    AppManagerSizeReadFileTypeFailed,
    AppManagerSizeReadMetadataFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerScanWarningDetailCode {
    PermissionDenied,
    NotFound,
    Interrupted,
    InvalidData,
    TimedOut,
    WouldBlock,
    LimitReached,
    IoOther,
}

impl AppManagerScanWarningDetailCode {
    pub fn from_io_error_kind(kind: std::io::ErrorKind) -> Self {
        use std::io::ErrorKind;

        match kind {
            ErrorKind::PermissionDenied => Self::PermissionDenied,
            ErrorKind::NotFound => Self::NotFound,
            ErrorKind::Interrupted => Self::Interrupted,
            ErrorKind::InvalidInput | ErrorKind::InvalidData => Self::InvalidData,
            ErrorKind::TimedOut => Self::TimedOut,
            ErrorKind::WouldBlock => Self::WouldBlock,
            ErrorKind::Other => Self::IoOther,
            _ => Self::IoOther,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerResidueScanResultDto {
    pub app_id: String,
    pub scan_mode: AppManagerResidueScanMode,
    pub total_size_bytes: u64,
    pub groups: Vec<AppManagerResidueGroupDto>,
    pub warnings: Vec<AppManagerScanWarningDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerCleanupInputDto {
    pub app_id: String,
    pub selected_item_ids: Vec<String>,
    pub delete_mode: AppManagerCleanupDeleteMode,
    pub include_main_app: bool,
    pub skip_on_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerCleanupDeleteMode {
    Trash,
    Permanent,
}

impl AppManagerCleanupDeleteMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trash => "trash",
            Self::Permanent => "permanent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerCleanupStatus {
    Deleted,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerCleanupReasonCode {
    Ok,
    SelfUninstallForbidden,
    ManagedByPolicy,
    NotFound,
    AppManagerCleanupDeleteFailed,
    AppManagerCleanupNotFound,
    AppManagerCleanupPathInvalid,
    AppManagerCleanupNotSupported,
    AppManagerUninstallFailed,
}

impl AppManagerCleanupReasonCode {
    pub fn from_error_code(code: &str) -> Self {
        match code {
            "app_manager_cleanup_delete_failed" => Self::AppManagerCleanupDeleteFailed,
            "app_manager_cleanup_not_found" => Self::AppManagerCleanupNotFound,
            "app_manager_cleanup_path_invalid" => Self::AppManagerCleanupPathInvalid,
            "app_manager_cleanup_not_supported" => Self::AppManagerCleanupNotSupported,
            "app_manager_uninstall_failed" => Self::AppManagerUninstallFailed,
            _ => Self::AppManagerCleanupDeleteFailed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerCleanupItemResultDto {
    pub item_id: String,
    pub path: String,
    pub kind: AppManagerResidueKind,
    pub status: AppManagerCleanupStatus,
    pub reason_code: AppManagerCleanupReasonCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManagerCleanupResultDto {
    pub app_id: String,
    pub delete_mode: AppManagerCleanupDeleteMode,
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
    pub code: AppManagerActionCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppManagerActionCode {
    AppManagerRefreshed,
    AppManagerStartupUpdated,
    AppManagerUninstallStarted,
    AppManagerUninstallHelpOpened,
    AppManagerPermissionHelpOpened,
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
pub struct ClipboardImageExportResultDto {
    pub saved: bool,
    pub path: Option<String>,
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
pub struct ScreenshotDisplayDto {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSessionDto {
    pub session_id: String,
    pub started_at_ms: i64,
    pub ttl_ms: i64,
    pub active_display_id: String,
    pub displays: Vec<ScreenshotDisplayDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenshotStartInputDto {
    pub display_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotCommitInputDto {
    pub session_id: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub auto_save: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenshotCancelInputDto {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotCommitResultDto {
    pub session_id: String,
    pub clipboard_accepted: bool,
    pub clipboard_async: bool,
    pub archive_path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotPinResultDto {
    pub session_id: String,
    pub clipboard_accepted: bool,
    pub clipboard_async: bool,
    pub window_label: String,
    pub width: u32,
    pub height: u32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotOperationResultPayload {
    pub session_id: String,
    pub operation: String,
    pub phase: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotWindowOpenedPayload {
    pub session: ScreenshotSessionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotPinWindowOpenedPayload {
    pub target_window_label: String,
    pub image_path: String,
    pub screen_x: i32,
    pub screen_y: i32,
    pub width: u32,
    pub height: u32,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherRuntimeStatusDto {
    pub started: bool,
    pub building: bool,
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

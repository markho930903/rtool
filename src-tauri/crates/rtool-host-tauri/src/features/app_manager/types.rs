use crate::shared::command_response::CommandPayloadContext;
use rtool_contracts::models::{
    AppManagerCleanupInputDto, AppManagerDetailQueryDto, AppManagerExportScanInputDto,
    AppManagerQueryDto, AppManagerResidueScanInputDto, AppManagerResolveSizesInputDto,
    AppManagerStartupUpdateInputDto, AppManagerUninstallInputDto,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct AppManagerListPayload {
    pub(super) query: Option<AppManagerQueryDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerDetailPayload {
    pub(super) query: AppManagerDetailQueryDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerResolveSizesPayload {
    pub(super) input: AppManagerResolveSizesInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerResidueInputPayload {
    pub(super) input: AppManagerResidueScanInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerCleanupPayload {
    pub(super) input: AppManagerCleanupInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerExportPayload {
    pub(super) input: AppManagerExportScanInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerStartupPayload {
    pub(super) input: AppManagerStartupUpdateInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerUninstallPayload {
    pub(super) input: AppManagerUninstallInputDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerHelpPayload {
    pub(super) app_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppManagerRevealPayload {
    pub(super) path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum AppManagerRequest {
    List(AppManagerListPayload),
    ListSnapshotMeta,
    ResolveSizes(AppManagerResolveSizesPayload),
    GetDetailCore(AppManagerDetailPayload),
    GetDetailHeavy(AppManagerResidueInputPayload),
    Cleanup(AppManagerCleanupPayload),
    ExportScanResult(AppManagerExportPayload),
    RefreshIndex,
    SetStartup(AppManagerStartupPayload),
    Uninstall(AppManagerUninstallPayload),
    OpenUninstallHelp(AppManagerHelpPayload),
    OpenPermissionHelp(AppManagerHelpPayload),
    RevealPath(AppManagerRevealPayload),
}

pub const APP_MANAGER_COMMAND_CONTEXT: CommandPayloadContext = CommandPayloadContext::new(
    "app_manager",
    "应用管理命令参数无效",
    "应用管理命令返回序列化失败",
    "未知应用管理命令",
);

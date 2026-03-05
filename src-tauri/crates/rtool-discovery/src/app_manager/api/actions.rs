use super::*;

pub fn set_managed_app_startup(
    app: &dyn LauncherHost,
    input: AppManagerStartupUpdateInputDto,
) -> AppResult<AppManagerActionResultDto> {
    let item = load_indexed_item(app, input.app_id.as_str())?;

    if !item.startup_editable {
        return Err(app_error(
            AppManagerErrorCode::StartupReadOnly,
            "当前应用启动项为只读，无法修改",
        ));
    }

    platform_set_startup(
        item.id.as_str(),
        Path::new(item.path.as_str()),
        input.enabled,
    )?;
    let _ = load_or_refresh_index(app, true)?;

    let message = if input.enabled {
        "已启用开机启动"
    } else {
        "已关闭开机启动"
    };
    Ok(make_action_result(
        true,
        AppManagerActionCode::AppManagerStartupUpdated,
        message,
        Some(item.name),
    ))
}

pub fn uninstall_managed_app(
    app: &dyn LauncherHost,
    input: AppManagerUninstallInputDto,
) -> AppResult<AppManagerActionResultDto> {
    let item = load_indexed_item(app, input.app_id.as_str())?;

    if item.fingerprint != input.confirmed_fingerprint {
        return Err(app_error(
            AppManagerErrorCode::FingerprintMismatch,
            "应用信息已变化，请刷新后重试",
        ));
    }

    if !item.uninstall_supported {
        return Err(app_error(
            AppManagerErrorCode::UninstallUnsupported,
            "该应用不支持在当前平台直接卸载",
        ));
    }

    if item.source == AppManagerSource::Rtool {
        return Err(app_error(
            AppManagerErrorCode::UninstallSelfForbidden,
            "不支持卸载当前运行中的应用",
        ));
    }

    platform_uninstall(&item)?;
    let _ = load_or_refresh_index(app, true)?;

    Ok(make_action_result(
        true,
        AppManagerActionCode::AppManagerUninstallStarted,
        "已触发系统卸载流程",
        Some(item.name),
    ))
}

pub fn open_uninstall_help(
    app: &dyn LauncherHost,
    app_id: String,
) -> AppResult<AppManagerActionResultDto> {
    let item = load_indexed_item(app, app_id.as_str())?;
    platform_open_uninstall_help(&item)?;
    Ok(make_action_result(
        true,
        AppManagerActionCode::AppManagerUninstallHelpOpened,
        "已打开系统卸载入口",
        Some(item.name),
    ))
}

pub fn open_permission_help(
    app: &dyn LauncherHost,
    app_id: String,
) -> AppResult<AppManagerActionResultDto> {
    let item = load_indexed_item(app, app_id.as_str())?;
    platform_open_permission_help(&item)?;
    Ok(make_action_result(
        true,
        AppManagerActionCode::AppManagerPermissionHelpOpened,
        "已打开系统权限设置入口",
        Some(item.name),
    ))
}

use super::*;

pub fn get_managed_app_detail(
    app: &dyn LauncherHost,
    query: AppManagerDetailQueryDto,
) -> AppResult<ManagedAppDetailDto> {
    let item = load_indexed_item(app, query.app_id.as_str())?;
    Ok(build_app_detail(item))
}

pub fn get_managed_app_detail_core(
    app: &dyn LauncherHost,
    query: AppManagerDetailQueryDto,
) -> AppResult<ManagedAppDetailDto> {
    get_managed_app_detail(app, query)
}

pub fn get_managed_app_detail_heavy(
    app: &dyn LauncherHost,
    input: AppManagerResidueScanInputDto,
) -> AppResult<AppManagerResidueScanResultDto> {
    super::residue::scan_managed_app_residue(app, input)
}

use super::*;

mod actions;
mod details;
mod indexing;
mod query;
mod residue;

pub use actions::*;
pub use details::*;
pub use indexing::*;
pub use query::*;
pub use residue::*;

pub(super) fn scan_cache_key(app_id: &str, mode: AppManagerResidueScanMode) -> String {
    let mode_key = match mode {
        AppManagerResidueScanMode::Quick => "quick",
        AppManagerResidueScanMode::Deep => "deep",
    };
    format!("{app_id}|{mode_key}")
}

pub(super) fn load_indexed_item(app: &dyn LauncherHost, app_id: &str) -> AppResult<ManagedAppDto> {
    let cache = load_or_refresh_index(app, false)?;
    find_indexed_item_in_cache(&cache, app_id)
}

pub(super) fn find_indexed_item_in_cache(
    cache: &AppIndexCache,
    app_id: &str,
) -> AppResult<ManagedAppDto> {
    cache
        .items
        .iter()
        .find(|candidate| candidate.id == app_id)
        .cloned()
        .ok_or_else(|| app_error(AppManagerErrorCode::NotFound, "应用不存在或索引已过期"))
}

pub(super) fn item_matches_keyword(item: &ManagedAppDto, keyword: Option<&str>) -> bool {
    let Some(keyword) = keyword else {
        return true;
    };
    contains_ignore_ascii_case(item.name.as_str(), keyword)
        || contains_ignore_ascii_case(item.path.as_str(), keyword)
        || item
            .publisher
            .as_deref()
            .is_some_and(|publisher| contains_ignore_ascii_case(publisher, keyword))
}

fn contains_ignore_ascii_case(haystack: &str, needle_lower: &str) -> bool {
    haystack.to_ascii_lowercase().contains(needle_lower)
}

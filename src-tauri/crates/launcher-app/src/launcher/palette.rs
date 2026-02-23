use app_core::AppResult;
use app_core::models::PaletteItemDto;
use crate::host::LauncherHost;
use crate::launcher::service::{execute_palette_legacy, search_palette_legacy};

pub fn search_palette(app: &dyn LauncherHost, query: &str) -> Vec<PaletteItemDto> {
    search_palette_legacy(app, query)
        .into_iter()
        .map(|item| PaletteItemDto {
            id: item.id,
            title: item.title,
            subtitle: item.subtitle,
            category: item.category,
        })
        .collect()
}

pub fn execute_palette_action(action_id: &str) -> AppResult<String> {
    execute_palette_legacy(action_id)
}

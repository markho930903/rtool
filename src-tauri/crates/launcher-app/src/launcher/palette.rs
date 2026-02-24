use crate::host::LauncherHost;
use crate::launcher::service::{execute_palette_action_id, search_palette_items};
use app_core::AppResult;
use app_core::models::PaletteItemDto;

pub fn search_palette(app: &dyn LauncherHost, query: &str) -> Vec<PaletteItemDto> {
    search_palette_items(app, query)
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
    execute_palette_action_id(action_id)
}

use crate::app::launcher_service::{execute_palette_legacy, search_palette_legacy};
use crate::core::models::{ActionResultDto, PaletteItemDto};
use tauri::AppHandle;

pub fn search_palette(app: &AppHandle, query: &str) -> Vec<PaletteItemDto> {
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

pub fn execute_palette_action(action_id: &str) -> ActionResultDto {
    execute_palette_legacy(action_id)
}

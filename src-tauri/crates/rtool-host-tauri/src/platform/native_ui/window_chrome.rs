use crate::constants::{CLIPBOARD_WINDOW_LABEL, LAUNCHER_WINDOW_LABEL, MAIN_WINDOW_LABEL};
use tauri::{AppHandle, Manager, Runtime};

const WINDOW_CORNER_RADIUS: f64 = 11.0;
const WINDOW_LABELS: [&str; 3] = [
    MAIN_WINDOW_LABEL,
    LAUNCHER_WINDOW_LABEL,
    CLIPBOARD_WINDOW_LABEL,
];

pub(crate) fn apply_window_chrome<R: Runtime>(
    app: &AppHandle<R>,
    transparent_window_background: bool,
) {
    #[cfg(target_os = "macos")]
    {
        for label in WINDOW_LABELS {
            let Some(window) = app.get_webview_window(label) else {
                continue;
            };

            if let Err(error) = apply_macos_window_effects(&window, transparent_window_background) {
                tracing::warn!(
                    event = "window_chrome_apply_failed",
                    window = label,
                    error = error.to_string()
                );
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }
}

#[cfg(target_os = "macos")]
fn apply_macos_window_effects<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    transparent_window_background: bool,
) -> tauri::Result<()> {
    use tauri::window::{Effect, EffectState, EffectsBuilder};

    if transparent_window_background {
        return window.set_effects(
            EffectsBuilder::new()
                .effect(Effect::Popover)
                .state(EffectState::Active)
                .radius(WINDOW_CORNER_RADIUS)
                .build(),
        );
    }

    window.set_effects(None)
}

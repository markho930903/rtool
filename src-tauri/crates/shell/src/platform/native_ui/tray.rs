use crate::constants::{
    TRAY_ICON_ID, TRAY_MENU_ID_CLIPBOARD, TRAY_MENU_ID_DASHBOARD, TRAY_MENU_ID_QUIT,
    TRAY_MENU_ID_TOOLS,
};
use crate::host::launcher::TauriLauncherHost;
use crate::platform::native_ui::windows::focus_main_window;
use foundation::i18n::t;
use foundation::models::LauncherActionDto;
use domain::launcher::service::execute_launcher_action;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{AppHandle, Runtime};

pub(crate) fn build_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    locale: &str,
) -> tauri::Result<Menu<R>> {
    let dashboard_label = t(locale, "tray.dashboard");
    let dashboard_item = MenuItem::with_id(
        app,
        TRAY_MENU_ID_DASHBOARD,
        &dashboard_label,
        true,
        None::<&str>,
    )?;
    let tools_label = t(locale, "tray.tools");
    let tools_item = MenuItem::with_id(app, TRAY_MENU_ID_TOOLS, &tools_label, true, None::<&str>)?;
    let clipboard_label = t(locale, "tray.clipboard");
    let clipboard_item = MenuItem::with_id(
        app,
        TRAY_MENU_ID_CLIPBOARD,
        &clipboard_label,
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_label = t(locale, "tray.quit");
    let quit_item = MenuItem::with_id(app, TRAY_MENU_ID_QUIT, &quit_label, true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &dashboard_item,
            &tools_item,
            &clipboard_item,
            &separator,
            &quit_item,
        ],
    )
}

pub(crate) fn refresh_tray_menu<R: Runtime>(app: &AppHandle<R>, locale: &str) {
    let Some(tray) = app.tray_by_id(TRAY_ICON_ID) else {
        return;
    };

    match build_tray_menu(app, locale) {
        Ok(menu) => {
            if let Err(error) = tray.set_menu(Some(menu)) {
                tracing::warn!(event = "tray_menu_update_failed", error = error.to_string());
            }
        }
        Err(error) => {
            tracing::warn!(event = "tray_menu_build_failed", error = error.to_string());
        }
    }

    if let Err(error) = tray.set_tooltip(Some(t(locale, "tray.tooltip"))) {
        tracing::warn!(
            event = "tray_tooltip_update_failed",
            error = error.to_string()
        );
    }

    if let Err(error) = tray.set_title(Option::<&str>::None) {
        tracing::warn!(event = "tray_title_clear_failed", error = error.to_string());
    }
}

fn run_tray_action(app: &AppHandle, action: LauncherActionDto, action_name: &str) {
    let host = TauriLauncherHost::new(app.clone());
    let result = execute_launcher_action(&host, &action);
    if let Err(error) = result {
        tracing::warn!(
            event = "tray_action_failed",
            action = action_name,
            error_code = error.code.as_str(),
            error_message = error.message.as_str(),
            error_detail = error.causes.first().map(String::as_str).unwrap_or_default()
        );
    }
}

pub(crate) fn handle_tray_menu(app: &AppHandle, menu_id: &str) {
    match menu_id {
        TRAY_MENU_ID_DASHBOARD => run_tray_action(
            app,
            LauncherActionDto::OpenBuiltinRoute {
                route: "/".to_string(),
            },
            "dashboard",
        ),
        TRAY_MENU_ID_TOOLS => run_tray_action(
            app,
            LauncherActionDto::OpenBuiltinRoute {
                route: "/tools".to_string(),
            },
            "tools",
        ),
        TRAY_MENU_ID_CLIPBOARD => run_tray_action(
            app,
            LauncherActionDto::OpenBuiltinWindow {
                window_label: "clipboard_history".to_string(),
            },
            "clipboard",
        ),
        TRAY_MENU_ID_QUIT => app.exit(0),
        _ => {}
    }
}

pub(crate) fn handle_tray_icon_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        focus_main_window(app);
    }
}

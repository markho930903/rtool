pub(crate) const CLIPBOARD_WINDOW_OPENED_EVENT: &str = "rtool://clipboard-window/opened";
pub(crate) const LAUNCHER_OPENED_EVENT: &str = "rtool://launcher/opened";

pub(crate) const SHORTCUT_LAUNCHER_PRIMARY: &str = "CommandOrControl+K";
pub(crate) const SHORTCUT_LAUNCHER_FALLBACK: &str = "Alt+Space";
pub(crate) const SHORTCUT_CLIPBOARD_WINDOW: &str = "Alt+V";
pub(crate) const SHORTCUT_CLIPBOARD_WINDOW_COMPACT: &str = "Alt+Shift+V";

pub(crate) const CLIPBOARD_WINDOW_LABEL: &str = "clipboard_history";
pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const LAUNCHER_WINDOW_LABEL: &str = "launcher";

pub(crate) const CLIPBOARD_COMPACT_WIDTH_LOGICAL: f64 = 560.0;
pub(crate) const CLIPBOARD_REGULAR_WIDTH_LOGICAL: f64 = 960.0;
pub(crate) const CLIPBOARD_MIN_HEIGHT_LOGICAL: f64 = 520.0;

pub(crate) const TRAY_ICON_ID: &str = "main-tray";
pub(crate) const TRAY_MENU_ID_DASHBOARD: &str = "tray.dashboard";
pub(crate) const TRAY_MENU_ID_TOOLS: &str = "tray.tools";
pub(crate) const TRAY_MENU_ID_CLIPBOARD: &str = "tray.clipboard";
pub(crate) const TRAY_MENU_ID_QUIT: &str = "tray.quit";

pub(crate) const CLIPBOARD_PLUGIN_UPDATE_EVENT: &str =
    "plugin:clipboard://clipboard-monitor/update";

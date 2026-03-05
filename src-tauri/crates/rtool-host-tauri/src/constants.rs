use rtool_kernel::WorkerId;

pub(crate) const CLIPBOARD_WINDOW_OPENED_EVENT: &str = "rtool://clipboard-window/opened";
pub(crate) const SCREENSHOT_WINDOW_OPENED_EVENT: &str = "rtool://screenshot-window/opened";
pub(crate) const SCREENSHOT_PIN_WINDOW_OPENED_EVENT: &str = "rtool://screenshot-pin-window/opened";
pub(crate) const SCREENSHOT_OPERATION_RESULT_EVENT: &str = "rtool://screenshot/operation-result";
pub(crate) const LAUNCHER_OPENED_EVENT: &str = "rtool://launcher/opened";

pub(crate) const SHORTCUT_LAUNCHER_PRIMARY: &str = "CommandOrControl+K";
pub(crate) const SHORTCUT_LAUNCHER_FALLBACK: &str = "Alt+Space";
pub(crate) const SHORTCUT_CLIPBOARD_WINDOW: &str = "Alt+V";
pub(crate) const SHORTCUT_CLIPBOARD_WINDOW_COMPACT: &str = "Alt+Shift+V";
pub(crate) const SHORTCUT_SCREENSHOT_DEFAULT: &str = "Alt+Shift+S";

pub(crate) const CLIPBOARD_WINDOW_LABEL: &str = "clipboard_history";
pub(crate) const MAIN_WINDOW_LABEL: &str = "main";
pub(crate) const LAUNCHER_WINDOW_LABEL: &str = "launcher";
pub(crate) const SCREENSHOT_WINDOW_LABEL: &str = "screenshot_overlay";
pub(crate) const SCREENSHOT_PIN_WINDOW_LABEL_1: &str = "screenshot_pin_1";
pub(crate) const SCREENSHOT_PIN_WINDOW_LABEL_2: &str = "screenshot_pin_2";
pub(crate) const SCREENSHOT_PIN_WINDOW_LABEL_3: &str = "screenshot_pin_3";
pub(crate) const SCREENSHOT_PIN_WINDOW_LABEL_4: &str = "screenshot_pin_4";
pub(crate) const SCREENSHOT_PIN_WINDOW_LABEL_5: &str = "screenshot_pin_5";
pub(crate) const SCREENSHOT_PIN_WINDOW_LABEL_6: &str = "screenshot_pin_6";
pub(crate) const SCREENSHOT_PIN_WINDOW_LABELS: [&str; 6] = [
    SCREENSHOT_PIN_WINDOW_LABEL_1,
    SCREENSHOT_PIN_WINDOW_LABEL_2,
    SCREENSHOT_PIN_WINDOW_LABEL_3,
    SCREENSHOT_PIN_WINDOW_LABEL_4,
    SCREENSHOT_PIN_WINDOW_LABEL_5,
    SCREENSHOT_PIN_WINDOW_LABEL_6,
];

pub(crate) const CLIPBOARD_COMPACT_WIDTH_LOGICAL: f64 = 560.0;
pub(crate) const CLIPBOARD_REGULAR_WIDTH_LOGICAL: f64 = 960.0;
pub(crate) const CLIPBOARD_MIN_HEIGHT_LOGICAL: f64 = 520.0;

pub(crate) const TRAY_ICON_ID: &str = "main-tray";
pub(crate) const TRAY_MENU_ID_TOOLS: &str = "tray.tools";
pub(crate) const TRAY_MENU_ID_CLIPBOARD: &str = "tray.clipboard";
pub(crate) const TRAY_MENU_ID_QUIT: &str = "tray.quit";

pub(crate) const CLIPBOARD_PLUGIN_UPDATE_EVENT: &str =
    "plugin:clipboard://clipboard-monitor/update";

pub(crate) const RUNTIME_WORKER_CLIPBOARD: WorkerId = WorkerId::Clipboard;
pub(crate) const RUNTIME_WORKER_APP_MANAGER: WorkerId = WorkerId::AppManager;
pub(crate) const RUNTIME_WORKER_SCREENSHOT: WorkerId = WorkerId::Screenshot;
pub(crate) const RUNTIME_WORKER_LAUNCHER: WorkerId = WorkerId::Launcher;

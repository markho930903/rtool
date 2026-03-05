use std::env;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct RuntimeBudget {
    pub screenshot_archive_concurrency: usize,
    pub screenshot_clipboard_concurrency: usize,
    pub app_manager_poll_base_secs: u64,
    pub app_manager_poll_min_secs: u64,
    pub app_manager_poll_max_secs: u64,
}

impl Default for RuntimeBudget {
    fn default() -> Self {
        Self {
            screenshot_archive_concurrency: 2,
            screenshot_clipboard_concurrency: 2,
            app_manager_poll_base_secs: 20,
            app_manager_poll_min_secs: 5,
            app_manager_poll_max_secs: 120,
        }
    }
}

fn parse_usize(key: &str, default: usize, min: usize, max: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn parse_u64(key: &str, default: u64, min: u64, max: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

impl RuntimeBudget {
    fn from_env() -> Self {
        let mut budget = Self {
            screenshot_archive_concurrency: parse_usize(
                "RTOOL_SCREENSHOT_ARCHIVE_CONCURRENCY",
                2,
                1,
                8,
            ),
            screenshot_clipboard_concurrency: parse_usize(
                "RTOOL_SCREENSHOT_CLIPBOARD_CONCURRENCY",
                2,
                1,
                8,
            ),
            app_manager_poll_base_secs: parse_u64("RTOOL_APP_MANAGER_POLL_BASE_SECS", 20, 2, 300),
            app_manager_poll_min_secs: parse_u64("RTOOL_APP_MANAGER_POLL_MIN_SECS", 5, 1, 60),
            app_manager_poll_max_secs: parse_u64("RTOOL_APP_MANAGER_POLL_MAX_SECS", 120, 5, 600),
        };

        if budget.app_manager_poll_min_secs > budget.app_manager_poll_max_secs {
            std::mem::swap(
                &mut budget.app_manager_poll_min_secs,
                &mut budget.app_manager_poll_max_secs,
            );
        }
        budget.app_manager_poll_base_secs = budget.app_manager_poll_base_secs.clamp(
            budget.app_manager_poll_min_secs,
            budget.app_manager_poll_max_secs,
        );

        budget
    }

    pub fn global() -> &'static Self {
        static BUDGET: OnceLock<RuntimeBudget> = OnceLock::new();
        BUDGET.get_or_init(|| {
            let budget = RuntimeBudget::from_env();
            tracing::info!(
                event = "runtime_budget_loaded",
                screenshot_archive_concurrency = budget.screenshot_archive_concurrency,
                screenshot_clipboard_concurrency = budget.screenshot_clipboard_concurrency,
                app_manager_poll_base_secs = budget.app_manager_poll_base_secs,
                app_manager_poll_min_secs = budget.app_manager_poll_min_secs,
                app_manager_poll_max_secs = budget.app_manager_poll_max_secs
            );
            budget
        })
    }
}

use foundation::models::ResourceModuleIdDto;
use foundation::{
    MonitorOptions, initialize_global_monitor, record_module_observation, reset_session, snapshot,
};

#[test]
fn launcher_submodule_observations_should_be_visible_in_snapshot() {
    initialize_global_monitor(MonitorOptions::default());
    reset_session();

    record_module_observation(ResourceModuleIdDto::LauncherIndex, true, 12);
    record_module_observation(ResourceModuleIdDto::LauncherFallback, true, 64);
    record_module_observation(ResourceModuleIdDto::LauncherCache, false, 8);

    let snapshot = snapshot();
    let by_id = snapshot
        .modules
        .into_iter()
        .map(|item| (item.module_id, item))
        .collect::<std::collections::HashMap<_, _>>();

    let index_stats = by_id
        .get(&ResourceModuleIdDto::LauncherIndex)
        .expect("launcher_index module should exist");
    assert_eq!(index_stats.calls, 1);
    assert_eq!(index_stats.error_calls, 0);
    assert_eq!(index_stats.avg_duration_ms, Some(12.0));

    let fallback_stats = by_id
        .get(&ResourceModuleIdDto::LauncherFallback)
        .expect("launcher_fallback module should exist");
    assert_eq!(fallback_stats.calls, 1);
    assert_eq!(fallback_stats.error_calls, 0);

    let cache_stats = by_id
        .get(&ResourceModuleIdDto::LauncherCache)
        .expect("launcher_cache module should exist");
    assert_eq!(cache_stats.calls, 1);
    assert_eq!(cache_stats.error_calls, 1);
    assert_eq!(cache_stats.avg_duration_ms, Some(8.0));
}

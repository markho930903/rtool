# 命令层治理约定（Rust / src-tauri/src/commands）

## 目标
- 消除每个 tauri command 中重复的 `normalize_request_id + command_start + command_end_*` 样板。
- 统一命令埋点行为，降低遗漏和行为漂移风险。

## 约定
- 优先使用 `commands/mod.rs` 提供的包装器：
  - `run_command_sync`
  - `run_command_async`
  - `run_blocking_command`（新增，统一 `run_blocking` + 命令埋点）
- command 函数主体只保留业务流程，不再手写开始/结束日志。
- 阻塞任务优先通过 `run_blocking_command` 执行；复杂场景可在 `run_command_async` 闭包内按步骤调用多个 `run_blocking`。

## 本轮覆盖模块
- 已统一：`app_manager.rs`、`clipboard.rs`、`dashboard.rs`、`i18n_import.rs`、`launcher.rs`、`locale.rs`、`logging.rs`、`palette.rs`、`transfer.rs`。

## 注意点
- 对仅返回 `Ok(...)` 的闭包，必要时显式标注错误类型（如 `Ok::<T, AppError>(...)`），避免泛型推断失败。
- `client_log` 保持原有业务日志路径（仍可内部使用 `run_blocking`）。

## 传输发现补充约定
- `transfer_service::start_discovery` 启动前应清理已结束任务句柄，避免异常退出后因句柄残留导致启动被误判为“已启动”。
- 在 Tauri `JoinHandle` 下可使用 `task.inner().is_finished()` 进行判断。
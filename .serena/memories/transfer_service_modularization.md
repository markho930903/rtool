# transfer_service 模块治理（第八至第二十三阶段）

## 第八至第二十二阶段摘要
- 完成 transfer_service 多轮模块化与治理：
  - 握手/manifest 拆分
  - incoming/outgoing 主循环编排器化
  - session 控制状态机集中
  - 终态持久化参数统一（`TerminalPersistOptions` + `TransferHistorySyncReason`）
  - outgoing 运行态对象化（`OutgoingLoopState`）
  - outgoing ACK 链路异步协议测试（duplex）

## 第二十三阶段：incoming 读帧链路异步协议测试补齐

### 目标
- 为 incoming 读帧行为建立独立的协议级测试护栏（正常/超时/连接断开），避免只依赖带副作用的业务方法测试。

### 结构变化
- `incoming_pipeline.rs`
  - 新增 `poll_incoming_frame_raw(...)`：
    - 输入：`reader + session_key + codec`
    - 行为：40ms timeout + `read_frame_from`，返回 `Option<TransferFrame>` 或错误
    - 无业务副作用（不做 session 持久化）
  - `poll_incoming_frame(...)` 改为调用 `poll_incoming_frame_raw(...)`，在错误分支继续执行既有终态收口（`finalize_incoming_read_failed`）。

### 新增测试（tokio + duplex）
- `poll_incoming_frame_raw_should_return_none_on_timeout`
- `poll_incoming_frame_raw_should_parse_incoming_frame`
- `poll_incoming_frame_raw_should_map_connection_closed_error`

### 验证
- `cargo check` 通过。
- `cargo test` 通过，最新 `116 passed`。

### 当前收益
- incoming 读帧协议层与业务终态副作用层解耦，更易定位“协议问题 vs 业务问题”。
- 与 outgoing ACK 异步测试形成对称覆盖，协议层回归防线更完整。

### 下一步建议（继续大步）
- 增加基于受控时间的 timeout/requeue 时序测试（重点覆盖 `collect_timeout_chunks + requeue_timeout_chunks` 的边界行为）。
- 继续评估并收敛 `incoming/outgoing` 的 finalize helper 到更高层共享抽象，减少跨文件重复。
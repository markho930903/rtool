# Anyhow 错误处理模式（后端）

- 命令边界统一返回 `Result<T, InvokeError>`，内部保留 `AppResult<T>`，边界使用 `result.map_err(Into::into)`。
- `AppError` 采用轻量壳（`AppError(Box<AppErrorPayload>)`）避免 `clippy::result_large_err`，并通过 `Deref` 保持字段访问兼容。
- `AppError::from_anyhow` 与 `InvokeError::from_anyhow` 保留链路信息（`error.chain()`），发布模式下对敏感原因做脱敏。
- 业务层推荐模式：
  - 外部 I/O/系统调用：`with_context(...)`（anyhow）+ `with_kind(...)`（业务错误码）+ `with_ctx(...)`（结构化上下文）。
  - 参数/状态校验：优先 `anyhow::ensure!` / `anyhow::bail!`，再映射业务错误语义。
- 在 `infrastructure/logging.rs`、`app_manager_service/residue_cleanup.rs`、`transfer_service/outgoing.rs` 已落地上述模式。
- 新增约束：优先使用 `with_context("key", value)` 记录结构化字段（如 `status/path/label/sessionId/fileId/peerDeviceId`），尽量避免 `with_detail(format!(...))`。
- 校验类错误（如 level/cursor/locale）推荐携带机器可读上下文键（`level` / `cursor` / `preference`），保持错误码稳定。
- `with_detail` 仅保留给兼容宏与少数确实需要纯文本 detail 的场景，常规链路默认不用。

- 新能力：`AppError` 增加 `with_source` / `with_boxed_source` / `with_anyhow_source`，可在错误边界保留 source chain（而非 `error.to_string()`）。
- `with_source` 会附带结构化上下文：`sourceType` 与 `sourceChainDepth`，用于排障与观测。
- 对于仅有字符串错误（如部分插件 API 返回 `String`），继续使用 `with_cause`，并优先补齐业务上下文字段。

- 最终契约已收敛为 `code/message/context/causes/requestId`，移除 `kind/detail` 与对应回退逻辑。
- `ResultExt` 统一为 `with_code` + `with_ctx`；不再使用 `with_kind`。
- `commands/mod.rs` 与前端 `src/services/invoke.ts` 已同步仅记录/消费 `errorCode`。

# 完成任务前的最小验证清单

- `pnpm format:check`
- `pnpm lint`
- `pnpm build`

如改动涉及 Tauri/Rust 逻辑：
- 在 `src-tauri/` 下运行 `cargo test` / `cargo clippy`

如改动涉及窗口 UI（launcher/clipboard_history）：
- `pnpm tauri dev` 打开对应窗口进行一次手动回归（检查圆角、透明/玻璃效果、点击/键盘交互）。

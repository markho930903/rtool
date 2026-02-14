# rtool 常用命令

## 前端（pnpm）
- 开发（仅前端）：`pnpm dev`
- 构建：`pnpm build`
- 预览构建产物：`pnpm preview`
- Lint：`pnpm lint`
- Lint 自动修复：`pnpm lint:fix`
- 格式化：`pnpm format`
- 格式检查：`pnpm format:check`
- i18n 检查：`pnpm i18n:check`

## Tauri
- 启动桌面端开发：`pnpm tauri dev`（会在 dev 时先运行 `pnpm dev`，见 `src-tauri/tauri.conf.json`）
- 构建桌面端：`pnpm tauri build`（会在 build 前运行 `pnpm build`）

## Rust（src-tauri）
- 测试：`cargo test`（在 `src-tauri/` 下）
- Clippy：`cargo clippy`（在 `src-tauri/` 下）
- 格式化：`cargo fmt`（在 `src-tauri/` 下）

# Tauri 入口解耦结构（lib.rs 防膨胀）

## 目标
- `src-tauri/src/lib.rs` 仅作为入口与兼容转发层。
- 业务逻辑和平台编排迁出，避免后续继续堆积到 `lib.rs`。

## 当前模块边界
- `src-tauri/src/lib.rs`
  - 仅保留：模块声明、`run()`、`apply_locale_to_native_ui` 转发、`apply_clipboard_window_mode` 转发。
- `src-tauri/src/bootstrap/`
  - `mod.rs`: `AppBootstrap::run()`，负责插件拼装、窗口关闭行为、运行上下文。
  - `setup.rs`: 初始化日志、i18n、数据库、tray、AppState、clipboard watcher。
  - `invoke.rs`: `invoke_handler` 命令清单集中注册。
- `src-tauri/src/native_ui/`
  - `tray.rs`: 托盘菜单构建与事件处理。
  - `windows.rs`: 主窗口/launcher 窗口行为、标题刷新。
  - `shortcuts.rs`: 全局快捷键分发与剪贴板窗口快捷键行为。
  - `clipboard_window.rs`: 剪贴板窗口模式尺寸/位置应用逻辑。
- `src-tauri/src/clipboard_watcher/`
  - `mod.rs`: watcher 启动与事件监听。
  - `processor.rs`: 文本/文件/图片剪贴板更新处理。
  - `image_preview.rs`: 图片签名、尺寸读取、预览落盘。
- `src-tauri/src/constants.rs`
  - 统一维护事件名、快捷键、tray id、窗口 label 常量。

## 兼容约定
- 维持 `crate::apply_locale_to_native_ui` 与 `crate::apply_clipboard_window_mode` 入口，避免 `commands/*` 和 `app/*` 现有调用大面积修改。
- `invoke_handler` 命令列表保持与重构前一致。

## 后续约束（建议）
- 新增逻辑不得直接写入 `lib.rs`，应按职责落在 `bootstrap/native_ui/clipboard_watcher`。
- 新事件名或快捷键优先落 `constants.rs`，禁止字符串散写。
- 触发跨层调用时优先通过 `pub(crate)` 小接口，不直接暴露实现细节。
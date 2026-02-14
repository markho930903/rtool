# rtool 项目概览

## 目的
- 桌面端工具应用：Tauri + React + TypeScript（Vite），Rust 侧提供系统能力（剪贴板、快捷键、托盘等）。

## 技术栈
- 前端：React 19、TypeScript（strict）、react-router（HashRouter）、zustand
- 构建：Vite
- 样式：UnoCSS（项目默认“UnoCSS-first”），主题 token 主要在 `src/styles/theme.css`
- Rust：Tauri 2（`src-tauri/`），SQLite 相关依赖（rusqlite/r2d2 等）

## 目录结构（高层）
- `src/`：前端代码
  - `src/pages/`：页面（含独立窗口页 `ClipboardWindowPage.tsx` / `LauncherWindowPage.tsx`）
  - `src/layouts/`：布局（`AppLayout.tsx`）
  - `src/components/`：组件（含 `src/components/ui/` 组件库）
  - `src/styles/theme.css`：主题颜色/阴影/圆角等 CSS 变量
- `src-tauri/`：Tauri + Rust 后端
  - `src-tauri/tauri.conf.json`：窗口配置（含 `launcher` / `clipboard_history` 透明无边框窗口）

## 入口
- 前端入口：`src/main.tsx`（加载 `uno.css` 与 `src/styles/theme.css`）
- 路由：`src/routers/index.tsx`（HashRouter + routes）

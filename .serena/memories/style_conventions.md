# rtool 风格与约定

## TypeScript/React
- TypeScript `strict: true`（见 `tsconfig.json`），避免 unused locals/params。
- 路由使用 `react-router`（HashRouter）。
- 路径别名：`@/* -> src/*`。

## 格式化/Lint
- 格式化：`oxfmt`（printWidth 120、semi、double quotes、trailingComma all，见 `.oxfmtrc.json`）
- Lint：`oxlint`（react/typescript/import 等插件，见 `.oxlintrc.json`）

## 样式（UnoCSS-first）
- 优先使用 UnoCSS utility/shortcuts；主题 token 集中在 `src/styles/theme.css` 的 CSS 变量。
- UnoCSS 配置在 `uno.config.ts`：
  - 颜色与圆角来自 CSS 变量（`--color-*`, `--radius-*`）。
  - `launcher` / `clipboard_history` 窗口的 `body/#root` 背景默认透明（通过 `data-window-label` 选择器）。

# rtool 主题 Token 规范

本文件定义前端样式系统的唯一来源与使用约束。

## 1. 单一来源

1. 主题 token 统一定义在 `src/styles/theme.css`。
2. 业务组件只使用语义类（如 `bg-surface`、`text-danger`、`shadow-overlay`），不直接写原始色值。
3. UnoCSS 通过 `uno.config.ts` 把 token 映射为可复用类名。

## 2. 分层模型

1. Primitive（原子值）：
   `--primitive-*`，只存储原始设计值（颜色、字号、间距、圆角）。
2. Semantic（语义值）：
   `--color-*`、`--font-size-*`、`--space-*`、`--radius-*`，描述用途，不描述视觉名字。
3. Component（组件值）：
   `--shadow-*`、局部高阶语义 token，用于具体组件态（浮层、侧栏选中态等）。

## 3. 命名约束

1. 颜色语义：
   `color-bg-*`、`color-surface-*`、`color-text-*`、`color-border-*`、`color-accent`、`color-danger`。
   图表语义使用 `color-chart-*` 前缀（轴线、网格、图例、系列、tooltip）。
2. 尺寸语义：
   `font-size-ui-*`、`line-height-ui-*`、`space-*`、`radius-*`。
3. 状态语义：
   使用 `accent` / `danger` / `info`，避免直接引用调色板色名。

## 4. 业务层禁止项

1. 禁止使用非主题色类名：
   `text-red-*`、`bg-blue-*`、`text-white` 等。
2. 禁止直接写颜色函数：
   `rgb(...)`、`rgba(...)`、`hsl(...)`、`hsla(...)`。
3. 禁止在业务组件中使用 `[...]` 直接读取 `var(--color-*)` / `var(--shadow-*)` / `var(--radius-*)`。
4. 禁止在 `shadow-[...]` 中直接拼接颜色函数。
5. AntV/G2 图表禁止在页面业务文件直接读取任意 `--color-*`；统一经 `src/theme/chartTheme.ts` 输出主题配置。

说明：`theme.css` 是 token 定义源，允许包含原始颜色函数。

## 5. 合规检查

运行：

```bash
pnpm tokens:check
```

该命令会扫描 `src/**/*.{ts,tsx,css}`，对违反规则的样式写法直接报错并阻断提交前检查。

## 6. 迁移建议

1. 先加 token，再替换类名，最后清理旧写法。
2. 先改高频复用组件（Button/Input/Select），再改页面级业务样式。
3. 每次迁移后执行 `pnpm lint && pnpm build && pnpm tokens:check`。

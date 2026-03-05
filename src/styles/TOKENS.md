# rtool 主题 Token 规范

本文件定义前端样式系统的唯一来源与使用约束。

## 1. 单一来源

1. 主题 token 统一定义在 `src/styles/theme.css`。
2. 业务组件只使用语义类（如 `bg-surface`、`text-danger`、`shadow-overlay`），不直接写原始色值。
3. UnoCSS 通过 `uno.config.ts` 把 token 映射为可复用类名。
4. 主题风格采用单一语义 token 体系，透明模式通过参数层调节，不复制第二套语义 token。
5. 当前主题基调：黑灰中性体系；深色以 `#181818` 与 `#2a2a2a` 为核心，透明模式通过参数层调节，深色布局区保持黑色策略。

## 2. 分层模型

1. Primitive（原子值）：
   `--primitive-*`，只存储原始设计值（颜色、字号、间距、圆角）。
2. Semantic（语义值）：
   `--color-*`、`--font-size-*`、`--space-*`、`--radius-*`，描述用途，不描述视觉名字。
   结构区语义（layout）使用 `--color-layout-*`，仅用于窗口框架区域（如主 layout 的标题栏/侧边栏）。
3. Component（组件值）：
   `--shadow-*`、局部高阶语义 token，用于具体组件态（浮层、侧栏选中态等）。
4. Mode Parameter（模式参数）：
   `--glass-alpha*`、`--glass-*`、`--window-root-*`、`--window-*opacity`，用于透明模式调参。

## 3. 命名约束

1. 颜色语义：
   `color-bg-*`、`color-surface-*`、`color-text-*`、`color-border-*`、`color-layout-*`、`color-accent`、`color-danger`。
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
5. 图表禁止在页面业务文件直接读取任意 `--color-*`；应通过主题封装层输出配置。

说明：`theme.css` 是 token 定义源，允许包含原始颜色函数。

## 5. 透明模式规则（统一 token + 参数层）

1. 透明模式（`data-window-transparency="on"`）默认只允许覆盖参数层变量，不允许覆盖语义颜色 token（`--color-*`）与语义阴影 token（`--shadow-*`）。
   例外：允许覆盖结构区语义 token `--color-layout-*`，用于窗口框架区域的专用视觉策略。
2. 允许覆盖变量白名单：
   `--glass-alpha`、`--glass-alpha-strong`、`--glass-alpha-surface`、`--glass-alpha-soft`、`--glass-alpha-overlay`、`--glass-blur`、`--glass-saturate`、`--glass-brightness`、`--window-root-background`、`--window-root-border-color`、`--window-root-blur`、`--window-root-saturate`、`--window-root-brightness`、`--window-frost-noise-opacity`、`--window-atmosphere-opacity`、`--window-sheen-opacity`。
   结构区例外白名单：`--color-layout-sidebar-bg`、`--color-layout-titlebar-bg`、`--color-layout-divider`。
3. 深浅模式共享同一套语义 token 命名与用途，透明开关只改变呈现强度，不改变语义含义。
4. 深色主题在 `data-window-transparency="off"` 下允许独立参数校准，以贴近 `on` 状态的视觉层级，但仍禁止改写语义颜色 token（`--color-*`）与语义阴影 token（`--shadow-*`）。

## 6. 合规检查

运行：

```bash
pnpm tokens:check
```

该命令会扫描 `src/**/*.{ts,tsx,css}`，对违反规则的样式写法直接报错并阻断提交前检查。

## 7. 迁移建议

1. 先加 token，再替换类名，最后清理旧写法。
2. 先改高频复用组件（Button/Input/Select），再改页面级业务样式。
3. 每次迁移后执行 `pnpm lint && pnpm build && pnpm tokens:check`。

## 8. 滚动条 Token

1. 滚动条样式统一由 `theme.css` 的全局 token 控制，禁止在业务组件单独定义 `::-webkit-scrollbar*`。
2. 尺寸 token：
   `--scrollbar-size`、`--scrollbar-radius`、`--scrollbar-thumb-min-size`。
3. 颜色与阴影 token：
   `--color-scrollbar-thumb`、`--color-scrollbar-thumb-hover`、`--color-scrollbar-thumb-active`、`--color-scrollbar-thumb-edge`、`--shadow-scrollbar-thumb`。
4. 轨道（track）应保持透明，默认 thumb 隐藏；仅在 hover/focus/active 交互态显现液态玻璃效果。

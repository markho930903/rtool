# 主窗口玻璃风格对齐启动器窗口

## 目标
- 主窗口、启动器窗口、剪贴板窗口使用统一液态玻璃视觉基准。
- 标题栏菜单弹层不得被标题栏裁切。
- 主窗口窗口圆角与页面圆角保持一致。

## 关键实现
- `theme.css` 引入/统一玻璃语义 token：`--color-surface-glass-*`、`--color-border-glass*`、`--glass-blur`、`--glass-saturate`、`--color-specular`。
- 深色主题提亮：`--color-bg-app` 与玻璃表面/阴影透明度降低暗度。
- 氛围层 `.rtool-glass-atmosphere` 下调发光范围与不透明度，避免主背景过深。
- sheen 拆分为：
  - `.rtool-glass-sheen-clip`：用于封闭圆角容器（保留 `overflow: hidden`）
  - `.rtool-glass-sheen-open`：用于标题栏/弹层（`overflow: visible`）
- `AppLayout` 主窗口壳层与内容容器统一使用 `rounded-md`，包含 sidebar 与 titlebar 两种布局分支。
- `uno.config.ts` 中 `html[data-window-label="main"] body` 与 `#root` 设置 `border-radius: var(--radius-md)` + `overflow: hidden`，确保窗口圆角与页面圆角同步。
- 标题栏菜单弹层统一为玻璃面板标准：`rounded-md` + `border-border-glass` + `bg-surface-glass-strong` + `shadow-overlay` + 统一内边距。
- 菜单浮窗小箭头已移除，保留 Portal 定位与键盘交互。
- 启动器列表项选中/悬停视觉与主窗口菜单项对齐（边框、背景、阴影语义一致）。

## 回归关注点
- 标题栏菜单弹层层级、宽度与滚动
- 主窗口四角裁切是否与系统窗口圆角一致
- 启动器条目选中/悬停对比度与可读性

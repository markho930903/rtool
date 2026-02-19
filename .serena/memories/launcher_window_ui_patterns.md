# Launcher 窗口置顶交互模式

- `LauncherWindowPage` 新增窗口级置顶状态：`alwaysOnTop` + `alwaysOnTopRef`。
- 打开 launcher 时（`rtool://launcher/opened`）调用 `appWindow.isAlwaysOnTop()` 同步 UI 状态。
- 切换置顶通过 `appWindow.setAlwaysOnTop(next)`，成功后更新状态；当 `next=true` 时调用 `cancelScheduledHide()`。
- `useWindowFocusAutoHide` 传入 `shouldSkipHide: () => alwaysOnTopRef.current`，实现置顶时失焦不自动隐藏。
- 搜索输入组件 `PaletteInput` 扩展可选插槽 `trailingActions?: ReactNode`，用于在输入框右侧插入窗口级动作按钮（launcher 使用置顶按钮，其他调用方可不传）。
- 置顶文案放在 `palette` 命名空间：
  - `launcher.pinWindowOn`
  - `launcher.pinWindowOff`

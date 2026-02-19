# 托盘左键与主窗口导航行为

## 变更背景
用户希望主窗口保留操作痕迹，避免每次点击托盘图标进入主窗口都触发页面初始化感。

## 已实现策略
- 托盘左键点击：仅 `show + focus` 主窗口，不再触发 `dashboard` 路由动作。
  - 位置：`src-tauri/src/lib.rs`
  - 实现：新增 `focus_main_window`，`handle_tray_icon_event` 左键分支调用该函数。
- 托盘菜单“仪表盘”：保持原行为，仍通过 `OpenBuiltinRoute("/")` 强制回首页。

## 前端保护
- `AppEventBridge` 对 `rtool://main/navigate` 增加同路由幂等检查：
  - 若 `event.payload.route` 与当前路由一致，则忽略本次导航。
  - 位置：`src/App.tsx`

## 回归关注点
- 左键托盘唤起主窗口时，应保留当前页面与页面内状态（如 `/tools` 查询参数与交互痕迹）。
- 菜单“仪表盘”仍应显式回到 `/`。
- 关注日志：`window_not_found` / `window_show_failed` / `window_focus_failed`。
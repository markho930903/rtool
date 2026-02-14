# Clipboard 窗口同步模式约定

## 触发语义
- 全局快捷键：`Alt+V` 打开普通模式；`Alt+Shift+V` 打开简洁模式。
- 两个快捷键在窗口可见时均为 toggle hide（再次按下隐藏窗口）。
- 窗口打开事件 `rtool://clipboard-window/opened` 使用 payload：`{ compact: boolean }`。

## 历史数据同步策略
- 前端 store 采用 `ensureInitialized()`：应用会话内仅初始化一次。
- 初始化时先调用 `clipboard_get_settings`，再按 `maxItems` 作为 `clipboard_list.limit` 拉取。
- 打开窗口不触发重拉；搜索/类型/仅置顶在前端本地过滤。

## 增量事件协议
- 新增事件：`rtool://clipboard/sync`。
- payload：`{ upsert: ClipboardItemDto[], removedIds: string[], clearAll: boolean, reason?: string }`。
- 处理顺序：`clearAll -> removedIds -> upsert`。
- 兼容期保留旧事件 `rtool://clipboard/updated`，仅作为 upsert 映射。

## Rust 实现注意点
- `db::prune_clipboard_items` 返回被裁剪项（含 id 和 preview_path），用于构造 `removedIds`。
- `ClipboardService::save_text/save_item/update_settings` 需要返回 `removed_ids`。
- 对会影响集合一致性的命令都应补发 `clipboard/sync`：pin/delete/clear/copy_back/copy_image_back/update_settings。

## 风险点
- 若新增命令改变历史集合但未发 `clipboard/sync`，前端会出现局部不一致。
- 若提高 `maxItems` 上限，需确认 `clipboard_list` 的 limit clamp 与 UI 初始化策略一致。
# LoadingIndicator 组件约定

- `InlineLoading` 已重命名为 `LoadingIndicator`，文件路径为 `src/components/loading/LoadingIndicator.tsx`。
- 组件支持两种模式：`mode="inline"`（默认）与 `mode="overlay"`。
- `overlay` 模式默认行为：遮罩开启（`showMask=true`）、阻断交互（`blockInteraction=true`）、内容在包裹区域内水平垂直居中。
- `overlay` 模式支持无 children 场景：会渲染最小高度容器（默认 `min-h-24`，可通过 `minHeightClassName` 自定义）来居中展示 loading。
- 导出入口改为 `src/components/loading/index.ts` 中的 `LoadingIndicator`、`LoadingIndicatorProps`、`LoadingIndicatorSize`、`LoadingIndicatorMode`。
- 典型区域级用法可参考 `src/pages/LogCenterPage.tsx`：用 `LoadingIndicator` 包裹列表容器并在加载时显示居中 overlay。
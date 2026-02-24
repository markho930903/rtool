# rtool - 桌面效率工具箱

基于 Tauri + React + TypeScript 构建的桌面效率工具应用，提供启动器、剪贴板管理、实用工具等功能。

## 功能特性

### 启动器

- 快速搜索和启动应用
- 快捷键：`Cmd/Ctrl + K`

### 剪贴板历史

- 自动记录剪贴板历史
- 支持常规/紧凑两种模式
- 快捷键：`Alt + V`（常规）/ `Alt + Shift + V`（紧凑）

### 工具箱

- **Base64** - 编码/解码
- **正则表达式** - 实时测试
- **时间戳** - 转换工具

### 仪表盘

- 应用内存监控
- 运行时间统计
- 数据库大小
- 系统信息（CPU、内存、内核版本等）

### 资源监控

- 独立资源监控页（主导航入口）
- 按功能模块 / crate 归因展示 CPU、内存、调用耗时
- 1 秒采样与趋势图可视化（ANTV G2）

## 技术栈

| 层级     | 技术                  |
| -------- | --------------------- |
| 前端框架 | React 19 + TypeScript |
| 构建工具 | Vite 7                |
| 样式方案 | UnoCSS                |
| 状态管理 | Zustand               |
| 路由     | React Router 7        |
| 国际化   | i18next               |
| 后端框架 | Tauri 2.x             |
| 后端语言 | Rust                  |
| 数据库   | Turso (libsql 本地模式) |

## 快速开始

### 环境要求

- Node.js 18+
- pnpm 8+
- Rust 1.75+

### 安装依赖

```bash
pnpm install
```

### 开发模式

```bash
# 启动前端开发服务器
pnpm dev

# 启动 Tauri 开发模式
pnpm tauri dev
```

### 构建发布

```bash
pnpm tauri build
```

## 项目结构

```
rtool/
├── src/                    # 前端源码
│   ├── components/        # React 组件
│   ├── pages/             # 页面组件
│   ├── stores/            # Zustand 状态管理
│   ├── services/          # 业务服务
│   ├── contracts/         # 手动维护的 TS 契约单一事实源
│   ├── hooks/             # React Hooks
│   ├── i18n/              # 国际化配置
│   └── styles/            # 样式文件
├── src-tauri/             # Tauri/Rust 后端
│   ├── src/
│   │   ├── main.rs        # 薄入口
│   │   └── lib.rs         # 薄桥接
│   ├── crates/
│   │   ├── core/          # 协议/DTO/错误模型
│   │   ├── infra/         # DB/日志/平台能力
│   │   ├── launcher-app/  # 启动器与应用管理领域
│   │   │   └── src/
│   │   │       ├── launcher_service.rs
│   │   │       ├── launcher_index_service.rs
│   │   │       ├── icon_service.rs
│   │   │       ├── palette_service.rs
│   │   │       └── app_manager_service/
│   │   ├── clipboard/     # 剪贴板领域
│   │   │   └── src/clipboard_service.rs
│   │   ├── transfer/      # 传输领域
│   │   │   └── src/transfer_service/
│   │   └── tauri-shell/   # Tauri 壳层（命令注册与平台适配）
│   └── Cargo.toml         # workspace
└── package.json
```

## 代码规范

- 前端代码格式化：`pnpm format`
- 代码检查：`pnpm lint`
- 国际化检查：`pnpm i18n:check`
- 契约维护：`src/contracts/index.ts`（手动同步后端 DTO 与命令名）

## 启动器索引策略

- 策略状态键：`launcher.search.scope_policy_applied`
- 当前状态：`applied`
- 迁移规则：状态未生效时，启动器会一次性强制覆盖历史 `roots`（仅覆盖 roots，其他阈值配置保持不变）

平台默认范围：

- Windows/macOS：`用户常用目录 + 应用目录 + 系统根目录`
- Linux：保守默认行为（不扩展为 Win/mac 的完整三层集合）

## 数据库说明

- 当前数据库为 Turso 本地模式（`libsql`），库文件为 `rtool-turso.db`。
- 启动时在新库初始化成功后，会自动尝试清理历史 SQLite 文件：
  - `rtool.db`
  - `rtool.db-wal`
  - `rtool.db-shm`
- 当前版本不提供旧 SQLite 数据自动迁移，请按“新库冷启动”处理历史数据。

### 本地 Turso 性能策略

- 全链路采用异步 `libsql` 访问（命令层/服务层/存储层不再使用 DB 同步桥接）。
- 初始化启用 `PRAGMA foreign_keys=ON`、`WAL`、`synchronous=NORMAL`、`temp_store=MEMORY`、`busy_timeout`，并在建表后执行 `PRAGMA optimize`。
- 高频配置读写采用批量访问，减少多次往返与 autocommit 开销。
- 日志关键词检索优先走 `FTS5`（`log_entries_fts`），`LIKE` 仅作为回退路径。
- 历史清理依赖外键级联删除（`transfer_sessions -> transfer_files`），避免重复清理语句。

### 为什么当前不接 Turso 云端

- 当前版本目标是桌面端本地优先与冷启动稳定，不引入网络依赖、鉴权与同步复杂度。
- 因此本版本不使用 `TURSO_DATABASE_URL` / `TURSO_AUTH_TOKEN`，也不启用远端复制/同步链路。
- 后续若要扩展云端，可在保持本地默认模式的前提下增加可选配置开关。

## 许可证

MIT

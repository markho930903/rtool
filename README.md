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

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端框架 | React 19 + TypeScript |
| 构建工具 | Vite 7 |
| 样式方案 | UnoCSS |
| 状态管理 | Zustand |
| 路由 | React Router 7 |
| 国际化 | i18next |
| 后端框架 | Tauri 2.x |
| 后端语言 | Rust |
| 数据库 | SQLite |

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
│   ├── hooks/             # React Hooks
│   ├── i18n/              # 国际化配置
│   └── styles/            # 样式文件
├── src-tauri/             # Tauri/Rust 后端
│   ├── src/
│   │   ├── commands/      # Tauri 命令
│   │   ├── core/          # 核心模块
│   │   └── infrastructure/# 基础设施
│   └── Cargo.toml
└── package.json
```

## 代码规范

- 前端代码格式化：`pnpm format`
- 代码检查：`pnpm lint`
- 国际化检查：`pnpm i18n:check`

## 许可证

MIT

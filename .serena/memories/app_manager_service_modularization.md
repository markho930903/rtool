# app_manager_service 模块瘦身约定

- 目录约束：`src-tauri/src/app/app_manager_service` 采用平铺文件结构，不引入二级子目录。
- 入口约束：保持目录模块风格（`mod.rs` + 同级子模块），不恢复 `app_manager_service.rs`。
- residue 拆分边界：
  - `residue_scan.rs`：残留根路径推导、候选收集、去重、分组与扫描结果构建（含 `should_replace_residue_candidate`）。
  - `residue_cleanup.rs`：删除模式实现、跨平台回收站/注册表删除、`execute_cleanup_plan`。
- 共享工具：平台通用 helper（如 `windows_powershell_escape`、`applescript_escape`、缓存访问）继续放在父模块或对应子模块，通过 `use super::*` 复用。
- 验证基线：每轮拆分后至少执行 `cargo check`、`cargo fmt --check`、`cargo test --no-run`。
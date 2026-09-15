# 首次启动 MCP 纳管引导实施计划

**Goal:** 首次启动自动发现本机工具的 MCP 配置，引导用户选择工具及服务并纳入管理。

**Architecture:** 复用适配器默认配置路径、发现解码和来源绑定。新工作区持久化引导状态，旧工作区缺省视为已完成。增加跨目标只读扫描及原子纳管命令，前端沿用工作区操作锁和现有弹窗样式。

**Tech Stack:** Tauri 2、Rust、React、TypeScript。

## 方案

- 采用集中扫描弹窗：直接查看所有有 MCP 的工具，支持按工具全选和逐项选择，明确确认后导入。
- 单工具向导需反复切换；自动导入则缺少用户选择。保留现有手动发现入口作为后续补充。
- 通过已知配置文件发现工具配置，不把文件存在解释为客户端正在运行或服务连接成功。
- 完成、跳过或关闭引导均保存完成状态；直接退出应用或扫描失败不自动完成。无配置时显示空状态、重新扫描和进入工作区。
- 单个文件读取失败、格式不支持和已管理分别展示；默认不勾选。一次提交重新读取所有选中来源，全部成功后原子保存服务和完成状态，原文件保持不变。

## 实施步骤

1. 修改 `src-tauri/src/model.rs`、`engine/types.rs`、`engine/mod.rs`：状态迁移、聚合发现、原子完成引导。
2. 修改 `src-tauri/src/commands.rs`、`src/api.ts`、`src/types.ts`、`src/hooks/useWorkspace.ts`：同步 IPC 类型及统一写入入口。
3. 新增 `src/features/targets/OnboardingDialog.tsx`，修改 `src/App.tsx`、`src/layout/WorkspaceDialogs.tsx`、`src/features/targets/targets.css`：自动打开、工具分组选择、扫描及错误重试、空状态、跳过。
4. 扩展 Rust 核心与命令测试：新建/重启/旧数据迁移、无 MCP、损坏配置、不支持项、重复名称、跨工具失败回滚、原文件不变。
5. 执行 `npm test`、`npm run check`、`npm run build`；隔离样例验证首次弹出、选择导入、跳过和重启。

## 验收

- 原 checkout 保持干净；改动仅保存在独立 worktree。
- 首次启动呈现扫描结果，只有可导入项可勾选，默认未选。
- 纳入成功后服务保留各自来源，不立即产生配置写入；失败不出现部分纳入。
- 完成或跳过后重启不再自动引导；旧工作区升级同样不重复提示。

## 验证结果

- `npm test -- --offline`：36 项通过，包含新增 7 项引导测试；覆盖跳过/完成后重新打开工作区、旧版本迁移、来源只读和失败回滚。
- `npm run check`：TypeScript 与全目标 Clippy 通过，无警告。
- `npm run build`、`npm run tauri build -- --debug --bundles app`：通过。
- 桌面隔离样例：启动自动显示 5 个有 MCP 的工具、9 个条目；不支持项禁用，另一个损坏文件独立显示解析错误；初始没有勾选。
- 桌面导入结果：2 个 Codex 服务进入服务库，完成标记持久化，待应用数量为 0；6 个源配置文件的 SHA-256 与启动前相同。
- 验收使用 worktree 内 `.local-dev/fixture-home` 和 `.local-dev/fixture-data`，未访问或写入真实用户的工具配置。

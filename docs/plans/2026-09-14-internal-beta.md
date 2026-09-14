# MCP Deck Internal Beta Implementation Plan

**Goal:** 在已确认的 A 方向上交付可安装、可持久化、可安全修改真实 MCP 配置的 Tauri 内部试用版。

**Architecture:** React 负责界面，Rust 核心负责模型、适配器、发现、校验、补丁与事务。独立适配器注册表支持 JSON/JSONC 与 TOML；公共模型之外保留来源专属字段。用户主动应用前只读配置。

**Tech Stack:** Tauri 2、React、TypeScript、Rust、jsonc-parser CST、toml_edit。内部版使用版本化 JSON 元数据与恢复日志，用户目录权限 0700、文件 0600；凭据暂保留客户端原有机制，默认导出脱敏。

用户已经确认实现与交付目标；在本任务内顺序实施，不再要求批准设计或创建另一个任务。

1. 创建项目及构建入口：package.json、src-tauri/Cargo.toml、tauri.conf.json。验证 npm install 与 cargo fetch。
2. 实现 src-tauri/src/adapters.rs、model.rs：12 个适配器注册表、工具能力和路径、配置转换与局部修改。添加格式往返、未知字段与注释保留、SSE 识别、能力拒绝测试。
3. 实现 src-tauri/src/engine.rs、storage.rs：发现、导入、服务编辑、分配、冲突预览、写入前指纹校验、备份恢复、崩溃后恢复、进程互斥。以临时目录测试，不能对用户现有配置执行测试写入。
4. 实现 src/ 下 A 方向界面：空状态引导、服务详情、编辑、工具设置、发现导入、导出、应用与历史恢复。通过 Tauri invoke 连接真实后端；普通浏览器只展示不可写的预览说明。
5. 运行 Rust 单元/集成测试、TypeScript 构建、Clippy；启动隔离测试目录下的 Tauri 应用完成真实 UI 操作，验证读写与重启持久化。
6. 生成 macOS .app 内部试用包，记录支持矩阵、实际验证范围和启动方式。未签名公证、未验证全部第三方客户端的运行加载不宣称已完成。

新增适配器约定：每项声明 ID、显示名、默认候选路径、root key、dialect、支持的 transports、cwd 支持与官方文档；通用补丁引擎不依赖具体工具名称，特殊转换留在 dialect 层。路径可在应用内自定义，扩展宿主/多个配置位置可添加独立目标。

范围边界：首版配置层支持 stdio / HTTP / 可识别的 SSE；不托管 OAuth，不安装依赖、不自动调用业务工具，不假装配置写入等于客户端已经连接。

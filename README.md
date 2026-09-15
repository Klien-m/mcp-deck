# MCP Deck · 0.1.0 内部试用版

基于 Tauri 2 + React + Rust 的本地 MCP 配置管理器。沿用已确认的 A 方向：浅色三栏、工具列表、中央服务库和配置详情。参考 [SkillDeck](https://github.com/crossoverJie/SkillDeck) 的集中管理体验，MCP 配置引擎独立实现。

支持 12 种工具适配器：Codex、Claude Code、Cursor、Gemini CLI、OpenCode、GitHub Copilot CLI、VS Code、Windsurf、Kiro、Cline、Roo Code、Claude Desktop。配置格式、传输能力和路径由适配器声明；同一种工具可添加多个配置位置。

## 立即试用

从 [GitHub Releases](https://github.com/Klien-m/mcp-deck/releases) 下载对应平台的安装包：Windows x64 提供 `.exe` / `.msi`，macOS Apple Silicon / Intel 各提供 `.dmg`，Linux x64 提供 `.deb` / `.rpm` / `.AppImage`。macOS 最低系统版本配置为 12，打开 DMG 后将 `MCP Deck.app` 拖入应用程序目录。实际验收系统见 [验证记录](docs/VALIDATION.md)。

1. 点击「发现本机配置」，选择目标工具和已有服务，纳入服务库。此步骤只读取原配置。
2. 编辑服务，打开要分配到的工具开关。修改先保存在服务库。
3. 点击「同步预览」，检查变更和冲突。可显式展开包含完整参数的差异。
4. 点击「应用」，程序先备份，再写入并回读检查。
5. 到目标工具中刷新 MCP / 重启并完成其授权。应用只报告配置文件状态。
6. 需要撤回时，进入「同步记录」恢复；如果文件已经被其他程序修改，会阻止覆盖。

默认路径不符合当前安装方式时，到「工具与路径」编辑。Cline CLI / 扩展、VS Code Profile、项目级配置可以添加独立目标。内置扩展路径只是候选位置，以宿主工具显示的实际文件为准。

## 已实现

- 本机配置发现、明确选择后纳入管理；保留不同来源的同名服务。
- 服务增改、搜索、移除与即时撤销；多目标分配。
- JSON / JSONC 与 Codex TOML 转换；局部修改 MCP 节点，保留其他配置内容。
- 目标路径选择、同适配器多实例、字段和命令位置检查。
- 脱敏导出模板 / 显式完整导出、文本导入。
- 差异预览、同名冲突和外部编辑检测、备份、恢复、异常中断恢复。
- 原生文件选择器、本地持久化、同一工作区进程互斥。

## 当前边界

- 这是配置管理内部版。12 种格式已通过自动化契约测试，**尚未逐一在 12 个第三方客户端完成加载认证验证**。
- 配置检查不会启动 MCP 进程、调用业务工具或进行网络握手；不自动安装 npx / uvx 服务，也不托管 OAuth。
- 保存分配并不代表目标工具已经启用、加载或联网。原生 `disabled`、`enabled`、OAuth、工具许可等字段按来源保留。
- 凭据、中央库与备份保存在本机文件中，使用目录 0700 / 文件 0600 保护，**没有加密或接入系统钥匙串**。原有目标文件权限保持原样。默认脱敏属于常见字段处理，分享导出文件前仍需检查来源自定义字段。
- 不解析多个配置作用域的合并优先级。工具或项目策略可能覆盖用户级配置。
- 未支持的配置结构会保留原文件、拒绝纳入管理，不进行有损转换。符号链接路径需选择其真实文件位置。
- macOS 包为本地 ad hoc 签名，未使用 Developer ID 签名或 Apple 公证；跨机器分发可能受到 Gatekeeper 限制。Windows、Linux 和 Intel Mac 构建未验收。

## 开发

已在 Node.js 24.10、npm 11.6、Rust 1.91 和 macOS Command Line Tools 环境构建。仓库包含 npm 与 Cargo 锁文件；安装依赖需要网络。

```bash
cd mcp-deck
npm ci
node scripts/seed-fixtures.mjs
npm run tauri dev
```

**Debug 构建默认使用隔离目录** `.local-dev/fixture-home` 与 `.local-dev/fixture-data`，不会写入真实用户的 Agent 配置。种子脚本只添加不存在的演示文件，不覆盖已经编辑的样例，也不执行其中的命令。

```bash
npm run check
npm test
npm run tauri build -- --bundles app
```

最后一条生成 Release 应用；本轮交付另外执行了本地完整包签名与验证：

```bash
codesign --force --deep --sign - "src-tauri/target/release/bundle/macos/MCP Deck.app"
codesign --verify --deep --strict "src-tauri/target/release/bundle/macos/MCP Deck.app"
```

应用图标使用已选定的哑光卡片与插头设计，原图为 `src-tauri/icons/app-icon-source.png`。画布内保留约 8.5% 的单侧透明边距，适配 Dock 的视觉尺寸；应用内标识与打包图标使用同一份图案。修改原图后重新生成资源：

```bash
npm run tauri icon -- src-tauri/icons/app-icon-source.png --output .local-dev/app-icons
for icon in 32x32.png 64x64.png 128x128.png 128x128@2x.png icon.png icon.icns icon.ico; do
  cp ".local-dev/app-icons/$icon" "src-tauri/icons/$icon"
done
cp .local-dev/app-icons/128x128@2x.png public/app-icon.png
```

应用位置：

`src-tauri/target/release/bundle/macos/MCP Deck.app`

**Release 默认读取真实用户目录**，应用数据位于 `~/Library/Application Support/com.mcpdeck.desktop/`。首次启动只创建 MCP Deck 自己的数据；目标配置需用户在界面应用后才修改。

可通过 `MCP_DECK_HOME` 指定独立工具目录，通过 `MCP_DECK_DATA_DIR` 指定独立服务库；只指定前者时，服务库默认放在该目录下的 `.mcp-deck-data`。这两个变量必须是绝对、无符号链接路径。

本次构建缓存单独放在 `.local-dev`。若沿用该缓存运行 Cargo，可在命令前使用：

```bash
CARGO_HOME="$PWD/.local-dev/cargo-home" npm test
```

## 标签发布

推送任意 Git 标签都会触发 [Release 工作流](.github/workflows/release.yml)。工作流从标签对应的提交构建四个架构、七个安装包，全部成功后发布 GitHub Release，并自动附上自上一个祖先标签以来的提交说明与完整变更链接；首次发布列出完整提交历史。

版本由仓库配置决定。发布前同步更新 `package.json` / `package-lock.json`、`src-tauri/Cargo.toml` / `Cargo.lock` 和 `src-tauri/tauri.conf.json` 的应用版本并提交到 master。首个标签 `v0.1` 对应应用版本 `0.1.0`。例如发布下一版：

```bash
git switch master
git pull --ff-only origin master
git tag -a v0.1.1 -m "发布 v0.1.1"
git push origin v0.1.1
```

只有推送到 GitHub 的标签才会触发构建，且该标签必须包含工作流文件。失败时可在 Actions 中重新运行失败任务，或使用 `gh workflow run release.yml --ref v0.1.1` 在同一标签重新执行。构建过程只需要自动提供的 `GITHUB_TOKEN`；Release 发布任务声明 `contents: write` 权限。macOS 使用 ad hoc 签名，Windows 安装包不签名。

## 真实配置验证

运行 `npm run validate:local`，使用当前用户的工具配置，验证发现、纳入管理、文本导入和同步预览。也可运行 `npm run validate:local -- --home /absolute/fixture-home` 验证指定样例目录。

验证工作区临时建立在项目的 `.local-dev/` 下，结束后删除。检测前后比较原配置内容指纹；仅在临时服务库里编辑一条配置以检查更新差异，不执行应用、恢复、服务启动或网络请求。报告只输出工具名称、数量和检查结果，不包含配置内容、服务名称、地址或凭据。

`invariantsPassed` 表示导入、预览和源文件保护的检查通过，不表示所有配置均受支持或服务可连接。请同时查看 `read-or-parse-error`、`unsupported`、`textImport` 和 `editPreview`。`sourceUnchanged: null` 表示文件无法读取，未验证内容一致性；预期的字段限制会记录为 `blocked-unsupported-cwd`。

本次实机结果见 [真实配置验证记录](docs/REAL_DATA_VALIDATION.md)。

## 结构

| 位置 | 职责 |
| --- | --- |
| `src/App.tsx`、`Editor.tsx` | A 方向界面、编辑、发现、差异、恢复与导出 |
| `src/api.ts`、`types.ts` | Tauri IPC 和前端契约 |
| `src-tauri/src/model.rs` | 服务、目标、绑定、校验和脱敏 |
| `src-tauri/src/adapters.rs` | 注册表、工具方言、路径、JSONC / TOML 补丁 |
| `src-tauri/src/engine.rs` | 用例、基线与冲突、写入事务、恢复 |
| `src-tauri/src/storage.rs` | 文件约束、进程锁、原子替换与回读 |
| `src-tauri/src/lib.rs` | 桌面命令分发和运行目录 |
| `src-tauri/tests/core.rs` | 配置契约、保留语义、并发和故障恢复测试 |

进一步见 [适配器说明](docs/ADAPTERS.md)、[验证记录](docs/VALIDATION.md)。

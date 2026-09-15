# MCP Deck · 0.1.0 内部试用版

基于 Tauri 2 + React + Rust 的本地 MCP 配置管理器。沿用已确认的 A 方向：浅色三栏、工具列表、中央服务库和配置详情。参考 [SkillDeck](https://github.com/crossoverJie/SkillDeck) 的集中管理体验，MCP 配置引擎独立实现。

支持 12 种工具适配器：Codex、Claude Code、Cursor、Gemini CLI、OpenCode、GitHub Copilot CLI、VS Code、Windsurf、Kiro、Cline、Roo Code、Claude Desktop。配置格式、传输能力和路径由适配器声明；同一种工具可添加多个配置位置。

## 立即试用

从 [GitHub Releases](https://github.com/Klien-m/mcp-deck/releases) 下载对应平台的安装包：Windows x64 提供 `.exe` / `.msi`，macOS Apple Silicon / Intel 各提供 `.dmg`，Linux x64 提供 `.deb` / `.rpm` / `.AppImage`。macOS 最低系统版本配置为 13.3（Ventura），打开 DMG 后将 `MCP Deck.app` 拖入应用程序目录。实际验收系统见 [验证记录](docs/VALIDATION.md)。

1. 首次启动自动扫描本机工具的 MCP 配置，按工具或逐项勾选后纳入服务库；可跳过，之后通过「发现本机配置」继续。此步骤只读取原配置，完成或跳过后不再自动弹出。
2. 编辑服务，打开要分配到的工具开关。修改先保存在服务库。
3. 点击「同步预览」，检查变更和冲突。可显式展开包含完整参数的差异。
4. 点击「应用」，程序先备份，再写入并回读检查。
5. 到目标工具中刷新 MCP / 重启并完成其授权。应用只报告配置文件状态。
6. 需要撤回时，进入「同步记录」恢复；如果文件已经被其他程序修改，会阻止覆盖。

默认路径不符合当前安装方式时，到「工具与路径」编辑。Cline CLI / 扩展、VS Code Profile、项目级配置可以添加独立目标。内置扩展路径只是候选位置，以宿主工具显示的实际文件为准。

## 已实现

- 首次启动按工具聚合发现 MCP，支持选择后批量纳管或跳过；后续可手动发现，保留不同来源的同名服务。
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
npm run test:ui
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

## macOS DMG 安装界面

DMG 使用极简浅灰背景、中文拖拽指引和原生 Finder 图标。背景素材为 `src-tauri/dmg/background.png`（1320 × 840 像素 / 660 × 420 点），窗口和图标位置统一在 `src-tauri/tauri.conf.json` 的 `bundle.macOS.dmg` 中配置。图标中心为 `(180, 230)` 和 `(480, 230)`；窗口高度额外预留 28 点标题栏。

在 macOS 上安装打包依赖并构建（Python 3.10+）：

```bash
python3 -m venv .local-dev/dmg-venv
.local-dev/dmg-venv/bin/python -m pip install -r scripts/dmg-requirements.txt
npm run tauri build -- --bundles app
.local-dev/dmg-venv/bin/python scripts/build-dmg.py
```

交叉编译时，为 Tauri 构建和 `build-dmg.py` 同时传入 `--target aarch64-apple-darwin` 或 `--target x86_64-apple-darwin`。安装盘输出到对应的 `target[/<target>]/release/bundle/dmg/`。

发布工作流采用同样的流程：Tauri 构建并签名 app，固定版本的 `dmgbuild` 直接写入安装盘的 Finder 视图信息。这样无需运行 Finder / AppleScript，也不会受构建机器的 Finder 默认排序或异步保存影响。不要使用 Tauri 默认的 `--bundles dmg` 代替此流程；它可能丢失定制背景和布局。

背景 PNG 随仓库保存，常规构建无需重新生成。修改背景时，在 macOS 上执行：

```bash
clang -fobjc-arc -framework AppKit scripts/generate-dmg-background.m -o /tmp/mcp-deck-dmg-background
/tmp/mcp-deck-dmg-background src-tauri/dmg/background.png
```

安装包生成后应挂载检查窗口、背景、图标位置和 `/Applications` 链接。

## 标签发布

推送任意 Git 标签都会触发 [Release 工作流](.github/workflows/release.yml)。工作流从标签对应的提交构建四个架构、七个安装包，全部成功后发布 GitHub Release，并自动附上自上一个祖先标签以来的提交说明与完整变更链接；首次发布列出完整提交历史。

构建时自动从标签提取版本，并同步 `package.json` / `package-lock.json`、`src-tauri/Cargo.toml` / `Cargo.lock` 和 `src-tauri/tauri.conf.json` 中的应用版本。支持 `v0.2` → `0.2.0`、`v0.2.1` → `0.2.1`，`v` 前缀可省略；其他格式（包括预发布后缀）会报错停止。版本修改只发生在 CI 工作目录，无需手动修改或提交这些配置；安装包内部版本和文件名使用同一版本，定制 DMG 也遵循该规则。例如发布下一版：

```bash
git switch master
git pull --ff-only origin master
git tag -a v0.1.1 -m "发布 v0.1.1"
git push origin v0.1.1
```

只有推送到 GitHub 的标签才会触发构建，且该标签必须包含工作流文件。失败时可在 Actions 中重新运行失败任务，或使用 `gh workflow run release.yml --ref v0.1.1` 在同一标签重新执行。构建过程只需要自动提供的 `GITHUB_TOKEN`；Release 发布任务声明 `contents: write` 权限。macOS 使用 ad hoc 签名，Windows 安装包不签名。

若要用最新工作流重建旧版本，运行 `gh workflow run release.yml --ref master -f tag=v0.3`。工作流使用 master 中的版本同步和更新说明脚本，检出指定标签的原始代码，再按标签版本打包；旧版没有定制 DMG 脚本时使用 Tauri 原生 DMG 打包。标签指向保持不变。新附件验证成功后，可清理该 Release 中旧版本号的附件。

可运行 `node --test scripts/set-release-version.test.mjs` 验证版本同步脚本。

## 真实配置验证

运行 `npm run validate:local`，使用当前用户的工具配置，验证发现、纳入管理、文本导入和同步预览。也可运行 `npm run validate:local -- --home /absolute/fixture-home` 验证指定样例目录。

验证工作区临时建立在项目的 `.local-dev/` 下，结束后删除。检测前后比较原配置内容指纹；仅在临时服务库里编辑一条配置以检查更新差异，不执行应用、恢复、服务启动或网络请求。报告只输出工具名称、数量和检查结果，不包含配置内容、服务名称、地址或凭据。

`invariantsPassed` 表示导入、预览和源文件保护的检查通过，不表示所有配置均受支持或服务可连接。请同时查看 `read-or-parse-error`、`unsupported`、`textImport` 和 `editPreview`。`sourceUnchanged: null` 表示文件无法读取，未验证内容一致性；预期的字段限制会记录为 `blocked-unsupported-cwd`。

本次实机结果见 [真实配置验证记录](docs/REAL_DATA_VALIDATION.md)。

## 结构

| 位置 | 职责 |
| --- | --- |
| `src/App.tsx`、`src/layout/` | 工作区装配、导航、弹窗路由、启动与空状态 |
| `src/features/services/` | 服务列表、筛选选择、详情和服务编辑表单 |
| `src/features/targets/` | 目标路径管理、目标编辑和本机配置发现 |
| `src/features/transfer/`、`src/features/sync/` | 导入导出、同步预览、冲突处理与恢复记录 |
| `src/hooks/useWorkspace.ts` | 快照和预览刷新，统一协调所有写操作、忙碌状态与错误 |
| `src/styles.css`、`src/styles/`、功能目录内 CSS | 样式入口、公共规则及按功能归属的样式；响应式覆盖最后加载 |
| `src/api.ts`、`types.ts` | 按命令关联参数及返回值的 Tauri IPC 契约 |
| `src-tauri/src/model.rs` | 服务、目标、绑定、校验和脱敏 |
| `src-tauri/src/adapters/` | 注册表与路径、工具字段转换、JSONC / TOML 文档编辑 |
| `src-tauri/src/engine/mod.rs` | 引擎生命周期、用例入口和工作区原子提交 |
| `src-tauri/src/engine/{services,discovery,targets,transfer,diagnostics}.rs` | 服务变更、发现导入、目标路径、导出与配置诊断 |
| `src-tauri/src/engine/{sync,planner,transaction}.rs` | 预览计划生命周期、纯计划计算、备份写入与故障恢复 |
| `src-tauri/src/storage.rs` | 文件约束、进程锁、原子替换与回读 |
| `src-tauri/src/commands.rs`、`lib.rs` | 可独立验证的命令契约与分发、桌面状态锁及运行目录 |
| `src-tauri/tests/core.rs` | 配置契约、保留语义、并发和故障恢复测试 |
| `src-tauri/tests/commands.rs` | IPC 参数与返回值、预览应用及导出保护测试 |

进一步见 [适配器说明](docs/ADAPTERS.md)、[验证记录](docs/VALIDATION.md)。

功能组件仅接收所需数据和具体操作回调；只有工作区协调层调用写入命令。Rust 用例在工作区副本上运行，通过校验并持久化后才发布新状态；适配器转换不负责文件 IO，事务模块统一负责目标文件与恢复日志的写入协议。

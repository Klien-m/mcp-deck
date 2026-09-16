<div align="center">
  <img src="public/app-icon.png" width="112" alt="MCP Deck 图标">
  <h1>MCP Deck</h1>
  <p>跨平台的 AI Agent MCP 配置管理桌面应用</p>
  <p>
    <a href="https://github.com/Klien-m/mcp-deck/actions/workflows/release.yml"><img src="https://github.com/Klien-m/mcp-deck/actions/workflows/release.yml/badge.svg" alt="Release"></a>
    <a href="https://github.com/Klien-m/mcp-deck/releases/latest"><img src="https://img.shields.io/github/v/release/Klien-m/mcp-deck?display_name=tag&amp;sort=semver&amp;label=release" alt="Latest release"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-555555" alt="Windows, macOS and Linux">
    <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&amp;logoColor=white" alt="Tauri 2">
    <img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&amp;logoColor=black" alt="React 19">
    <img src="https://img.shields.io/badge/MCP%20adapters-12-6F42C1" alt="12 MCP adapters">
  </p>
  <p>中文 | <a href="README_EN.md">English</a></p>
</div>

---

MCP Deck 使用一个本地服务库统一管理多个 AI 工具的 MCP 配置。它可以发现已有配置，将同一服务分配到不同工具，并在写入前展示差异、检查冲突和创建备份。无需反复编辑 JSON、JSONC 或 TOML 文件。

支持 Codex、Claude Code、Cursor、Gemini CLI、OpenCode、GitHub Copilot CLI、VS Code、Windsurf、Kiro、Cline、Roo Code 和 Claude Desktop。

## 功能特性

- **统一服务库** — 集中增改、搜索和移除 MCP 服务，再按需分配给多个工具。
- **配置发现** — 首次启动扫描本机已有 MCP 配置，可按工具或逐项选择纳入管理；之后也可随时重新发现。
- **多工具、多目标** — 内置 12 种工具适配器，同一种工具可以添加多个配置位置。
- **格式转换** — 在 JSON、JSONC 与 Codex TOML 之间转换公共字段，并保留未修改的配置内容与工具专属字段。
- **安全同步** — 应用前展示完整差异，检测同名冲突和外部编辑；写入时自动备份、回读检查，并支持恢复。
- **导入与导出** — 支持文本导入、默认脱敏导出，以及需要明确确认的完整导出。
- **本地优先** — 服务库、凭据和备份只保存在本机；MCP Deck 不启动 MCP 服务，也不托管 OAuth。

## 安装

从 [GitHub Releases](https://github.com/Klien-m/mcp-deck/releases) 下载最新安装包：

| 平台                | 安装包                        |
| ------------------- | ----------------------------- |
| Windows x64         | `.exe` / `.msi`               |
| macOS Apple Silicon | `.dmg`                        |
| macOS Intel         | `.dmg`                        |
| Linux x64           | `.deb` / `.rpm` / `.AppImage` |

macOS 最低支持 13.3（Ventura）。打开 DMG 后，将 `MCP Deck.app` 拖入“应用程序”目录。当前 macOS 安装包使用 ad hoc 签名，未经过 Apple 公证，跨机器安装时可能需要在“隐私与安全性”中确认打开。

> 目前仅在 macOS Apple Silicon 完成了完整构建与验收；其他平台由发布工作流生成安装包，仍需要更多实机验证。

## 快速开始

1. 首次启动时选择要纳入服务库的本机 MCP 配置，也可以跳过并稍后使用“发现本机配置”。
2. 新建或编辑服务，打开要分配到的工具开关。
3. 点击“同步预览”，检查每个目标文件的变更与冲突。
4. 点击“应用”。MCP Deck 会先备份，再写入并回读检查。
5. 在目标工具中刷新 MCP 或重启工具，并完成该工具要求的授权。
6. 如果需要撤回，进入“同步记录”恢复；目标文件已被其他程序修改时，MCP Deck 会阻止直接覆盖。

默认路径与实际安装方式不一致时，可在“工具与路径”中编辑。Cline CLI / 扩展、VS Code Profile 和项目级配置可以分别添加为独立目标。

## 支持的工具

下表中的路径是默认候选位置。文件不存在只表示尚未发现配置，不代表对应工具已经或尚未安装。

| 工具               | 默认配置位置                                                      | 格式  | 传输方式         |
| ------------------ | ----------------------------------------------------------------- | ----- | ---------------- |
| Codex              | `~/.codex/config.toml`                                            | TOML  | stdio、HTTP      |
| Claude Code        | `~/.claude.json`                                                  | JSON  | stdio、HTTP、SSE |
| Cursor             | `~/.cursor/mcp.json`                                              | JSON  | stdio、HTTP      |
| Gemini CLI         | `~/.gemini/settings.json`                                         | JSON  | stdio、HTTP、SSE |
| OpenCode           | `~/.config/opencode/opencode.jsonc`                               | JSONC | stdio、HTTP      |
| GitHub Copilot CLI | `~/.copilot/mcp-config.json`                                      | JSON  | stdio、HTTP      |
| VS Code            | `~/Library/Application Support/Code/User/mcp.json`                | JSONC | stdio、HTTP、SSE |
| Windsurf           | `~/.codeium/windsurf/mcp_config.json`                             | JSON  | stdio、HTTP      |
| Kiro               | `~/.kiro/settings/mcp.json`                                       | JSON  | stdio、HTTP      |
| Cline              | `~/.cline/mcp.json` 或扩展配置                                    | JSON  | stdio、HTTP、SSE |
| Roo Code           | VS Code `globalStorage` 中的扩展配置                              | JSON  | stdio、HTTP、SSE |
| Claude Desktop     | `~/Library/Application Support/Claude/claude_desktop_config.json` | JSON  | stdio            |

完整的路径、字段差异与保留规则见[适配器说明](docs/ADAPTERS.md)。

## 工作原理

MCP Deck 基于 Tauri 2、React 和 Rust。前端只负责交互与状态展示，所有配置解析、差异计算、备份和文件写入均由 Rust 引擎处理。

```text
React UI
   ↓ Tauri IPC
工作区协调层
   ↓
服务 / 发现 / 目标 / 导入导出
   ↓
同步计划 → 冲突检查 → 事务写入 → 备份与恢复
   ↓
工具适配器（JSON / JSONC / TOML）
```

写入时只修改目标 MCP 节点：JSONC 会保留外围注释与未修改条目的文本，TOML 会保留其他表和注释。工作区副本通过校验并持久化后，才会发布为新的应用状态。

## 当前边界

- 12 种配置格式已通过自动化契约测试，但尚未逐一完成第三方客户端的加载与认证实测。
- 配置检查不会启动 MCP 进程、调用业务工具或执行网络握手，也不会自动安装 `npx` / `uvx` 服务。
- 保存分配只表示目标配置文件已更新，不代表目标工具已经启用、加载或联网。
- 中央服务库与备份使用目录 `0700`、文件 `0600` 保护，但尚未加密，也未接入系统钥匙串。
- 不解析多个配置作用域的最终合并优先级；项目配置或组织策略仍可能覆盖用户级配置。
- 不支持的配置结构会保持原文件不变并拒绝接管，避免有损转换。

## 从源码运行

已验证环境：Node.js 24.10、npm 11.6、Rust 1.91 和 macOS Command Line Tools。仓库包含 npm 与 Cargo 锁文件，安装依赖需要网络。

```bash
git clone https://github.com/Klien-m/mcp-deck.git
cd mcp-deck
npm ci
node scripts/seed-fixtures.mjs
npm run tauri dev
```

Debug 构建默认使用 `.local-dev/fixture-home` 和 `.local-dev/fixture-data`，不会写入真实用户的 Agent 配置。运行检查和构建：

```bash
npm run check
npm run test:ui
npm test
npm run tauri build -- --bundles app
```

Release 构建默认读取真实用户目录，应用数据位于 `~/Library/Application Support/com.mcpdeck.desktop/`。也可以使用绝对路径环境变量 `MCP_DECK_HOME` 和 `MCP_DECK_DATA_DIR` 指向隔离目录。

## 发布

推送格式为 `v0.2` 或 `v0.2.1` 的 Git 标签会触发发布工作流。工作流从标签对应的提交构建四个架构、七个安装包，并在全部成功后发布 GitHub Release；应用版本会在 CI 工作目录中自动同步，无需手动修改版本文件。

```bash
git tag -a v0.4 -m "发布 v0.4"
git push origin v0.4
```

## 文档

- [适配器说明与支持矩阵](docs/ADAPTERS.md)
- [自动化验证记录](docs/VALIDATION.md)
- [真实配置验证记录](docs/REAL_DATA_VALIDATION.md)

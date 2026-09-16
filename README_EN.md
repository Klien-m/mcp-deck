<div align="center">
  <img src="public/app-icon.png" width="112" alt="MCP Deck icon">
  <h1>MCP Deck</h1>
  <p>A cross-platform desktop app for managing MCP configurations across AI agents</p>
  <p>
    <a href="https://github.com/Klien-m/mcp-deck/actions/workflows/release.yml"><img src="https://github.com/Klien-m/mcp-deck/actions/workflows/release.yml/badge.svg" alt="Release"></a>
    <a href="https://github.com/Klien-m/mcp-deck/releases/latest"><img src="https://img.shields.io/github/v/release/Klien-m/mcp-deck?display_name=tag&amp;sort=semver&amp;label=release" alt="Latest release"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-555555" alt="Windows, macOS and Linux">
    <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&amp;logoColor=white" alt="Tauri 2">
    <img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&amp;logoColor=black" alt="React 19">
    <img src="https://img.shields.io/badge/MCP%20adapters-12-6F42C1" alt="12 MCP adapters">
  </p>
  <p><a href="README.md">中文</a> | English</p>
</div>

---

MCP Deck keeps MCP configurations for multiple AI tools in one local service library. It discovers existing configurations, assigns a service to multiple tools, previews every change, detects conflicts, and creates backups before writing. You no longer need to edit JSON, JSONC, or TOML files by hand.

It supports Codex, Claude Code, Cursor, Gemini CLI, OpenCode, GitHub Copilot CLI, VS Code, Windsurf, Kiro, Cline, Roo Code, and Claude Desktop.

## Features

- **Unified service library** — Create, edit, search, and remove MCP services in one place, then assign them to multiple tools.
- **Configuration discovery** — Scan existing MCP configurations on first launch, import selected tools or services, and run discovery again at any time.
- **Multiple tools and targets** — Use 12 built-in tool adapters and add more than one configuration location for the same tool.
- **Format conversion** — Convert shared fields between JSON, JSONC, and Codex TOML while preserving untouched content and tool-specific fields.
- **Safe synchronization** — Preview full diffs, detect name conflicts and external edits, create backups, verify writes, and restore earlier configurations.
- **Sync status and diagnostics** — Distinguish pending additions, updates, removals, read failures, and aligned files. Pending removals can be undone. Static checks explain command locations, permissions, working directories, variables, and entry scripts without starting servers.
- **Import and export** — Import configuration text, export redacted templates by default, or explicitly export complete values.
- **Local first** — The service library, credentials, and backups stay on your computer. MCP Deck neither starts MCP servers nor hosts OAuth flows.

## Installation

Download the latest installer from [GitHub Releases](https://github.com/Klien-m/mcp-deck/releases):

| Platform            | Package                       |
| ------------------- | ----------------------------- |
| Windows x64         | `.exe` / `.msi`               |
| macOS Apple Silicon | `.dmg`                        |
| macOS Intel         | `.dmg`                        |
| Linux x64           | `.deb` / `.rpm` / `.AppImage` |

macOS 13.3 Ventura or later is required. Open the DMG and drag `MCP Deck.app` into Applications. The current macOS builds use ad hoc signing and are not notarized by Apple, so installations on another Mac may require approval in Privacy & Security.

> Only the macOS Apple Silicon build has completed full local build and acceptance testing. The release workflow produces the other packages, but they still need broader testing on physical machines.

## Quick Start

1. On first launch, choose which local MCP configurations to add to the service library. You can also skip this step and use **Discover local configurations** later.
2. Create or edit a service, then enable the tools that should receive it.
3. Select **Sync preview** and review the changes and conflicts for every target file.
4. Select **Apply**. MCP Deck creates a backup before writing and verifies the result afterward.
5. Refresh MCP or restart the target tool, then complete any authorization required by that tool.
6. To undo a change, restore it from **Sync history**. MCP Deck blocks the restore if another program has modified the target file.

If a default path does not match your installation, edit it under **Tools & paths**. Cline CLI and extension configurations, VS Code profiles, and project-level configurations can be added as separate targets.

## Supported Tools

The paths below are default candidates. A missing file only means that no configuration was found at that path; it does not indicate whether the tool itself is installed.

| Tool               | Default configuration                                             | Format | Transports       |
| ------------------ | ----------------------------------------------------------------- | ------ | ---------------- |
| Codex              | `~/.codex/config.toml`                                            | TOML   | stdio, HTTP      |
| Claude Code        | `~/.claude.json`                                                  | JSON   | stdio, HTTP, SSE |
| Cursor             | `~/.cursor/mcp.json`                                              | JSON   | stdio, HTTP      |
| Gemini CLI         | `~/.gemini/settings.json`                                         | JSON   | stdio, HTTP, SSE |
| OpenCode           | `~/.config/opencode/opencode.jsonc`                               | JSONC  | stdio, HTTP      |
| GitHub Copilot CLI | `~/.copilot/mcp-config.json`                                      | JSON   | stdio, HTTP      |
| VS Code            | `~/Library/Application Support/Code/User/mcp.json`                | JSONC  | stdio, HTTP, SSE |
| Windsurf           | `~/.codeium/windsurf/mcp_config.json`                             | JSON   | stdio, HTTP      |
| Kiro               | `~/.kiro/settings/mcp.json`                                       | JSON   | stdio, HTTP      |
| Cline              | `~/.cline/mcp.json` or an extension configuration                 | JSON   | stdio, HTTP, SSE |
| Roo Code           | Extension configuration under VS Code `globalStorage`             | JSON   | stdio, HTTP, SSE |
| Claude Desktop     | `~/Library/Application Support/Claude/claude_desktop_config.json` | JSON   | stdio            |

See the [adapter guide](docs/ADAPTERS.md) for complete paths, field mappings, and preservation rules.

## Cross-platform Configuration Paths

| Tool / directory | macOS | Windows | Linux |
| --- | --- | --- | --- |
| VS Code user configuration | `~/Library/Application Support/Code/User` | `%APPDATA%/Code/User` | `$XDG_CONFIG_HOME/Code/User`, falling back to `~/.config/Code/User` |
| Cline / Roo extension configuration | `globalStorage` under the corresponding VS Code user directory | Same rule | Same rule |
| Shared Cline configuration | `~/.cline/data/settings/cline_mcp_settings.json`; existing legacy files remain discoverable | Same path | Same path |
| Claude Desktop | `~/Library/Application Support/Claude` | `%APPDATA%/Claude` | Discover existing files under `$XDG_CONFIG_HOME/Claude` (default `~/.config/Claude`); add a target manually when absent |
| Codex | `$CODEX_HOME/config.toml`, falling back to `~/.codex/config.toml` | Same rule | Same rule |
| OpenCode | `$XDG_CONFIG_HOME/opencode`, falling back to `~/.config/opencode` | Same rule, not APPDATA | Same rule |

Windows falls back to `~/AppData/Roaming` when `APPDATA` is unset or invalid. Custom editor profiles, portable installs, and other hosts can still be added with an explicit path.

Incorrect legacy defaults are corrected only when the old file is absent and no service or history references it. Existing files and custom paths are preserved; Windows/Linux targets still referring to an old macOS default show a review notice. With `MCP_DECK_HOME` set, tool directory overrides cannot redirect discovery into real user directories.

## How It Works

MCP Deck is built with Tauri 2, React, and Rust. The frontend handles interaction and presentation, while the Rust engine performs configuration parsing, diff planning, backups, and file writes.

```text
React UI
   ↓ Tauri IPC
Workspace coordinator
   ↓
Services / Discovery / Targets / Transfer
   ↓
Sync plan → Conflict checks → Transactional writes → Backup and restore
   ↓
Tool adapters (JSON / JSONC / TOML)
```

Only the target MCP node is changed during a write. JSONC keeps surrounding comments and untouched entries, while TOML keeps unrelated tables and comments. A workspace copy is validated and persisted before it becomes the new application state.

## Current Limitations

- All 12 configuration formats have automated contract coverage, but not every third-party client has completed end-to-end loading and authentication testing.
- Validation does not start MCP processes, call business tools, perform network handshakes, or install `npx` / `uvx` services.
- Saving an assignment only updates the desired state in the library; target files change after Apply. Aligned files do not prove that a client loaded or connected to the service. If a write succeeds but the UI refresh fails, the successful result is preserved and only reads are retried.
- On Unix systems, the central library and backups use `0700` directories and `0600` files, but they are not encrypted and are not stored in the system keychain.
- MCP Deck does not resolve the final precedence of multiple configuration scopes. Project settings or organization policies may override user-level configuration.
- Unsupported structures are left untouched and rejected from management to avoid lossy conversion.

## Build from Source

The verified development environment uses Node.js 24.10, npm 11.6, Rust 1.91, and macOS Command Line Tools. npm and Cargo lockfiles are included; installing dependencies requires network access.

```bash
git clone https://github.com/Klien-m/mcp-deck.git
cd mcp-deck
npm ci
node scripts/seed-fixtures.mjs
npm run tauri dev
```

Debug builds use `.local-dev/fixture-home` and `.local-dev/fixture-data` by default and do not write to real AI tool configurations. Run checks and build the app with:

```bash
npm run check
npm run test:ui
npm test
node --test scripts/set-release-version.test.mjs
npm run tauri build -- --bundles app
```

Release builds use the real user directory by default and store application data in `com.mcpdeck.desktop` under the system user data directory (`~/Library/Application Support/` on macOS, `%APPDATA%/` on Windows, and `~/.local/share/` by default on Linux). Set the absolute-path environment variables `MCP_DECK_HOME` and `MCP_DECK_DATA_DIR` to use isolated directories instead.

## Automated Regression

The `Checks` workflow runs core tests, UI tests, release-version tests, Clippy, and frontend builds on Windows, Linux, and macOS for pull requests and pushes to `master` or `req/**`. Coverage includes static diagnostics, platform paths, migration guards, pending removals, display-only toggles, and recovery after committed writes followed by failed refreshes. Release builds also run core regression and the UI / version tests available in the tagged source before packaging; failures stop publishing.

## Releases

Pushing a Git tag such as `v0.2` or `v0.2.1` starts the release workflow. It builds seven installers for four architectures from the tagged commit and publishes a GitHub Release after every build succeeds. Application versions are synchronized inside the CI workspace, so version files do not need to be edited manually.

```bash
git tag -a v0.4 -m "Release v0.4"
git push origin v0.4
```

## Documentation

- [Adapter guide and support matrix](docs/ADAPTERS.md)
- [Automated validation record](docs/VALIDATION.md)
- [Real configuration validation record](docs/REAL_DATA_VALIDATION.md)

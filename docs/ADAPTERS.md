# 适配器设计与支持矩阵

适配器负责工具配置的差异，事务引擎不直接依赖工具名称。每个适配器声明 ID、名称、配置根键、格式、路径候选、传输能力、cwd 支持、官方文档和提示。

服务的公共字段是 `transport / command / args / cwd / env / url / headers`。额外的原生配置记录在 `native[adapterId]`，每个目标最近接管或应用的原始值记录在 `bindings[targetId].raw`。写入前使用原始基线保留该目标的原生字段，且检查实际磁盘内容是否已经改变。

## 支持矩阵

下面的 `~` 是目标用户目录。默认路径针对本次交付的 macOS；文件不存在表示尚未发现配置，不表示已经检测到该工具安装。

| 工具 | 默认配置位置 | 根键 / 格式 | 本版传输 | cwd |
| --- | --- | --- | --- | --- |
| Codex | `~/.codex/config.toml` | `mcp_servers` / TOML | stdio、HTTP | 是 |
| Claude Code | `~/.claude.json` | `mcpServers` / JSON | stdio、HTTP、SSE | 否 |
| Cursor | `~/.cursor/mcp.json` | `mcpServers` / JSON | stdio、HTTP | 否 |
| Gemini CLI | `~/.gemini/settings.json` | `mcpServers` / JSON | stdio、HTTP、SSE | 是 |
| OpenCode | `~/.config/opencode/opencode.jsonc` 或 `.json` | `mcp` / JSONC | stdio、HTTP | 否 |
| GitHub Copilot CLI | `~/.copilot/mcp-config.json` | `mcpServers` / JSON | stdio、HTTP | 否 |
| VS Code | `~/Library/Application Support/Code/User/mcp.json` | `servers` / JSONC | stdio、HTTP、SSE | 是 |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | `mcpServers` / JSON | stdio、HTTP | 否 |
| Kiro | `~/.kiro/settings/mcp.json` | `mcpServers` / JSON | stdio、HTTP | 否 |
| Cline | `~/.cline/mcp.json`；或 VS Code 的 `globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json` | `mcpServers` / JSON | stdio、HTTP、SSE | 否 |
| Roo Code | VS Code 的 `globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json` | `mcpServers` / JSON | stdio、HTTP、SSE | 是 |
| Claude Desktop | `~/Library/Application Support/Claude/claude_desktop_config.json` | `mcpServers` / JSON | stdio | 否 |

VS Code 的 `globalStorage` 位于 `~/Library/Application Support/Code/User/` 下。候选路径顺序优先选择已存在文件；多个安装位置可分别添加目标。`CODEX_HOME` 和 `XDG_CONFIG_HOME` 在真实用户默认发现时适用。已有目标路径持久化后不会随环境变量静默变化。

## 方言转换

- Codex：HTTP 请求头使用 `http_headers`，其他 TOML 配置不重写。
- Gemini：HTTP 使用 `httpUrl`，SSE 使用 `url`。
- OpenCode：本地 `type: "local"`，`command` 为命令和参数组成的数组，环境变量为 `environment`；远程使用 `type: "remote"`。原生 `enabled`、OAuth 设置保留。
- Copilot CLI：本地 `type: "local"`，远程 `type: "http"`；新配置默认 `tools: ["*"]`。
- VS Code：根键为 `servers`；本地写 `type: "stdio"`。根级 `inputs` 保留。
- Cursor / Kiro：远程写 `url`，由客户端处理其网络连接。
- Windsurf：远程写 `serverUrl`，可读取已有 `url`。
- Cline：HTTP 写 `type: "streamableHttp"`；缺少 type 的 url 按其旧格式视为 SSE。
- Roo Code：HTTP 写 `type: "streamable-http"`。远程配置缺少显式类型会拒绝接管，避免与 Cline 方言混淆。
- Claude Desktop：仅管理配置文件中的本地 stdio；远程 Connector 仍由 Desktop 自身管理。

JSONC 使用 CST 仅修改目标 MCP 条目，未修改条目的文本和外围注释保留；被修改条目内部会重新序列化。TOML 使用 `toml_edit` 更新对应服务表，保留其他表和注释。不承诺保留被编辑服务内部的逐行排版。

同一服务复制到不同适配器时，迁移公共字段；来源专属扩展字段只回写同方言，不猜测跨工具映射。无法转成公共模型的服务（例如对象形式的 env 值、非 HTTP(S) 的地址表达式）只读保留。切换传输方式前需检查原生扩展字段是否仍适用于新传输。

## 新增工具

1. 在 `registry()` 增加唯一 ID 和上述能力信息，路径选择作为 `Target` 保存。
2. 如与已有工具格式完全一致，可以复用 `Dialect`；存在字段或语义差异则新增方言，在 `decode()` / `encode()` 中实现转换。
3. `decode()` 必须拒绝当前公共模型不能完整表达的配置。`encode()` 不得把不支持的能力静默丢弃。
4. 保留非公共原生字段；未修改的已导入配置应该原样返回。
5. 给新方言添加基于官方示例的独立期望值测试，同时加入往返、局部更新、未知字段、冲突和不支持能力的测试。
6. 在真实客户端完成至少一次 stdio 与其支持的远程加载检查，再将“格式支持”提升为“该版本客户端实测”。

当前适配器是编译期 Rust 模块，不加载外部可执行插件。40 个配置目标、500 个服务（含保留的移除记录）、单配置文件 2 MB、单次恢复日志总文本 12 MB 为内部版限制。

## 官方依据

核验日期：2026-09-14。链接对应配置能力参考，客户端实际版本可能存在差异。

- [Codex MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
- [Claude Code MCP](https://code.claude.com/docs/en/mcp)
- [Cursor MCP](https://prod.cursor.com/docs/mcp)
- [Gemini CLI MCP](https://geminicli.com/docs/tools/mcp-server/)
- [OpenCode MCP](https://opencode.ai/docs/mcp-servers/)
- [GitHub Copilot CLI MCP](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers)
- [VS Code MCP 配置](https://code.visualstudio.com/docs/agents/reference/mcp-configuration)
- [Windsurf MCP](https://docs.devin.ai/desktop/cascade/mcp)
- [Kiro MCP](https://kiro.dev/docs/mcp/configuration/)
- [Cline MCP](https://docs.cline.bot/mcp/mcp-overview)
- [Roo Code MCP](https://roocodeinc.github.io/Roo-Code/features/mcp/using-mcp-in-roo/)
- [Claude Desktop 本地服务](https://modelcontextprotocol.io/docs/develop/connect-local-servers)

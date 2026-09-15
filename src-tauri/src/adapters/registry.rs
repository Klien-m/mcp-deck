//! 当前应用支持的客户端方言、能力和默认目标路径；不代表客户端运行时连接状态。

use crate::model::{Result, Target, Transport};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 配置字段约定；多个客户端可共享一个方言，避免按品牌重复实现同一转换。
pub enum Dialect {
    Codex,
    Standard,
    Url,
    Roo,
    Gemini,
    OpenCode,
    Copilot,
    VsCode,
    Windsurf,
    Cline,
    Desktop,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 前端可展示的适配能力及后端专用方言与路径候选；后两者不通过 IPC 暴露。
pub struct Adapter {
    pub id: &'static str,
    pub name: &'static str,
    pub root_key: &'static str,
    pub format: &'static str,
    pub transports: Vec<Transport>,
    pub supports_cwd: bool,
    pub docs: &'static str,
    pub note: &'static str,
    #[serde(skip)]
    pub dialect: Dialect,
    #[serde(skip)]
    pub paths: Vec<&'static str>,
}

/// 集中定义支持矩阵；新增客户端时需同时检查 decode、encode 和文档根节点。
pub fn registry() -> Vec<Adapter> {
    use Dialect::*;
    let records = [
        ("codex", "Codex", Codex, "mcp_servers", vec![".codex/config.toml"], true, "https://learn.chatgpt.com/docs/extend/mcp?surface=cli", "用户级配置；受项目配置及组织策略影响"),
        ("claude", "Claude Code", Standard, "mcpServers", vec![".claude.json"], false, "https://code.claude.com/docs/en/mcp", "用户级配置；项目私有配置保持原样"),
        ("cursor", "Cursor", Url, "mcpServers", vec![".cursor/mcp.json"], false, "https://prod.cursor.com/docs/mcp", "项目 .cursor/mcp.json 可能覆盖全局同名配置"),
        ("gemini", "Gemini CLI", Gemini, "mcpServers", vec![".gemini/settings.json"], true, "https://geminicli.com/docs/tools/mcp-server/", "HTTP 使用 httpUrl，SSE 使用 url"),
        ("opencode", "OpenCode", OpenCode, "mcp", vec![".config/opencode/opencode.jsonc", ".config/opencode/opencode.json"], false, "https://opencode.ai/docs/mcp-servers/", "command 为数组；保留 OAuth 与 enabled 字段"),
        ("copilot", "GitHub Copilot CLI", Copilot, "mcpServers", vec![".copilot/mcp-config.json"], false, "https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers", "用户级配置；登录由 Copilot 管理"),
        ("vscode", "VS Code", VsCode, "servers", vec!["Library/Application Support/Code/User/mcp.json"], true, "https://code.visualstudio.com/docs/agents/reference/mcp-configuration", "默认 macOS 用户配置；其他 Profile 请指定路径"),
        ("windsurf", "Windsurf", Windsurf, "mcpServers", vec![".codeium/windsurf/mcp_config.json"], false, "https://docs.devin.ai/desktop/cascade/mcp", "HTTP 使用 serverUrl；保留原生认证表达式"),
        ("kiro", "Kiro", Url, "mcpServers", vec![".kiro/settings/mcp.json"], false, "https://kiro.dev/docs/mcp/configuration/", "适用于兼容此配置的 IDE / CLI；自定义 Agent 的继承规则另行检查"),
        ("cline", "Cline", Cline, "mcpServers", vec![".cline/mcp.json", "Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json"], false, "https://docs.cline.bot/mcp/mcp-overview", "CLI 与扩展可添加为不同配置目标；扩展位置以其设置页为准"),
        ("roo", "Roo Code", Roo, "mcpServers", vec!["Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json"], true, "https://roocodeinc.github.io/Roo-Code/features/mcp/using-mcp-in-roo/", "默认 VS Code 扩展路径；其他宿主请指定其 mcp_settings.json"),
        ("claude-desktop", "Claude Desktop", Desktop, "mcpServers", vec!["Library/Application Support/Claude/claude_desktop_config.json"], false, "https://modelcontextprotocol.io/docs/develop/connect-local-servers", "此文件适配本地 stdio；远程连接由 Desktop 自身管理"),
    ];
    records
        .into_iter()
        .map(
            |(id, name, dialect, root_key, paths, supports_cwd, docs, note)| Adapter {
                id,
                name,
                dialect,
                root_key,
                paths,
                supports_cwd,
                docs,
                note,
                format: if dialect == Codex {
                    "TOML"
                } else {
                    "JSON / JSONC"
                },
                transports: if dialect == Desktop {
                    vec![Transport::Stdio]
                } else if matches!(dialect, Codex | OpenCode | Windsurf | Url | Copilot) {
                    vec![Transport::Stdio, Transport::Http]
                } else {
                    vec![Transport::Stdio, Transport::Http, Transport::Sse]
                },
            },
        )
        .collect()
}

/// 由稳定适配器 ID 查找能力，不根据用户可修改的目标名称推测格式。
pub fn get(id: &str) -> Result<Adapter> {
    registry()
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| "未知适配器".into())
}

/// 每种适配器优先选已有候选文件，否则使用首选路径；实际创建留到用户应用时。
pub fn default_targets(home: &Path) -> Vec<Target> {
    registry()
        .iter()
        .map(|adapter| {
            let paths: Vec<_> = adapter.paths.iter().map(|p| home.join(p)).collect();
            let mut path = paths
                .iter()
                .find(|p| p.is_file())
                .unwrap_or(&paths[0])
                .clone();
            // 指定隔离 home 时忽略真实进程的自定义工具路径，防止样例目标越出测试目录。
            if std::env::var_os("MCP_DECK_HOME").is_none()
                && dirs::home_dir().as_deref() == Some(home)
            {
                if adapter.id == "codex" {
                    if let Some(p) = std::env::var_os("CODEX_HOME") {
                        path = Path::new(&p).join("config.toml");
                    }
                }
                if adapter.id == "opencode" {
                    if let Some(p) = std::env::var_os("XDG_CONFIG_HOME") {
                        path = Path::new(&p).join("opencode/opencode.json");
                    }
                }
            }
            Target {
                id: adapter.id.into(),
                adapter_id: adapter.id.into(),
                name: adapter.name.into(),
                path: path.to_string_lossy().into(),
            }
        })
        .collect()
}

//! Agent-specific paths and dialects. The engine only calls these conversion/patch functions.
use crate::model::{Config, Result, Target, Transport};
use jsonc_parser::{
    cst::{CstInputValue, CstNode, CstRootNode},
    ParseOptions,
};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub fn get(id: &str) -> Result<Adapter> {
    registry()
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| "未知适配器".into())
}

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
            // Never use custom process environment when a test home is selected.
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

pub fn parse(adapter: &Adapter, text: &str) -> Result<BTreeMap<String, Value>> {
    let value: Value = if adapter.dialect == Dialect::Codex {
        toml_edit::de::from_str(text).map_err(|_| "TOML 格式无效，请先在工具中修复配置")?
    } else {
        let tree = CstRootNode::parse(text, &ParseOptions::default())
            .map_err(|_| "JSON/JSONC 格式无效")?;
        if let Some(node) = tree.value() {
            reject_duplicate_keys(&node)?;
        }
        jsonc_parser::parse_to_serde_value::<Value>(text, &ParseOptions::default())
            .map_err(|_| "JSON/JSONC 格式无效，请先修复配置")?
    };
    let root = value.as_object().ok_or("配置根节点必须是对象")?;
    match root.get(adapter.root_key) {
        None => Ok(BTreeMap::new()),
        Some(Value::Object(items)) => {
            Ok(items.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        }
        _ => Err(format!("{} 必须是对象，已阻止覆盖", adapter.root_key)),
    }
}

fn reject_duplicate_keys(node: &CstNode) -> Result<()> {
    if let Some(object) = node.as_object() {
        let mut seen = std::collections::BTreeSet::new();
        for property in object.properties() {
            let name = property.decoded_name().ok_or("无效的 JSON 属性名")?;
            if !seen.insert(name) {
                return Err("配置包含重复属性名，已阻止歧义读写，请先修复原文件".into());
            }
            if let Some(child) = property.value() {
                reject_duplicate_keys(&child)?;
            }
        }
    } else if let Some(array) = node.as_array() {
        for child in array.elements() {
            reject_duplicate_keys(&child)?;
        }
    }
    Ok(())
}

fn string(obj: &Map<String, Value>, key: &str) -> Result<String> {
    match obj.get(key) {
        None => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{key} 不是字符串；此服务先只读保留")),
    }
}
fn strings(obj: &Map<String, Value>, key: &str) -> Result<Vec<String>> {
    match obj.get(key) {
        None => Ok(vec![]),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{key} 必须是字符串数组"))
            })
            .collect(),
        _ => Err(format!("{key} 必须是数组")),
    }
}
fn map(obj: &Map<String, Value>, key: &str) -> Result<BTreeMap<String, String>> {
    match obj.get(key) {
        None => Ok(BTreeMap::new()),
        Some(Value::Object(a)) => a
            .iter()
            .map(|(k, v)| {
                v.as_str()
                    .map(|s| (k.clone(), s.into()))
                    .ok_or_else(|| format!("{key}.{k} 不是字符串；此服务先只读保留"))
            })
            .collect(),
        _ => Err(format!("{key} 必须是键值对象")),
    }
}

pub fn decode(adapter: &Adapter, raw: &Value) -> Result<Config> {
    let obj = raw.as_object().ok_or("此条目不是服务对象，先只读保留")?;
    let kind = obj.get("type").and_then(Value::as_str).unwrap_or("");
    if ![
        "",
        "stdio",
        "local",
        "remote",
        "http",
        "streamable-http",
        "streamableHttp",
        "sse",
    ]
    .contains(&kind)
    {
        return Err("暂不支持此服务类型，原配置保持不变".into());
    }
    let (command, args) = if adapter.dialect == Dialect::OpenCode && kind == "local" {
        let mut arr = strings(obj, "command")?;
        if arr.is_empty() {
            return Err("OpenCode command 数组为空".into());
        }
        (arr.remove(0), arr)
    } else {
        (string(obj, "command")?, strings(obj, "args")?)
    };
    if adapter.dialect == Dialect::Roo
        && command.is_empty()
        && !matches!(kind, "streamable-http" | "sse")
    {
        return Err("Roo Code 远程服务必须显式指定 streamable-http 或 sse 类型".into());
    }
    let transport = if !command.is_empty() {
        Transport::Stdio
    } else if kind == "sse"
        || (adapter.dialect == Dialect::Gemini
            && obj.contains_key("url")
            && !obj.contains_key("httpUrl"))
        || (adapter.dialect == Dialect::Cline && kind.is_empty())
    {
        Transport::Sse
    } else {
        Transport::Http
    };
    let url_key = if obj.contains_key("httpUrl") {
        "httpUrl"
    } else if obj.contains_key("serverUrl") {
        "serverUrl"
    } else {
        "url"
    };
    let config = Config {
        transport,
        command,
        args,
        cwd: string(obj, "cwd")?,
        url: string(obj, url_key)?,
        env: map(
            obj,
            if adapter.dialect == Dialect::OpenCode {
                "environment"
            } else {
                "env"
            },
        )?,
        headers: map(
            obj,
            if adapter.dialect == Dialect::Codex {
                "http_headers"
            } else {
                "headers"
            },
        )?,
    };
    config.validate()?;
    if !adapter.transports.contains(&config.transport) {
        return Err("此适配器暂不支持该传输方式；原配置保持不变".into());
    }
    Ok(config)
}

pub fn encode(adapter: &Adapter, config: &Config, base: Option<&Value>) -> Result<Value> {
    config.validate()?;
    if !adapter.transports.contains(&config.transport) {
        return Err(format!("{} 不支持此传输方式", adapter.name));
    }
    if let Some(base) = base {
        if decode(adapter, base).as_ref().ok() == Some(config) {
            return Ok(base.clone());
        }
    }
    if !adapter.supports_cwd && !config.cwd.is_empty() {
        return Err(format!(
            "{} 未声明 cwd 支持，请使用支持此字段的目标或清空工作目录",
            adapter.name
        ));
    }
    let mut raw = base.and_then(Value::as_object).cloned().unwrap_or_default();
    for key in [
        "type",
        "command",
        "args",
        "cwd",
        "env",
        "environment",
        "url",
        "httpUrl",
        "serverUrl",
        "headers",
        "http_headers",
    ] {
        raw.remove(key);
    }
    if config.transport == Transport::Stdio {
        if adapter.dialect == Dialect::OpenCode {
            raw.insert("type".into(), json!("local"));
            raw.insert(
                "command".into(),
                json!(std::iter::once(&config.command)
                    .chain(config.args.iter())
                    .collect::<Vec<_>>()),
            );
        } else {
            raw.insert("command".into(), json!(config.command));
            raw.insert("args".into(), json!(config.args));
            if adapter.dialect == Dialect::Copilot {
                raw.insert("type".into(), json!("local"));
            } else if adapter.dialect == Dialect::VsCode {
                raw.insert("type".into(), json!("stdio"));
            }
        }
        if !config.env.is_empty() {
            raw.insert(
                if adapter.dialect == Dialect::OpenCode {
                    "environment"
                } else {
                    "env"
                }
                .into(),
                json!(config.env),
            );
        }
        if !config.cwd.is_empty() {
            raw.insert("cwd".into(), json!(config.cwd));
        }
    } else {
        let url_key = match adapter.dialect {
            Dialect::Gemini if config.transport == Transport::Http => "httpUrl",
            Dialect::Windsurf => "serverUrl",
            _ => "url",
        };
        raw.insert(url_key.into(), json!(config.url));
        let kind = if config.transport == Transport::Sse {
            "sse"
        } else {
            match adapter.dialect {
                Dialect::OpenCode => "remote",
                Dialect::Cline => "streamableHttp",
                Dialect::Roo => "streamable-http",
                _ => "http",
            }
        };
        if !matches!(
            adapter.dialect,
            Dialect::Codex | Dialect::Gemini | Dialect::Windsurf | Dialect::Url
        ) {
            raw.insert("type".into(), json!(kind));
        }
        if !config.headers.is_empty() {
            raw.insert(
                if adapter.dialect == Dialect::Codex {
                    "http_headers"
                } else {
                    "headers"
                }
                .into(),
                json!(config.headers),
            );
        }
    }
    if adapter.dialect == Dialect::Copilot {
        raw.entry("tools").or_insert(json!(["*"]));
    }
    Ok(Value::Object(raw))
}

fn input(v: &Value) -> CstInputValue {
    match v {
        Value::Null => CstInputValue::Null,
        Value::Bool(b) => CstInputValue::Bool(*b),
        Value::Number(n) => CstInputValue::Number(n.to_string()),
        Value::String(s) => CstInputValue::String(s.clone()),
        Value::Array(a) => CstInputValue::Array(a.iter().map(input).collect()),
        Value::Object(o) => {
            CstInputValue::Object(o.iter().map(|(k, v)| (k.clone(), input(v))).collect())
        }
    }
}

pub fn patch(
    adapter: &Adapter,
    original: Option<&str>,
    patches: &BTreeMap<String, Option<Value>>,
) -> Result<String> {
    let source = original.unwrap_or(if adapter.dialect == Dialect::Codex {
        ""
    } else {
        "{\n}\n"
    });
    parse(adapter, source)?;
    let output = if adapter.dialect == Dialect::Codex {
        let mut doc = source
            .parse::<toml_edit::DocumentMut>()
            .map_err(|_| "无法解析 TOML")?;
        if !doc.contains_key(adapter.root_key) {
            doc[adapter.root_key] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        let servers = doc[adapter.root_key]
            .as_table_like_mut()
            .ok_or("MCP 节点不是 TOML 表")?;
        for (name, value) in patches {
            match value {
                Some(v) => {
                    let fragment = toml_edit::ser::to_string(&json!({"service":v}))
                        .map_err(|_| "无法生成 TOML 配置")?;
                    let mut part = fragment
                        .parse::<toml_edit::DocumentMut>()
                        .map_err(|_| "无法生成 TOML 表")?;
                    servers.insert(name, part.remove("service").ok_or("服务为空")?);
                }
                None => {
                    servers.remove(name);
                }
            }
        }
        doc.to_string()
    } else {
        let doc =
            CstRootNode::parse(source, &ParseOptions::default()).map_err(|_| "无法解析 JSONC")?;
        let root = doc.object_value_or_set();
        if root.get(adapter.root_key).is_none() {
            root.append(adapter.root_key, CstInputValue::Object(vec![]));
        }
        let servers = root
            .get(adapter.root_key)
            .and_then(|p| p.value())
            .and_then(|v| v.as_object())
            .ok_or("MCP 节点不是对象")?;
        for (name, value) in patches {
            match (servers.get(name), value) {
                (Some(prop), Some(v)) => {
                    prop.set_value(input(v));
                }
                (None, Some(v)) => {
                    servers.append(name, input(v));
                }
                (Some(prop), None) => {
                    prop.remove();
                }
                (None, None) => {}
            }
        }
        doc.to_string()
    };
    let parsed = parse(adapter, &output)?;
    for (key, v) in patches {
        if parsed.get(key) != v.as_ref() {
            return Err("写入前回读校验失败".into());
        }
    }
    Ok(output)
}

//! Conversion between client-specific service fields and the shared configuration.
use super::{Adapter, Dialect};
use crate::model::{Config, Result, Transport};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

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

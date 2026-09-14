//! Document parsing and local edits that preserve unrelated configuration content.
use super::{Adapter, Dialect};
use crate::model::Result;
use jsonc_parser::{
    cst::{CstInputValue, CstNode, CstRootNode},
    ParseOptions,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

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
        if doc[adapter.root_key].is_inline_table() {
            let root = std::mem::take(&mut doc[adapter.root_key]);
            doc[adapter.root_key] = toml_edit::Item::Table(
                root.into_table().map_err(|_| "MCP 节点不是 TOML 表")?,
            );
        }
        let servers = doc[adapter.root_key]
            .as_table_mut()
            .ok_or("MCP 节点不是 TOML 表")?;
        for (name, value) in patches {
            match value {
                Some(v) => {
                    let fragment = toml_edit::ser::to_string(&json!({"service":v}))
                        .map_err(|_| "无法生成 TOML 配置")?;
                    let mut part = fragment
                        .parse::<toml_edit::DocumentMut>()
                        .map_err(|_| "无法生成 TOML 表")?;
                    let mut table = part
                        .remove("service")
                        .ok_or("服务为空")?
                        .into_table()
                        .map_err(|_| "服务配置必须是 TOML 表")?;
                    table.set_implicit(false);
                    servers.insert(name, toml_edit::Item::Table(table));
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

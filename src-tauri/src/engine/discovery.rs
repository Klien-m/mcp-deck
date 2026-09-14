use super::Discovery;
use crate::{adapters, model::*};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn discover(
    workspace: &Workspace,
    target: &Target,
    entries: BTreeMap<String, Value>,
) -> Result<Vec<Discovery>> {
    let target_id = target.id.as_str();
    let adapter = adapters::get(&target.adapter_id)?;
    Ok(entries
        .into_iter()
        .map(|(key, raw)| {
            let decoded = adapters::decode(&adapter, &raw);
            let managed = workspace.services.iter().any(|s| {
                s.key == key && s.bindings.get(target_id).is_some_and(|b| b.raw.is_some())
            });
            Discovery {
                key,
                config: decoded.as_ref().ok().cloned(),
                preview: redact(&raw),
                error: decoded.err(),
                managed,
            }
        })
        .collect())
}

pub(super) fn adopt(
    workspace: &mut Workspace,
    target: &Target,
    entries: &BTreeMap<String, Value>,
    keys: Vec<String>,
) -> Result<()> {
    let target_id = target.id.as_str();
    let adapter = adapters::get(&target.adapter_id)?;
    let mut unique = BTreeSet::new();
    for key in keys {
        if !unique.insert(key.clone()) {
            continue;
        }
        if workspace.services.iter().any(|s| {
            s.key == key
                && (s.targets.contains(&target.id)
                    || s.bindings.get(target_id).is_some_and(|b| b.raw.is_some()))
        }) {
            return Err(format!("{key} 已在该目标纳入管理"));
        }
        let raw = entries.get(&key).ok_or("配置已变化，请刷新发现列表")?;
        let config = adapters::decode(&adapter, raw)?;
        workspace.services.push(Service {
            id: id(),
            name: key.clone(),
            key,
            description: format!("导入自 {}", target.name),
            config,
            targets: vec![target.id.clone()],
            bindings: BTreeMap::from([(
                target.id.clone(),
                Binding {
                    raw: Some(raw.clone()),
                },
            )]),
            native: BTreeMap::from([(adapter.id.into(), raw.clone())]),
            deleted: false,
            deleted_targets: vec![],
        });
    }
    if workspace.services.len() > 500 {
        return Err("服务数量超过内部版上限".into());
    }
    Ok(())
}

pub(super) fn import_text(
    workspace: &mut Workspace,
    adapter_id: &str,
    text: &str,
) -> Result<usize> {
    if text.len() > 1024 * 1024 {
        return Err("导入内容不能超过 1 MB".into());
    }
    let adapter = adapters::get(adapter_id)?;
    let entries = adapters::parse(&adapter, text)?;
    if entries.is_empty() || entries.len() > 100 {
        return Err("一次导入需要包含 1–100 个服务".into());
    }
    for (key, raw) in &entries {
        let config = adapters::decode(&adapter, raw)?;
        workspace.services.push(Service {
            id: id(),
            name: key.clone(),
            key: key.clone(),
            description: format!("导入 {} 格式；尚未分配", adapter.name),
            config,
            targets: vec![],
            bindings: BTreeMap::new(),
            native: BTreeMap::from([(adapter.id.into(), raw.clone())]),
            deleted: false,
            deleted_targets: vec![],
        });
    }
    if workspace.services.len() > 500 {
        return Err("服务数量超过内部版上限".into());
    }
    Ok(entries.len())
}

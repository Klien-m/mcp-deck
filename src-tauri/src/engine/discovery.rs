//! 发现与导入：来源文件只读，完整原生条目留在服务库中用于无损回写。
//! 调用方必须传入草稿并在整个批次成功后提交，不能持久化中途追加的部分服务。

use super::{targets, Adoption, Discovery, TargetDiscovery};
use crate::{adapters, model::*};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// 不把不存在或没有 MCP 的配置文件列为纳管候选，读取错误独立保留。
pub(super) fn discover_all(workspace: &Workspace) -> Vec<TargetDiscovery> {
    workspace
        .targets
        .iter()
        .filter_map(|target| {
            let result = targets::read_target(target)
                .and_then(|(_, entries)| discover(workspace, target, entries));
            match result {
                Ok(items) if items.is_empty() => None,
                Ok(items) => Some(TargetDiscovery {
                    target: target.clone(),
                    items,
                    error: None,
                }),
                Err(error) => Some(TargetDiscovery {
                    target: target.clone(),
                    items: vec![],
                    error: Some(error),
                }),
            }
        })
        .collect()
}

/// 仅更新调用方提供的草稿；任何目标失败都不应提交部分服务或完成标记。
pub(super) fn complete_onboarding(
    workspace: &mut Workspace,
    selections: Vec<Adoption>,
) -> Result<()> {
    for selection in selections {
        if selection.keys.is_empty() {
            continue;
        }
        let target = targets::find_target(workspace, &selection.target_id)?;
        let (_, entries) = targets::read_target(&target)
            .map_err(|error| format!("{}：{error}", target.name))?;
        adopt(workspace, &target, &entries, selection.keys)
            .map_err(|error| format!("{}：{error}", target.name))?;
    }
    workspace.onboarding_complete = true;
    Ok(())
}

/// 逐项解码；不支持项仍以脱敏预览和错误返回，便于展示只读状态。
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

/// 按唯一配置键接管当前目标条目，记录来源基线和原生字段；缺失或已管理时拒绝。
pub(super) fn adopt(
    workspace: &mut Workspace,
    target: &Target,
    entries: &BTreeMap<String, Value>,
    keys: Vec<String>,
) -> Result<()> {
    let target_id = target.id.as_str();
    let adapter = adapters::get(&target.adapter_id)?;
    // 同批次重复选择只处理一次；跨服务的同键目标占用仍由下面的校验拒绝。
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

/// 受大小和数量限制的文本导入；仅保留来源格式，不建立实际目标绑定。
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

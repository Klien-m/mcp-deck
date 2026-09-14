//! Pure change planning from a workspace and already-read target snapshots.
use super::FileEdit;
use crate::{adapters, model::*};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) struct TargetSnapshot<'a> {
    pub target: &'a Target,
    pub contents: Result<(Option<String>, BTreeMap<String, Value>)>,
}

pub(super) struct Draft {
    pub changes: Vec<Change>,
    pub errors: Vec<String>,
    pub files: Vec<FileEdit>,
}

pub(super) fn relevant_services<'a>(
    workspace: &'a Workspace,
    target: &'a Target,
) -> impl Iterator<Item = &'a Service> {
    workspace.services.iter().filter(move |service| {
        service.targets.contains(&target.id)
            || service
                .bindings
                .get(&target.id)
                .is_some_and(|binding| binding.raw.is_some())
    })
}

pub(super) fn build(workspace: &Workspace, snapshots: Vec<TargetSnapshot<'_>>) -> Result<Draft> {
    let mut changes = vec![];
    let mut errors = vec![];
    let mut files = vec![];
    if workspace
        .history
        .iter()
        .any(|h| h.status == "recovery-needed")
    {
        errors.push("存在未解决的恢复记录，请先在同步记录中处理".into());
    }
    for snapshot in snapshots {
        let target = snapshot.target;
        let relevant = relevant_services(workspace, target);
        let (before, entries) = match snapshot.contents {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(format!("{}：{error}", target.name));
                continue;
            }
        };
        let adapter = adapters::get(&target.adapter_id)?;
        let mut patches = BTreeMap::new();
        for service in relevant {
            let expected = service
                .bindings
                .get(&target.id)
                .and_then(|b| b.raw.as_ref());
            let actual = entries.get(&service.key);
            let wanted = !service.deleted && service.targets.contains(&target.id);
            let desired = if wanted {
                let base = expected.or_else(|| service.native.get(&target.adapter_id));
                match adapters::encode(&adapter, &service.config, base) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        errors.push(format!("{} / {}：{e}", service.name, target.name));
                        continue;
                    }
                }
            } else {
                None
            };
            let conflict = actual != expected;
            if desired.as_ref() == actual && !conflict {
                continue;
            }
            let message = if conflict {
                if expected.is_none() {
                    "发现同名已有配置，需要明确选择保留哪一版"
                } else {
                    "磁盘配置与上次同步基线不同"
                }
            } else {
                ""
            };
            let action = if !wanted {
                "remove"
            } else if actual.is_none() {
                "add"
            } else {
                "update"
            };
            changes.push(Change {
                service_id: service.id.clone(),
                target_id: target.id.clone(),
                target_name: target.name.clone(),
                key: service.key.clone(),
                action: action.into(),
                before: actual.cloned(),
                after: desired.clone(),
                conflict,
                message: message.into(),
            });
            if patches.insert(service.key.clone(), desired).is_some() {
                errors.push(format!("{} 中存在重复配置键 {}", target.name, service.key));
            }
        }
        if !patches.is_empty() {
            match adapters::patch(&adapter, before.as_deref(), &patches) {
                Ok(after) => files.push(FileEdit {
                    target_id: target.id.clone(),
                    path: target.path.clone(),
                    before,
                    after: Some(after),
                }),
                Err(e) => errors.push(format!("{}：{e}", target.name)),
            }
        }
    }
    Ok(Draft {
        changes,
        errors,
        files,
    })
}

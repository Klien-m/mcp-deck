//! 基于已读取的目标快照计算差异，无文件读写或工作区提交。
//! 比较磁盘实际值、上次绑定基线和服务库期望值，输出展示差异、文件草稿及阻断原因。

use super::FileEdit;
use crate::{adapters, model::*};
use serde_json::Value;
use std::collections::BTreeMap;

/// 单次目标读取结果；读取错误作为输入保留，使其他目标仍能生成诊断信息。
pub(super) struct TargetSnapshot<'a> {
    pub target: &'a Target,
    pub contents: Result<(Option<String>, BTreeMap<String, Value>)>,
}

/// 未授权执行的计划草稿；Engine 只有在无错误且无冲突时才缓存为 Plan。
pub(super) struct Draft {
    pub changes: Vec<Change>,
    pub errors: Vec<String>,
    pub files: Vec<FileEdit>,
}

/// 期望分配或仍有原始绑定的服务均需参与；后者用于生成取消分配和软删除差异。
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

/// 对每个目标生成条目补丁和完整文件文本；输出原始值，展示脱敏由同步层处理。
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
            // expected 是上次确认的基线，actual 是本次磁盘值，desired 是本次期望值。
            let expected = service
                .bindings
                .get(&target.id)
                .and_then(|b| b.raw.as_ref());
            let actual = entries.get(&service.key);
            let wanted = !service.deleted && service.targets.contains(&target.id);
            let desired = if wanted {
                // 优先沿用该目标的原生字段；首次分配才回退到来源适配器的原始条目。
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
            // 即使磁盘碰巧等于期望值，只要偏离已确认基线，也要求用户明确解决冲突。
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
            // 多个服务争用同一配置键时阻断整次应用，避免后一个补丁静默覆盖前一个。
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

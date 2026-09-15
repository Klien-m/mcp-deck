//! 服务库中的创建、分配、软删除与冲突选择。
//! 函数修改调用方提供的草稿；批次失败后的丢弃和持久化由 Engine::update 负责。

use super::targets::find_target;
use crate::{adapters, model::*};
use serde_json::Value;
use std::collections::BTreeMap;

/// 校验配置与展示字段；编辑保留 ID 和配置键，新建服务不自动分配目标。
pub(super) fn save(workspace: &mut Workspace, input: ServiceInput) -> Result<String> {
    input.config.validate()?;
    if input.name.trim().is_empty()
        || input.name.len() > 120
        || input.key.trim().is_empty()
        || input.key.len() > 120
        || input.key.chars().any(char::is_control)
    {
        return Err("服务名称与配置键不能为空、过长或包含控制字符".into());
    }
    if input.description.len() > 1000 {
        return Err("描述不能超过 1000 字节".into());
    }
    let service_id = match input.id {
        Some(id) => {
            let service = workspace
                .services
                .iter_mut()
                .find(|s| s.id == id && !s.deleted)
                .ok_or("服务不存在")?;
            if service.key != input.key {
                return Err("现有服务的配置键不可修改，请新建服务后分配".into());
            }
            service.name = input.name.trim().into();
            service.description = input.description;
            service.config = input.config;
            id
        }
        None => {
            if workspace.services.len() >= 500 {
                return Err("内部版最多管理 500 个服务（含待移除项）".into());
            }
            let sid = id();
            workspace.services.push(Service {
                id: sid.clone(),
                key: input.key,
                name: input.name.trim().into(),
                description: input.description,
                config: input.config,
                targets: vec![],
                bindings: BTreeMap::new(),
                native: BTreeMap::new(),
                deleted: false,
                deleted_targets: vec![],
            });
            sid
        }
    };
    Ok(service_id)
}

/// 校验目标兼容性及同键占用，修改期望分配但保留上次同步绑定。
pub(super) fn assign(
    workspace: &mut Workspace,
    service_id: &str,
    target_id: &str,
    enabled: bool,
) -> Result<()> {
    let target = find_target(workspace, target_id)?;
    let service = workspace
        .services
        .iter()
        .find(|s| s.id == service_id && !s.deleted)
        .ok_or("服务不存在")?;
    if enabled {
        // 已取消分配但尚未应用的旧绑定仍占用配置键，不能被另一个服务抢占。
        if workspace.services.iter().any(|s| {
            s.id != service_id
                && s.key == service.key
                && (s.targets.contains(&target.id)
                    || s.bindings.get(&target.id).is_some_and(|b| b.raw.is_some()))
        }) {
            return Err("该工具中已有另一个同配置键服务，请先移除旧绑定并应用".into());
        }
        adapters::encode(
            &adapters::get(&target.adapter_id)?,
            &service.config,
            service.bindings.get(target_id).and_then(|b| b.raw.as_ref()),
        )?;
    }
    let service = workspace
        .services
        .iter_mut()
        .find(|s| s.id == service_id)
        .unwrap();
    // 仅改期望分配；保留 bindings 才能在下次预览中识别需要移除的磁盘条目。
    service.targets.retain(|t| t != target_id);
    if enabled {
        service.targets.push(target_id.into());
    }
    Ok(())
}

/// 软删除时记住分配列表，undo 用其恢复；条目不会立即从工作区移除。
pub(super) fn remove(workspace: &mut Workspace, service_id: &str, undo: bool) -> Result<()> {
    let service = workspace
        .services
        .iter_mut()
        .find(|s| s.id == service_id)
        .ok_or("服务不存在")?;
    if undo {
        service.deleted = false;
        service.targets = service.deleted_targets.clone();
    } else {
        service.deleted = true;
        service.deleted_targets = service.targets.clone();
        service.targets.clear();
    }
    Ok(())
}

/// 以本次读取的目标条目更新基线；采用磁盘时同时更新可解码配置或取消分配。
pub(super) fn resolve(
    workspace: &mut Workspace,
    target: &Target,
    entries: &BTreeMap<String, Value>,
    service_id: &str,
    use_disk: bool,
) -> Result<()> {
    let target_id = target.id.as_str();
    let service = workspace
        .services
        .iter_mut()
        .find(|s| s.id == service_id)
        .ok_or("服务不存在")?;
    let raw = entries.get(&service.key).cloned();
    if use_disk {
        match &raw {
            Some(v) => {
                service.config = adapters::decode(&adapters::get(&target.adapter_id)?, v)?;
                service.deleted = false;
                if !service.targets.contains(&target.id) {
                    service.targets.push(target.id.clone());
                }
            }
            None => {
                service.targets.retain(|t| t != target_id);
            }
        }
    }
    // 保留服务库版本也要确认最新磁盘基线，随后才能生成不带冲突的新计划。
    service.bindings.insert(target_id.into(), Binding { raw });
    Ok(())
}

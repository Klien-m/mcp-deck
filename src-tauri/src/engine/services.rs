use super::targets::find_target;
use crate::{adapters, model::*};
use serde_json::Value;
use std::collections::BTreeMap;

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
    service.targets.retain(|t| t != target_id);
    if enabled {
        service.targets.push(target_id.into());
    }
    Ok(())
}

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
    service.bindings.insert(target_id.into(), Binding { raw });
    Ok(())
}

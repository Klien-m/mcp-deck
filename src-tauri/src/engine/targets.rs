use crate::{adapters, model::*, storage};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(super) fn save(workspace: &mut Workspace, data_dir: &Path, mut target: Target) -> Result<()> {
    adapters::get(&target.adapter_id)?;
    storage::validate_path(Path::new(&target.path))?;
    target.path = Path::new(&target.path)
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .into();
    if Path::new(&target.path).starts_with(data_dir) {
        return Err("目标配置不能位于 MCP Deck 数据目录内".into());
    }
    if target.name.trim().is_empty() || target.name.len() > 100 {
        return Err("请输入目标名称（不超过 100 字节）".into());
    }
    if workspace
        .targets
        .iter()
        .any(|t| t.id != target.id && t.path == target.path)
    {
        return Err("该路径已由另一个目标管理".into());
    }
    if let Some(old) = workspace.targets.iter_mut().find(|t| t.id == target.id) {
        if (old.path != target.path || old.adapter_id != target.adapter_id)
            && workspace.services.iter().any(|s| {
                s.targets.contains(&target.id)
                    || s.bindings.get(&target.id).is_some_and(|b| b.raw.is_some())
            })
        {
            return Err("该目标仍有关联配置，请先取消分配并应用，再修改路径".into());
        }
        *old = target;
    } else {
        if workspace.targets.len() >= 40 {
            return Err("最多支持 40 个配置目标".into());
        }
        workspace.targets.push(Target { id: id(), ..target });
    }
    Ok(())
}

pub(super) fn find_target(workspace: &Workspace, id: &str) -> Result<Target> {
    workspace
        .targets
        .iter()
        .find(|target| target.id == id)
        .cloned()
        .ok_or_else(|| "目标工具不存在".into())
}

pub(super) fn read_target(target: &Target) -> Result<(Option<String>, BTreeMap<String, Value>)> {
    storage::validate_path(Path::new(&target.path))?;
    let text = storage::read_config(Path::new(&target.path))?;
    let entries = match &text {
        Some(text) => adapters::parse(&adapters::get(&target.adapter_id)?, text)?,
        None => BTreeMap::new(),
    };
    Ok((text, entries))
}

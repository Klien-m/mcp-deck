use crate::{adapters, model::*, storage};
use std::{collections::BTreeMap, path::Path};

pub(super) fn export(
    workspace: &Workspace,
    adapter_id: &str,
    service_ids: &[String],
    include_secrets: bool,
) -> Result<String> {
    let adapter = adapters::get(adapter_id)?;
    let mut patches = BTreeMap::new();
    for service in workspace
        .services
        .iter()
        .filter(|s| !s.deleted && (service_ids.is_empty() || service_ids.contains(&s.id)))
    {
        let raw = adapters::encode(&adapter, &service.config, service.native.get(adapter_id))?;
        let raw = if include_secrets { raw } else { redact(&raw) };
        if patches.insert(service.key.clone(), Some(raw)).is_some() {
            return Err("存在相同配置键，请选择一个服务单独导出".into());
        }
    }
    adapters::patch(&adapter, None, &patches)
}

pub(super) fn save_export(
    workspace: &Workspace,
    data_dir: &Path,
    adapter_id: &str,
    service_ids: &[String],
    include_secrets: bool,
    path: &Path,
) -> Result<()> {
    storage::validate_path(path)?;
    if path.starts_with(data_dir)
        || workspace
            .targets
            .iter()
            .any(|target| Path::new(&target.path) == path)
    {
        return Err("导出不能覆盖数据目录或正在管理的配置，请选择其他文件".into());
    }
    let text = export(workspace, adapter_id, service_ids, include_secrets)?;
    storage::atomic_write(path, &text, true)
}

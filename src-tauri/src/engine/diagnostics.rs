use crate::model::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub(super) fn checks(workspace: &Workspace, home: &Path, service_id: &str) -> Result<Value> {
    let service = workspace
        .services
        .iter()
        .find(|s| s.id == service_id)
        .ok_or("服务不存在")?;
    let mut issues = vec![];
    if let Err(e) = service.config.validate() {
        issues.push(e);
    }
    for (key, value) in service
        .config
        .env
        .iter()
        .chain(service.config.headers.iter())
    {
        if value.trim().is_empty() || value.contains("<已隐藏>") {
            issues.push(format!("{key} 尚未填写"));
        }
        if value.contains("${") || value.contains("{env:") {
            issues.push(format!("{key} 使用变量引用，需在目标工具的运行环境中确认"));
        }
    }
    let mut executable = None;
    if service.config.transport == Transport::Stdio {
        let command = &service.config.command;
        let candidates = if Path::new(command).is_absolute() {
            vec![PathBuf::from(command)]
        } else {
            let mut dirs = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .collect::<Vec<_>>();
            dirs.extend([
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
                home.join(".local/bin"),
            ]);
            dirs.into_iter().map(|p| p.join(command)).collect()
        };
        executable = candidates
            .into_iter()
            .find(|p| p.is_file())
            .map(|p| p.to_string_lossy().into_owned());
        if executable.is_none() {
            issues.push("未发现启动命令，请安装依赖或使用可执行文件的完整路径".into());
        }
    }
    Ok(
        json!({"issues":issues,"executable":executable,"note":"仅检查配置字段与命令位置；未启动进程、未执行网络连接。最终加载和认证状态请在目标工具中确认。"}),
    )
}

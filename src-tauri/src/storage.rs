use crate::model::Result;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub const MAX_CONFIG_BYTES: u64 = 2 * 1024 * 1024;

pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| format!("无法创建数据目录：{e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock_exclusive()
        .map_err(|_| "此工作区已被另一个 MCP Deck 实例打开")?;
    Ok(file)
}

pub fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("配置路径必须是绝对路径，且不能包含 ..".into());
    }
    if !matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("json" | "jsonc" | "toml")
    ) {
        return Err("请选择 .json、.jsonc 或 .toml 配置文件".into());
    }
    reject_symlinks(path)
}

pub fn reject_symlinks(path: &Path) -> Result<()> {
    // Follow no links during a write. A user can explicitly select the resolved real path instead.
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!(
                    "路径包含符号链接，请选择其真实位置：{}",
                    ancestor.display()
                ))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("无法检查配置路径：{e}")),
        }
    }
    Ok(())
}

pub fn read_config(path: &Path) -> Result<Option<String>> {
    reject_symlinks(path)?;
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("无法读取配置：{e}")),
    };
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > MAX_CONFIG_BYTES {
        return Err("配置必须是小于 2 MB 的普通文件".into());
    }
    let mut text = String::new();
    (&mut file)
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|_| "配置不是有效的 UTF-8 文本")?;
    if text.len() as u64 > MAX_CONFIG_BYTES {
        return Err("配置超过 2 MB".into());
    }
    Ok(Some(text))
}

pub fn fingerprint(text: Option<&str>) -> String {
    match text {
        None => "missing".into(),
        Some(t) => format!("{:x}", Sha256::digest(t.as_bytes())),
    }
}

pub fn atomic_write(path: &Path, text: &str, private: bool) -> Result<()> {
    reject_symlinks(path)?;
    let parent = path.parent().ok_or("缺少父目录")?;
    let mut missing = Vec::<PathBuf>::new();
    let mut cursor = parent;
    while !cursor.exists() {
        missing.push(cursor.to_path_buf());
        cursor = cursor.parent().ok_or("无法找到有效的父目录")?;
    }
    for dir in missing.iter().rev() {
        private_dir(dir)?;
    }
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("无法创建临时文件：{e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if private {
            0o600
        } else {
            fs::metadata(path)
                .map(|m| m.permissions().mode() & 0o777)
                .unwrap_or(0o600)
        };
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(mode))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = private;
    temp.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    reject_symlinks(path)?;
    temp.persist(path)
        .map_err(|e| format!("无法替换配置文件：{}", e.error))?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn write_checked(path: &Path, before: Option<&str>, after: Option<&str>) -> Result<()> {
    let current = read_config(path)?;
    if fingerprint(current.as_deref()) != fingerprint(before) {
        return Err(format!(
            "文件在预览后被修改，请重新预览：{}",
            path.display()
        ));
    }
    match after {
        Some(text) => atomic_write(path, text, false)?,
        None => {
            if current.is_some() {
                fs::remove_file(path).map_err(|e| format!("无法恢复文件不存在状态：{e}"))?;
            }
        }
    }
    if fingerprint(read_config(path)?.as_deref()) != fingerprint(after) {
        return Err("文件写入后校验失败，可能存在外部并发修改".into());
    }
    Ok(())
}

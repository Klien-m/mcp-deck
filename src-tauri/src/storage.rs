//! 受约束的文件读写：检查路径、限制配置大小、持有进程锁并原子替换单个文件。
//! 这里提供乐观并发检查，不是操作系统级的多文件事务；跨文件恢复由 transaction 负责。

use crate::model::Result;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

/// 单个目标配置的读取上限；工作区和完整备份采用各自的存储路径与限制。
pub const MAX_CONFIG_BYTES: u64 = 2 * 1024 * 1024;

/// 创建应用所需目录，Unix 下限制为所有者可读写执行（0700）。
pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| format!("无法创建数据目录：{e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 非阻塞获取工作区排他锁；调用方必须保留返回的文件句柄直到工作区关闭。
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

/// 目标和导出路径须为绝对的 JSON/JSONC/TOML 文件路径，拒绝 .. 与符号链接。
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

/// 检查文件及所有祖先是否为符号链接；不存在的末级路径允许后续创建。
/// 这是写入前检查，不能声称消除了不受本进程控制的全部路径竞争。
pub fn reject_symlinks(path: &Path) -> Result<()> {
    // 让用户显式选择真实位置，不把符号链接背后的文件隐式纳入管理。
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

/// 受限读取普通 UTF-8 文件；不存在返回 None，读取失败或格式不符返回错误。
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
    // 读取时再施加上限，避免文件在 metadata 检查后增长导致无界分配。
    (&mut file)
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|_| "配置不是有效的 UTF-8 文本")?;
    if text.len() as u64 > MAX_CONFIG_BYTES {
        return Err("配置超过 2 MB".into());
    }
    Ok(Some(text))
}

/// 对完整原文做 SHA-256；独立标记缺失状态，使空文件与不存在不等价。
pub fn fingerprint(text: Option<&str>) -> String {
    match text {
        None => "missing".into(),
        Some(t) => format!("{:x}", Sha256::digest(t.as_bytes())),
    }
}

/// 在目标同目录创建临时文件，刷盘后原子替换，Unix 下再同步父目录。
/// private=true 强制 0600；否则保留已有文件权限，新文件仍默认私有。
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
    // 同目录临时文件保证替换不跨文件系统；失败时由 NamedTempFile 清理未发布文件。
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

/// 比较预期原文指纹后写入并回读校验；after=None 用于恢复文件原本不存在的状态。
/// 检测到外部变化就返回错误，由事务层决定后续恢复。
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

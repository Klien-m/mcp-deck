//! 按宿主系统解析用户级配置位置；路径选择与环境读取分离，便于在任意系统验证。

use super::{registry, Adapter};
use crate::{
    model::{Target, Workspace},
    storage,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    MacOs,
    Windows,
    Linux,
}

impl Platform {
    fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }

    // 不依赖编译宿主的 Path::is_absolute，跨平台测试也能识别盘符和 UNC 路径。
    fn absolute(self, path: &Path) -> bool {
        let path = path.to_string_lossy();
        if self != Self::Windows {
            return path.starts_with('/');
        }
        let bytes = path.as_bytes();
        let drive = bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\');
        let unc = path
            .strip_prefix("\\\\")
            .or_else(|| path.strip_prefix("//"))
            .is_some_and(|tail| {
                let mut segments = tail.split(['/', '\\']);
                segments.next().is_some_and(|part| !part.is_empty())
                    && segments.next().is_some_and(|part| !part.is_empty())
            });
        drive || unc
    }
}

#[derive(Default)]
struct PathEnvironment {
    app_data: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    codex_home: Option<PathBuf>,
    isolated: bool,
}

impl PathEnvironment {
    fn read(home: &Path) -> Self {
        // Engine 测试传入的临时 home 也应隔离，不能依赖测试进程是否设置环境变量。
        Self {
            app_data: std::env::var_os("APPDATA").map(PathBuf::from),
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            codex_home: std::env::var_os("CODEX_HOME").map(PathBuf::from),
            isolated: std::env::var_os("MCP_DECK_HOME").is_some()
                || dirs::home_dir().as_deref() != Some(home),
        }
    }

    fn directory(&self, value: &Option<PathBuf>, platform: Platform) -> Option<PathBuf> {
        if self.isolated {
            return None;
        }
        value
            .as_ref()
            .filter(|path| {
                // 相对路径、~ 和未展开的变量不能定位用户目录；也不接受越级路径。
                platform.absolute(path)
                    && !path
                        .to_string_lossy()
                        .split(['/', '\\'])
                        .any(|part| part == "..")
            })
            .cloned()
    }

    fn config_dir(&self, home: &Path, platform: Platform) -> PathBuf {
        self.directory(&self.xdg_config_home, platform)
            .unwrap_or_else(|| home.join(".config"))
    }

    fn app_data_dir(&self, home: &Path, platform: Platform) -> PathBuf {
        match platform {
            Platform::MacOs => home.join("Library/Application Support"),
            Platform::Windows => self
                .directory(&self.app_data, platform)
                .unwrap_or_else(|| home.join("AppData/Roaming")),
            Platform::Linux => self.config_dir(home, platform),
        }
    }
}

fn candidates(
    adapter: &Adapter,
    home: &Path,
    platform: Platform,
    environment: &PathEnvironment,
) -> Vec<PathBuf> {
    let vscode_user = environment.app_data_dir(home, platform).join("Code/User");
    match adapter.id {
        "codex" => vec![environment
            .directory(&environment.codex_home, platform)
            .unwrap_or_else(|| home.join(".codex"))
            .join("config.toml")],
        // OpenCode 使用 XDG 目录约定，Windows 也不改为 APPDATA / LOCALAPPDATA。
        "opencode" => {
            let directory = environment.config_dir(home, platform).join("opencode");
            vec![
                directory.join("opencode.jsonc"),
                directory.join("opencode.json"),
            ]
        }
        "vscode" => vec![vscode_user.join("mcp.json")],
        "cline" => vec![
            home.join(".cline/data/settings/cline_mcp_settings.json"),
            home.join(".cline/mcp.json"),
            vscode_user
                .join("globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json"),
        ],
        "roo" => {
            vec![vscode_user
                .join("globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json")]
        }
        "claude-desktop" => vec![environment
            .app_data_dir(home, platform)
            .join("Claude/claude_desktop_config.json")],
        _ => adapter.paths.iter().map(|path| home.join(path)).collect(),
    }
}

fn resolve_targets(
    home: &Path,
    platform: Platform,
    environment: &PathEnvironment,
    is_file: impl Fn(&Path) -> bool,
) -> Vec<Target> {
    registry()
        .iter()
        .filter_map(|adapter| {
            let paths = candidates(adapter, home, platform, environment);
            let existing = paths.iter().find(|path| is_file(path));
            // Linux Desktop 已有官方 beta，但手工 MCP 路径文档尚未统一。
            // 发现已有配置；未出现文件时由用户从 Desktop 设置确认后添加。
            let path = if platform == Platform::Linux && adapter.id == "claude-desktop" {
                existing?
            } else {
                existing.or_else(|| paths.first())?
            };
            Some(Target {
                id: adapter.id.into(),
                adapter_id: adapter.id.into(),
                name: adapter.name.into(),
                path: path.to_string_lossy().into(),
            })
        })
        .collect()
}

/// 优先选已有候选文件，否则使用该系统首选路径；不创建 Agent 配置文件。
/// Linux 仅发现已有的 Claude Desktop 配置，未找到时可手动添加确认后的路径。
pub fn default_targets(home: &Path) -> Vec<Target> {
    resolve_targets(
        home,
        Platform::current(),
        &PathEnvironment::read(home),
        Path::is_file,
    )
}

fn legacy_mac_path(target: &Target, home: &Path, platform: Platform) -> bool {
    if platform == Platform::MacOs || target.id != target.adapter_id {
        return false;
    }
    let relative = match target.adapter_id.as_str() {
        "vscode" => "Library/Application Support/Code/User/mcp.json",
        "cline" => "Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json",
        "roo" => "Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json",
        "claude-desktop" => "Library/Application Support/Claude/claude_desktop_config.json",
        _ => return false,
    };
    Path::new(&target.path) == home.join(relative)
}

/// 对仍有引用、不能自动迁移的旧目标给出可见提示，不误判任意自定义目录。
pub fn legacy_default_path_notice(target: &Target, home: &Path) -> Option<String> {
    legacy_mac_path(target, home, Platform::current()).then(|| {
        "此路径来自旧版 macOS 默认配置，请在工具与配置路径中确认；已有服务关联和备份保持原样".into()
    })
}

fn migrate_targets(
    workspace: &mut Workspace,
    home: &Path,
    platform: Platform,
    environment: &PathEnvironment,
    exists: impl Fn(&Path) -> bool,
    is_file: impl Fn(&Path) -> bool,
) -> bool {
    let defaults = resolve_targets(home, platform, environment, is_file);
    let old_paths: Vec<_> = workspace
        .targets
        .iter()
        .map(|target| target.path.clone())
        .collect();
    let mut changed = false;
    workspace.targets.retain_mut(|target| {
        let Some(adapter) = registry()
            .into_iter()
            .find(|adapter| adapter.id == target.adapter_id)
        else {
            return true;
        };
        let legacy_cline = target.id == "cline"
            && target.adapter_id == "cline"
            && Path::new(&target.path) == home.join(".cline/mcp.json");
        if target.name != adapter.name
            || (!legacy_mac_path(target, home, platform) && !legacy_cline)
            || exists(Path::new(&target.path))
            || workspace.services.iter().any(|service| {
                service.targets.contains(&target.id)
                    || service.deleted_targets.contains(&target.id)
                    || service.bindings.contains_key(&target.id)
            })
            || workspace.history.iter().any(|history| {
                history.paths.iter().any(|path| {
                    storage::same_path_for_platform(
                        Path::new(path),
                        Path::new(&target.path),
                        platform == Platform::Windows,
                    )
                })
            })
        {
            return true;
        }
        if let Some(replacement) = defaults.iter().find(|item| item.id == target.id) {
            // 用户已手动添加的新路径不能再被另一个默认目标重复管理。
            if old_paths.iter().any(|path| {
                storage::same_path_for_platform(
                    Path::new(path),
                    Path::new(&replacement.path),
                    platform == Platform::Windows,
                )
            }) {
                return true;
            }
            target.path.clone_from(&replacement.path);
            changed = true;
            true
        } else {
            changed = true;
            false
        }
    });
    changed
}

/// 仅纠正未使用且不存在的精确旧默认值；不挪动文件、绑定或用户自定义路径。
pub fn migrate_default_targets(workspace: &mut Workspace, home: &Path) -> bool {
    migrate_targets(
        workspace,
        home,
        Platform::current(),
        &PathEnvironment::read(home),
        // 权限不足与悬空符号链接都不等同于“默认文件尚未创建”。
        |path| match std::fs::symlink_metadata(path) {
            Ok(_) => true,
            Err(error) => error.kind() != std::io::ErrorKind::NotFound,
        },
        Path::is_file,
    )
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;

//! 只读静态诊断：不启动命令、不展开客户端变量、不请求远程服务。

use crate::model::*;
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Platform {
    Windows,
    Macos,
    Linux,
}

/// 环境以值传入，测试不修改全局 PATH / PATHEXT 或探测真实用户目录。
struct Environment {
    platform: Platform,
    path: Vec<PathBuf>,
    path_ext: Vec<String>,
    fallback: Vec<PathBuf>,
}

impl Environment {
    fn current(home: &Path) -> Self {
        let platform = if cfg!(windows) {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::Macos
        } else {
            Platform::Linux
        };
        let path = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
        let path_ext =
            extensions(&std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into()));
        let fallback = match platform {
            Platform::Windows => vec![],
            Platform::Macos => vec![
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
                home.join(".local/bin"),
            ],
            Platform::Linux => vec![
                PathBuf::from("/usr/local/bin"),
                home.join(".local/bin"),
                home.join("bin"),
            ],
        };
        Self {
            platform,
            path,
            path_ext,
            fallback,
        }
    }
}

#[derive(Serialize)]
struct Item {
    code: &'static str,
    level: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}

#[derive(Default, Serialize)]
struct Report {
    issues: Vec<String>,
    executable: Option<String>,
    items: Vec<Item>,
    note: &'static str,
}

impl Report {
    fn add(
        &mut self,
        code: &'static str,
        level: &'static str,
        message: impl Into<String>,
        hint: Option<&'static str>,
    ) {
        let message = message.into();
        if level != "ok" {
            self.issues.push(message.clone());
        }
        self.items.push(Item {
            code,
            level,
            message,
            hint,
        });
    }
}

pub(super) fn checks(workspace: &Workspace, home: &Path, service_id: &str) -> Result<Value> {
    let service = workspace
        .services
        .iter()
        .find(|s| s.id == service_id)
        .ok_or("服务不存在")?;
    serde_json::to_value(inspect(&service.config, &Environment::current(home)))
        .map_err(|_| "无法生成静态检查结果".into())
}

fn inspect(config: &Config, environment: &Environment) -> Report {
    let mut report = Report {
        note: "仅检查静态配置与本机文件。未启动进程、未检测连接或认证；执行权限、PATH 和依赖在目标工具中可能不同，最终加载状态请在目标工具中确认。",
        ..Report::default()
    };
    if let Err(message) = config.validate() {
        report.add(
            "config.invalid",
            "error",
            message,
            Some("编辑配置，修正字段后重新检查。"),
        );
    } else {
        report.add("config.valid", "ok", "配置字段格式有效", None);
    }
    // 只显示字段类别和序号，绝不回显凭据、参数或带认证信息的地址。
    for (label, values) in [("环境变量", &config.env), ("请求头", &config.headers)] {
        for (index, value) in values.values().enumerate() {
            if value.trim().is_empty() {
                report.add(
                    "value.empty",
                    "warning",
                    format!("第 {} 个{label}的值为空", index + 1),
                    Some("确认该字段是否允许空值；凭据等必填字段需要补全。"),
                );
            } else if placeholder(value) {
                report.add(
                    "value.missing",
                    "error",
                    format!("第 {} 个{label}为空或仍是占位值", index + 1),
                    Some("打开完整配置补全该字段；隐藏后的导出内容不能作为真实凭据使用。"),
                );
            } else if variable(value) {
                report.add(
                    "value.variable",
                    "warning",
                    format!("第 {} 个{label}使用变量引用，尚未解析", index + 1),
                    Some("确认目标工具支持该变量语法，并在目标工具的运行环境中提供实际值。"),
                );
            }
        }
    }
    for (index, value) in config.args.iter().enumerate() {
        if !value.is_empty() && placeholder(value) {
            report.add(
                "argument.placeholder",
                "error",
                format!("第 {} 个启动参数仍包含占位值", index + 1),
                Some("在完整配置中补全实际值；参数内容不会出现在检查报告中。"),
            );
        } else if variable(value) {
            report.add(
                "argument.variable",
                "warning",
                format!("第 {} 个启动参数使用变量引用，尚未解析", index + 1),
                Some("确认目标工具支持该语法；启动参数不会自动经过 shell 展开。"),
            );
        }
    }
    if config.transport != Transport::Stdio {
        if !config.url.is_empty() && placeholder(&config.url) {
            report.add(
                "url.placeholder",
                "error",
                "服务地址中仍有占位值",
                Some("在完整配置中补全地址；凭据应放入目标工具支持的认证配置。"),
            );
        } else if variable(&config.url) {
            report.add(
                "url.variable",
                "warning",
                "服务地址使用变量引用，尚未解析",
                Some("确认目标工具支持该语法，并提供对应的变量。"),
            );
        }
        report.add(
            "connection.not_checked",
            "warning",
            "未检测远程连接与认证",
            Some("地址格式有效不代表服务可达；请在目标工具中确认授权和连接。"),
        );
        return report;
    }
    let cwd = inspect_cwd(&config.cwd, environment.platform, &mut report);
    inspect_command(config, cwd.as_deref(), environment, &mut report);
    inspect_script(config, cwd.as_deref(), environment.platform, &mut report);
    report
}

fn placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.is_empty()
        || trimmed.contains("<已隐藏>")
        || matches!(
            trimmed,
            "<redacted>" | "[REDACTED]" | "YOUR_API_KEY" | "YOUR_TOKEN" | "***" | "********"
        )
}

fn variable(value: &str) -> bool {
    value.contains("${")
        || value.contains("{env:")
        || value.starts_with('$')
        || value
            .split_once('%')
            .is_some_and(|(_, rest)| rest.contains('%'))
}

fn absolute(path: &str, platform: Platform) -> bool {
    if platform == Platform::Windows {
        let bytes = path.as_bytes();
        // 在其他宿主上测试 Windows 命令查找时也允许临时目录的绝对路径。
        Path::new(path).is_absolute()
            || path.starts_with("\\\\")
            || (bytes.len() > 2
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && matches!(bytes[2], b'\\' | b'/'))
    } else {
        Path::new(path).is_absolute()
    }
}

fn drive_relative(path: &str, platform: Platform) -> bool {
    let bytes = path.as_bytes();
    platform == Platform::Windows
        && bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes.len() == 2 || !matches!(bytes[2], b'\\' | b'/'))
}

fn inspect_cwd(value: &str, platform: Platform, report: &mut Report) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }
    if variable(value) || value.starts_with('~') {
        report.add(
            "cwd.unresolved",
            "warning",
            "工作目录包含尚未解析的变量或 ~",
            Some("建议使用完整路径；变量和 ~ 不会由 MCP Deck 自动展开。"),
        );
        return None;
    }
    if drive_relative(value, platform) {
        report.add(
            "cwd.relative",
            "warning",
            "工作目录使用盘符相对路径，无法确定该盘符的当前目录",
            Some("使用包含盘符和根目录分隔符的完整路径，例如 C:\\project。"),
        );
        return None;
    }
    if !absolute(value, platform) {
        report.add(
            "cwd.relative",
            "warning",
            "工作目录是相对路径，无法确定目标工具的基准目录",
            Some("将工作目录改为完整路径后重新检查。"),
        );
        return None;
    }
    let path = PathBuf::from(value);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_dir() => {
            if !executable_mode(&metadata, platform) {
                report.add(
                    "cwd.permission",
                    "error",
                    "工作目录没有访问所需的执行权限位",
                    Some("检查该目录及父目录的访问权限，或选择可访问的工作目录。"),
                );
                None
            } else {
                report.add("cwd.found", "ok", "工作目录存在", None);
                Some(path)
            }
        }
        Ok(_) => {
            report.add(
                "cwd.not_directory",
                "error",
                "工作目录指向文件，而不是目录",
                Some("选择已有目录作为工作目录。"),
            );
            None
        }
        Err(_) => {
            report.add(
                "cwd.missing",
                "error",
                "工作目录不存在或无法访问",
                Some("确认目录已创建，并检查当前用户的访问权限。"),
            );
            None
        }
    }
}

fn configured_env<'a>(config: &'a Config, name: &str, platform: Platform) -> Option<&'a str> {
    config
        .env
        .iter()
        .find(|(key, _)| {
            if platform == Platform::Windows {
                key.eq_ignore_ascii_case(name)
            } else {
                key.as_str() == name
            }
        })
        .map(|(_, value)| value.as_str())
}

fn split_path(value: &str, platform: Platform) -> Vec<PathBuf> {
    value
        .split(if platform == Platform::Windows {
            ';'
        } else {
            ':'
        })
        .map(|part| {
            PathBuf::from(if platform == Platform::Windows {
                part.trim_matches('"')
            } else {
                part
            })
        })
        .collect()
}

fn extensions(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|part| part.starts_with('.') && !part.contains(['/', '\\']))
        .map(str::to_ascii_lowercase)
        .collect()
}

fn inspect_command(
    config: &Config,
    cwd: Option<&Path>,
    environment: &Environment,
    report: &mut Report,
) {
    let command = &config.command;
    if command.trim().is_empty() || command.contains('\0') {
        return;
    }
    if variable(command) || command.starts_with('~') {
        report.add(
            "command.unresolved",
            "warning",
            "启动命令包含尚未解析的变量或 ~",
            Some("使用可执行文件的完整路径；不要依赖 shell 展开。"),
        );
        return;
    }
    let platform = environment.platform;
    // C:server.exe 的基准是该盘符的隐含当前目录，不能让 Path::join 猜测。
    if drive_relative(command, platform) {
        report.add(
            "command.relative",
            "warning",
            "启动命令使用盘符相对路径，无法确定该盘符的当前目录",
            Some("使用包含盘符和根目录分隔符的完整命令路径，例如 C:\\tools\\server.exe。"),
        );
        return;
    }
    let path_ext = configured_env(config, "PATHEXT", platform)
        .map(extensions)
        .unwrap_or_else(|| environment.path_ext.clone());
    let has_separator =
        command.contains('/') || (platform == Platform::Windows && command.contains('\\'));
    let direct = absolute(command, platform) || has_separator;
    let (directories, source) = if direct {
        if absolute(command, platform) {
            (vec![PathBuf::new()], "absolute")
        } else if let Some(cwd) = cwd {
            (vec![cwd.to_owned()], "cwd")
        } else {
            report.add(
                "command.relative",
                "warning",
                "启动命令是相对路径，缺少可确定的工作目录",
                Some("配置有效的绝对工作目录，或改为可执行文件的完整路径。"),
            );
            return;
        }
    } else if let Some(path) = configured_env(config, "PATH", platform) {
        if variable(path) || placeholder(path) {
            report.add(
                "path.unresolved",
                "warning",
                "配置中的 PATH 无法静态解析，暂不判断命令是否存在",
                Some("提供完整的 PATH 值，或将启动命令改为完整路径。"),
            );
            return;
        }
        (split_path(path, platform), "configured")
    } else {
        (environment.path.clone(), "desktop")
    };
    let mut candidates = vec![];
    let mut unknown_path = false;
    for directory in directories {
        let directory = if direct || directory.is_absolute() {
            directory
        } else if let Some(cwd) = cwd {
            cwd.join(directory)
        } else {
            // 不使用 MCP Deck 的当前目录猜测客户端相对 PATH 的含义。
            unknown_path = true;
            continue;
        };
        candidates.extend(command_candidates(
            &directory.join(command),
            platform,
            &path_ext,
        ));
    }
    if unknown_path {
        report.add(
            "path.relative",
            "warning",
            "PATH 含相对目录，因缺少工作目录而跳过这些候选项",
            Some("使用绝对 PATH 目录或设置有效的工作目录。"),
        );
    }
    let mut not_executable = false;
    let found = candidates.into_iter().find_map(|candidate| {
        let path = existing_file(&candidate, platform)?;
        let metadata = fs::metadata(&path).ok()?;
        if executable_mode(&metadata, platform) {
            Some(path)
        } else {
            not_executable = true;
            None
        }
    });
    if let Some(path) = found {
        report.executable = Some(path.to_string_lossy().into_owned());
        report.add(
            "command.found",
            "ok",
            match source {
                "absolute" => "已找到启动命令文件",
                "cwd" => "已根据工作目录找到启动命令文件",
                "configured" => "已在配置提供的 PATH 中找到命令文件",
                _ => "已在 MCP Deck 的 PATH 中找到命令文件",
            },
            Some("这只确认本机文件与权限位；目标工具的 PATH、权限和运行环境仍需确认。"),
        );
        if platform == Platform::Windows
            && path.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat")
            })
        {
            report.add(
                "command.shell_script",
                "warning",
                "找到 Windows 批处理脚本，客户端可能需要显式 shell",
                Some(
                    "确认目标工具支持该脚本；若直接启动失败，按该工具文档使用 cmd /c 等启动方式。",
                ),
            );
        }
        return;
    }
    if not_executable {
        report.add(
            "command.permission",
            "error",
            "发现命令文件，但未发现具有执行权限位的候选项",
            Some("检查启动文件的执行权限，或改用具有执行权限的完整命令路径。"),
        );
        return;
    }
    // 补查只是定位安装位置；不能把补查成功当成目标客户端 PATH 已正确配置。
    if !direct {
        for directory in &environment.fallback {
            for candidate in command_candidates(&directory.join(command), platform, &path_ext) {
                if let Some(path) = existing_file(&candidate, platform) {
                    report.executable = Some(path.to_string_lossy().into_owned());
                    report.add(
                        "command.fallback",
                        "warning",
                        "仅在补充安装目录发现命令，当前检查的 PATH 未找到它",
                        Some("建议将启动命令改成下方完整路径，或在目标工具启动环境中补全 PATH。"),
                    );
                    if fs::metadata(path)
                        .is_ok_and(|metadata| !executable_mode(&metadata, platform))
                    {
                        report.add(
                            "command.permission",
                            "error",
                            "补查到的命令文件没有执行权限位",
                            Some("检查该文件及父目录的执行权限。"),
                        );
                    }
                    return;
                }
            }
        }
    }
    report.add("command.missing", "error", "未发现启动命令文件", Some("确认依赖已安装，并使用可执行文件的完整路径。Windows 下还需检查 PATHEXT 和脚本启动方式。"));
}

fn command_candidates(path: &Path, platform: Platform, path_ext: &[String]) -> Vec<PathBuf> {
    if platform != Platform::Windows || path.extension().is_some() {
        return vec![path.to_owned()];
    }
    path_ext
        .iter()
        .map(|extension| {
            let mut name = path.as_os_str().to_os_string();
            name.push(extension);
            PathBuf::from(name)
        })
        .collect()
}

fn existing_file(path: &Path, platform: Platform) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_owned());
    }
    if platform == Platform::Windows {
        // 显式比较也让跨宿主测试覆盖 Windows PATHEXT 的大小写。
        let name = path.file_name()?.to_string_lossy();
        return fs::read_dir(path.parent()?)
            .ok()?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&name)
                    && entry.path().is_file()
            })
            .map(|entry| entry.path());
    }
    None
}

fn executable_mode(metadata: &fs::Metadata, platform: Platform) -> bool {
    if platform == Platform::Windows {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        true
    }
}

fn inspect_script(config: &Config, cwd: Option<&Path>, platform: Platform, report: &mut Report) {
    let command = config
        .command
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let interpreter = command.trim_end_matches(".exe");
    let python = matches!(interpreter, "python" | "python3" | "python2" | "py")
        || interpreter.starts_with("python3.");
    if !python && !matches!(interpreter, "node" | "nodejs") {
        return;
    }
    if python && config.args.first().is_some_and(|argument| argument == "-m") {
        report.add(
            "dependency.not_checked",
            "warning",
            "使用 Python 模块启动，尚未验证模块是否安装",
            Some("静态检查不会执行解释器；请在目标工具使用的 Python 环境中确认模块依赖。"),
        );
        return;
    }
    let Some(script) = config.args.first().filter(|value| !value.starts_with('-')) else {
        return;
    };
    if !python && script == "inspect" {
        return;
    }
    if variable(script) || script.starts_with('~') {
        report.add(
            "script.unresolved",
            "warning",
            "入口脚本参数包含尚未解析的变量或 ~",
            Some("使用完整的入口脚本路径。"),
        );
        return;
    }
    let path = if absolute(script, platform) {
        PathBuf::from(script)
    } else if let Some(cwd) = cwd {
        cwd.join(script)
    } else {
        report.add(
            "script.relative",
            "warning",
            "入口脚本参数是相对路径，无法确定客户端中的实际位置",
            Some("配置绝对工作目录，或使用完整脚本路径。"),
        );
        return;
    };
    if !path.is_file() || fs::File::open(&path).is_err() {
        report.add(
            "script.unreadable",
            "error",
            "入口脚本不存在或当前用户无法读取",
            Some("确认入口脚本文件及工作目录，并检查读取权限。"),
        );
    } else {
        report.add(
            "script.readable",
            "ok",
            "入口脚本文件可读取",
            Some("尚未执行脚本，也未检查其依赖、版本或运行结果。"),
        );
    }
}

#[cfg(test)]
mod tests;

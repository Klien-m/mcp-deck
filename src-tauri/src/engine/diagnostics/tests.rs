use super::*;
use std::collections::BTreeMap;
use tempfile::tempdir;

fn config(command: &str) -> Config {
    Config {
        transport: Transport::Stdio,
        command: command.into(),
        args: vec![],
        cwd: String::new(),
        env: BTreeMap::new(),
        url: String::new(),
        headers: BTreeMap::new(),
    }
}

fn environment(platform: Platform, path: Vec<PathBuf>) -> Environment {
    Environment {
        platform,
        path,
        path_ext: extensions(".COM;.EXE;.BAT;.CMD"),
        fallback: vec![],
    }
}

fn file(path: &Path) {
    fs::write(path, "test fixture; never executed").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

fn has(report: &Report, code: &str, level: &str) -> bool {
    report
        .items
        .iter()
        .any(|item| item.code == code && item.level == level)
}

#[test]
fn command_location_does_not_claim_client_connection() {
    let root = tempdir().unwrap();
    let command = root.path().join("server");
    file(&command);
    let report = inspect(
        &config(command.to_str().unwrap()),
        &environment(Platform::Linux, vec![]),
    );
    assert!(has(&report, "command.found", "ok"));
    assert!(report.note.contains("未检测连接"));
    assert_eq!(report.executable.as_deref(), command.to_str());
    assert!(report.issues.is_empty());
}

#[test]
fn fallback_is_warning_not_successful_path_resolution() {
    let root = tempdir().unwrap();
    file(&root.path().join("server"));
    let mut env = environment(Platform::Macos, vec![]);
    env.fallback.push(root.path().to_owned());
    let report = inspect(&config("server"), &env);
    assert!(has(&report, "command.fallback", "warning"));
    assert!(!has(&report, "command.found", "ok"));
    assert!(report.executable.is_some());
}

#[test]
fn windows_pathext_is_case_insensitive_and_respects_order() {
    let root = tempdir().unwrap();
    file(&root.path().join("SERVER.EXE"));
    file(&root.path().join("server.CMD"));
    let mut env = environment(Platform::Windows, vec![root.path().to_owned()]);
    env.path_ext = extensions(".CMD;.EXE");
    let report = inspect(&config("server"), &env);
    assert!(report
        .executable
        .unwrap()
        .to_ascii_lowercase()
        .ends_with("server.cmd"));
    assert!(has(
        &inspect(&config("server"), &env),
        "command.shell_script",
        "warning"
    ));
    env.path_ext = extensions(".EXE;.CMD");
    assert!(inspect(&config("server"), &env)
        .executable
        .unwrap()
        .to_ascii_lowercase()
        .ends_with("server.exe"));
}

#[test]
fn configured_windows_path_overrides_desktop_path_without_global_mutation() {
    let root = tempdir().unwrap();
    let desktop = root.path().join("desktop");
    let target = root.path().join("target");
    fs::create_dir_all(&desktop).unwrap();
    fs::create_dir_all(&target).unwrap();
    file(&desktop.join("server.exe"));
    file(&target.join("server.CMD"));
    let env = environment(Platform::Windows, vec![desktop]);
    let mut config = config("server");
    config
        .env
        .insert("Path".into(), target.to_string_lossy().into_owned());
    config.env.insert("PathExt".into(), ".CMD".into());
    let report = inspect(&config, &env);
    assert!(report
        .executable
        .as_deref()
        .unwrap()
        .eq_ignore_ascii_case(&target.join("server.CMD").to_string_lossy()));
    assert!(report
        .items
        .iter()
        .any(|item| item.message.contains("配置提供的 PATH")));
}

#[test]
fn relative_command_requires_valid_absolute_working_directory() {
    let root = tempdir().unwrap();
    file(&root.path().join("server"));
    let env = environment(Platform::Linux, vec![]);
    let mut config = config("./server");
    assert!(has(&inspect(&config, &env), "command.relative", "warning"));
    config.cwd = "relative".into();
    assert!(has(&inspect(&config, &env), "cwd.relative", "warning"));
    config.cwd = root.path().join("missing").to_string_lossy().into_owned();
    assert!(has(&inspect(&config, &env), "cwd.missing", "error"));
    config.cwd = root.path().join("server").to_string_lossy().into_owned();
    assert!(has(&inspect(&config, &env), "cwd.not_directory", "error"));
    config.cwd = root.path().to_string_lossy().into_owned();
    assert!(has(&inspect(&config, &env), "command.found", "ok"));
}

#[test]
fn relative_path_entries_are_not_resolved_against_app_directory() {
    let env = environment(Platform::Linux, vec![PathBuf::from("")]);
    let report = inspect(&config("server"), &env);
    assert!(has(&report, "path.relative", "warning"));
    assert!(report.executable.is_none());
}

#[test]
fn windows_drive_relative_commands_and_cwd_are_not_guessed() {
    let root = tempdir().unwrap();
    file(&root.path().join("server.exe"));
    let env = environment(Platform::Windows, vec![root.path().to_owned()]);
    for command in ["C:server.exe", "C:", r"C:tools\server.exe"] {
        let mut config = config(command);
        config.cwd = root.path().to_string_lossy().into_owned();
        let report = inspect(&config, &env);
        assert!(has(&report, "command.relative", "warning"));
        assert!(!has(&report, "command.missing", "error"));
        assert!(report.executable.is_none());
    }
    for directory in ["C:project", "D:", r"C:project\tools"] {
        let mut report = Report::default();
        assert!(inspect_cwd(directory, Platform::Windows, &mut report).is_none());
        assert!(has(&report, "cwd.relative", "warning"));
        assert!(!has(&report, "cwd.missing", "error"));
    }
    assert!(!drive_relative(r"C:\tools\server.exe", Platform::Windows));
    assert!(!drive_relative("C:/tools/server.exe", Platform::Windows));
    assert!(!drive_relative("C:server.exe", Platform::Linux));
}

#[cfg(unix)]
#[test]
fn command_without_execute_bits_is_an_error() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir().unwrap();
    let command = root.path().join("server");
    file(&command);
    fs::set_permissions(&command, fs::Permissions::from_mode(0o600)).unwrap();
    let report = inspect(
        &config("server"),
        &environment(Platform::Linux, vec![root.path().to_owned()]),
    );
    assert!(has(&report, "command.permission", "error"));
    assert!(report.executable.is_none());
}

#[test]
fn script_readability_is_checked_without_echoing_argument_secrets() {
    let root = tempdir().unwrap();
    let command = root.path().join("python3");
    file(&command);
    let mut config = config(command.to_str().unwrap());
    config.cwd = root.path().to_string_lossy().into_owned();
    config.args = vec!["private-script.py".into(), "--token=private-token".into()];
    let env = environment(Platform::Linux, vec![]);
    let report = inspect(&config, &env);
    assert!(has(&report, "script.unreadable", "error"));
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("private-script"));
    assert!(!text.contains("private-token"));
    file(&root.path().join("private-script.py"));
    assert!(has(&inspect(&config, &env), "script.readable", "ok"));
    config.args = vec!["-m".into(), "private-module".into()];
    assert!(has(
        &inspect(&config, &env),
        "dependency.not_checked",
        "warning"
    ));
}

#[test]
fn variable_references_are_reported_without_expansion_or_secret_echo() {
    let mut config = config("server");
    config.env.insert("KEY".into(), "<已隐藏>".into());
    config
        .env
        .insert("TOKEN".into(), "${env:secret-name}".into());
    config.env.insert("PATH".into(), "${PATH}:/private".into());
    config.env.insert("PRIVATE".into(), "hidden-value".into());
    let report = inspect(&config, &environment(Platform::Linux, vec![]));
    assert!(has(&report, "value.missing", "error"));
    assert!(has(&report, "value.variable", "warning"));
    assert!(has(&report, "path.unresolved", "warning"));
    assert!(!has(&report, "command.missing", "error"));
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("secret-name"));
    assert!(!text.contains("hidden-value"));
}

#[test]
fn argument_placeholders_are_errors_but_intentional_empty_values_are_only_warnings() {
    let mut config = config("server");
    config.args = vec![
        "--token=<已隐藏>".into(),
        "${env:private-argument}".into(),
        String::new(),
    ];
    config.env.insert("NO_COLOR".into(), String::new());
    let report = inspect(&config, &environment(Platform::Linux, vec![]));
    assert!(has(&report, "argument.placeholder", "error"));
    assert!(has(&report, "argument.variable", "warning"));
    assert!(has(&report, "value.empty", "warning"));
    assert!(!has(&report, "value.missing", "error"));
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("--token"));
    assert!(!text.contains("private-argument"));
}

#[test]
fn remote_authentication_is_not_tested_and_url_credentials_are_never_echoed() {
    let mut config = config("");
    config.transport = Transport::Http;
    config.url = "https://private-user:private-password@example.test/mcp?key=private-key".into();
    config
        .headers
        .insert("Authorization".into(), "private-header".into());
    let report = inspect(&config, &environment(Platform::Linux, vec![]));
    assert!(has(&report, "config.invalid", "error"));
    assert!(has(&report, "connection.not_checked", "warning"));
    assert!(report.executable.is_none());
    assert!(!serde_json::to_string(&report).unwrap().contains("private-"));
}

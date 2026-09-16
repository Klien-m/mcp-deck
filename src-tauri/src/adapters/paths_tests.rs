use super::*;
use crate::model::{Binding, Config, History, Service, Transport};
use std::collections::BTreeMap;

fn home(platform: Platform) -> PathBuf {
    match platform {
        Platform::Windows => "C:/Users/test-user".into(),
        Platform::MacOs => "/Users/test-user".into(),
        Platform::Linux => "/home/test-user".into(),
    }
}

fn target(targets: &[Target], id: &str) -> PathBuf {
    targets
        .iter()
        .find(|target| target.id == id)
        .unwrap()
        .path
        .clone()
        .into()
}

#[test]
fn each_platform_uses_its_user_config_directory_and_common_home_paths() {
    for (platform, data) in [
        (Platform::MacOs, "Library/Application Support"),
        (Platform::Windows, "AppData/Roaming"),
        (Platform::Linux, ".config"),
    ] {
        let home = home(platform);
        let targets = resolve_targets(&home, platform, &PathEnvironment::default(), |_| false);
        assert_eq!(registry().len(), 12);
        assert_eq!(
            targets.len(),
            if platform == Platform::Linux { 11 } else { 12 }
        );
        assert!(targets
            .iter()
            .all(|target| platform.absolute(Path::new(&target.path))));
        assert_eq!(
            target(&targets, "vscode"),
            home.join(data).join("Code/User/mcp.json")
        );
        assert_eq!(
            target(&targets, "roo"),
            home.join(data).join(
                "Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json"
            )
        );
        if platform == Platform::Linux {
            assert!(!targets.iter().any(|target| target.id == "claude-desktop"));
            assert!(super::super::get("claude-desktop").is_ok());
        } else {
            assert_eq!(
                target(&targets, "claude-desktop"),
                home.join(data).join("Claude/claude_desktop_config.json")
            );
        }
        for (id, path) in [
            ("codex", ".codex/config.toml"),
            ("claude", ".claude.json"),
            ("cursor", ".cursor/mcp.json"),
            ("gemini", ".gemini/settings.json"),
            ("opencode", ".config/opencode/opencode.jsonc"),
            ("copilot", ".copilot/mcp-config.json"),
            ("windsurf", ".codeium/windsurf/mcp_config.json"),
            ("kiro", ".kiro/settings/mcp.json"),
            ("cline", ".cline/data/settings/cline_mcp_settings.json"),
        ] {
            assert_eq!(
                target(&targets, id),
                home.join(path),
                "{id} on {platform:?}"
            );
        }
    }
}

#[test]
fn windows_appdata_and_cross_platform_tool_overrides_take_precedence() {
    for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
        let home = home(platform);
        let external = if platform == Platform::Windows {
            PathBuf::from("D:/User settings")
        } else {
            PathBuf::from("/external/settings")
        };
        let environment = PathEnvironment {
            app_data: Some(external.join("roaming")),
            xdg_config_home: Some(external.join("xdg")),
            codex_home: Some(external.join("codex")),
            isolated: false,
        };
        let targets = resolve_targets(&home, platform, &environment, |_| false);
        assert_eq!(
            target(&targets, "codex"),
            external.join("codex/config.toml")
        );
        assert_eq!(
            target(&targets, "opencode"),
            external.join("xdg/opencode/opencode.jsonc")
        );
        let vscode = match platform {
            Platform::Windows => external.join("roaming/Code/User/mcp.json"),
            Platform::Linux => external.join("xdg/Code/User/mcp.json"),
            Platform::MacOs => home.join("Library/Application Support/Code/User/mcp.json"),
        };
        assert_eq!(target(&targets, "vscode"), vscode);
        if platform == Platform::Windows {
            assert_eq!(
                target(&targets, "claude-desktop"),
                external.join("roaming/Claude/claude_desktop_config.json")
            );
        }
    }
}

#[test]
fn existing_jsonc_or_json_is_selected_inside_xdg_override_before_fallback() {
    let home = home(Platform::Linux);
    let directory = PathBuf::from("/other/xdg/opencode");
    let environment = PathEnvironment {
        xdg_config_home: Some("/other/xdg".into()),
        ..Default::default()
    };
    let json = directory.join("opencode.json");
    let jsonc = directory.join("opencode.jsonc");
    let only_json = resolve_targets(&home, Platform::Linux, &environment, |path| path == json);
    assert_eq!(target(&only_json, "opencode"), json);
    let both = resolve_targets(&home, Platform::Linux, &environment, |path| {
        path == json || path == jsonc
    });
    assert_eq!(target(&both, "opencode"), jsonc);
    let old_home_file = home.join(".config/opencode/opencode.json");
    let old_only = resolve_targets(&home, Platform::Linux, &environment, |path| {
        path == old_home_file
    });
    assert_eq!(
        target(&old_only, "opencode"),
        jsonc,
        "XDG override must not silently use an unrelated default file"
    );
}

#[test]
fn linux_desktop_discovers_existing_config_but_does_not_invent_a_new_file() {
    let home = home(Platform::Linux);
    for directory in [home.join(".config"), PathBuf::from("/custom/xdg")] {
        let environment = PathEnvironment {
            xdg_config_home: Some(directory.clone()),
            ..Default::default()
        };
        let config = directory.join("Claude/claude_desktop_config.json");
        let targets = resolve_targets(&home, Platform::Linux, &environment, |path| path == config);
        assert_eq!(target(&targets, "claude-desktop"), config);
        let absent = resolve_targets(&home, Platform::Linux, &environment, |_| false);
        assert!(!absent.iter().any(|target| target.id == "claude-desktop"));
    }
}

#[test]
fn cline_preserves_existing_legacy_files_but_prefers_the_existing_shared_file() {
    for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
        let home = home(platform);
        let environment = PathEnvironment::default();
        let extension = environment.app_data_dir(&home, platform).join(
            "Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json",
        );
        let cli = home.join(".cline/mcp.json");
        let shared = home.join(".cline/data/settings/cline_mcp_settings.json");
        for existing in [&extension, &cli, &shared] {
            let targets = resolve_targets(&home, platform, &environment, |path| path == existing);
            assert_eq!(&target(&targets, "cline"), existing);
        }
        let all = resolve_targets(&home, platform, &environment, |_| true);
        assert_eq!(target(&all, "cline"), shared);
    }
}

#[test]
fn relative_empty_and_parent_traversing_environment_directories_are_ignored() {
    for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
        let home = home(platform);
        let baseline = resolve_targets(&home, platform, &PathEnvironment::default(), |_| false);
        for value in [
            "",
            "relative/path",
            "~/custom",
            "$HOME/config",
            "C:relative",
            "/tmp/../other",
            "C:/settings/../other",
        ] {
            // C:/... is a relative POSIX path; /tmp/... is a drive-relative Windows path.
            let environment = PathEnvironment {
                app_data: Some(value.into()),
                xdg_config_home: Some(value.into()),
                codex_home: Some(value.into()),
                isolated: false,
            };
            let actual = resolve_targets(&home, platform, &environment, |_| false);
            for id in ["vscode", "codex", "opencode"] {
                assert_eq!(
                    target(&actual, id),
                    target(&baseline, id),
                    "{value:?} on {platform:?}"
                );
            }
        }
    }
    assert!(Platform::Windows.absolute(Path::new(r"\\server\share\config")));
    assert!(!Platform::Windows.absolute(Path::new(r"\config")));
    assert!(!Platform::Windows.absolute(Path::new(r"\\server")));
}

#[test]
fn isolated_homes_ignore_every_external_directory_override() {
    for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
        let home = home(platform);
        let external = if platform == Platform::Windows {
            "D:/real/user"
        } else {
            "/real/user"
        };
        let environment = PathEnvironment {
            app_data: Some(external.into()),
            xdg_config_home: Some(external.into()),
            codex_home: Some(external.into()),
            isolated: true,
        };
        let targets = resolve_targets(&home, platform, &environment, |_| false);
        assert!(targets
            .iter()
            .all(|target| Path::new(&target.path).starts_with(&home)));
    }
    let temp = tempfile::tempdir().unwrap();
    let temporary_home = temp.path().join("fixture-home");
    assert!(PathEnvironment::read(&temporary_home).isolated);
    assert!(default_targets(&temporary_home)
        .iter()
        .all(|target| Path::new(&target.path).starts_with(&temporary_home)));
}

fn workspace(platform: Platform) -> Workspace {
    Workspace {
        version: 1,
        revision: 5,
        onboarding_complete: true,
        services: vec![],
        history: vec![],
        targets: resolve_targets(
            &home(platform),
            Platform::MacOs,
            &PathEnvironment::default(),
            |_| false,
        ),
    }
}

fn service() -> Service {
    Service {
        id: "test-service".into(),
        key: "test".into(),
        name: "test".into(),
        description: String::new(),
        config: Config {
            transport: Transport::Stdio,
            command: "node".into(),
            args: vec![],
            env: BTreeMap::new(),
            cwd: String::new(),
            url: String::new(),
            headers: BTreeMap::new(),
        },
        targets: vec![],
        bindings: BTreeMap::new(),
        native: BTreeMap::new(),
        deleted: false,
        deleted_targets: vec![],
    }
}

#[test]
fn unused_missing_old_defaults_migrate_without_changing_stable_ids() {
    for platform in [Platform::Windows, Platform::Linux] {
        let home = home(platform);
        let mut workspace = workspace(platform);
        let environment = PathEnvironment::default();
        assert!(migrate_targets(
            &mut workspace,
            &home,
            platform,
            &environment,
            |_| false,
            |_| false
        ));
        let expected = resolve_targets(&home, platform, &environment, |_| false);
        assert_eq!(workspace.targets.len(), expected.len());
        for target in &workspace.targets {
            let expected = expected.iter().find(|item| item.id == target.id).unwrap();
            assert_eq!(target.path, expected.path);
            assert_eq!(target.adapter_id, expected.adapter_id);
        }
        assert!(!migrate_targets(
            &mut workspace,
            &home,
            platform,
            &environment,
            |_| false,
            |_| false
        ));
    }
}

#[test]
fn linux_desktop_migration_uses_an_existing_xdg_file_only_when_unreferenced() {
    let home = home(Platform::Linux);
    let environment = PathEnvironment {
        xdg_config_home: Some("/custom/xdg".into()),
        ..Default::default()
    };
    let new_path = PathBuf::from("/custom/xdg/Claude/claude_desktop_config.json");
    for linked in [false, true] {
        let mut workspace = workspace(Platform::Linux);
        let before = target(&workspace.targets, "claude-desktop");
        if linked {
            let mut service = service();
            service
                .bindings
                .insert("claude-desktop".into(), Binding { raw: None });
            workspace.services.push(service);
        }
        migrate_targets(
            &mut workspace,
            &home,
            Platform::Linux,
            &environment,
            |_| false,
            |path| path == new_path,
        );
        assert_eq!(
            target(&workspace.targets, "claude-desktop"),
            if linked { before } else { new_path.clone() }
        );
    }
}

#[test]
fn migration_preserves_assignments_baselines_deleted_assignments_and_history() {
    for reference in [
        "assignment",
        "binding",
        "deleted",
        "history",
        "history_alias",
    ] {
        let home = home(Platform::Windows);
        let mut workspace = workspace(Platform::Windows);
        let before = target(&workspace.targets, "vscode");
        let mut service = service();
        match reference {
            "assignment" => service.targets.push("vscode".into()),
            "binding" => {
                service
                    .bindings
                    .insert("vscode".into(), Binding { raw: None });
            }
            "deleted" => service.deleted_targets.push("vscode".into()),
            "history" | "history_alias" => workspace.history.push(History {
                id: "history".into(),
                at: 0,
                status: "applied".into(),
                summary: String::new(),
                paths: vec![if reference == "history_alias" {
                    before.to_string_lossy().replace('/', "\\").to_lowercase()
                } else {
                    before.to_string_lossy().into()
                }],
                count: 1,
            }),
            _ => unreachable!(),
        }
        workspace.services.push(service);
        migrate_targets(
            &mut workspace,
            &home,
            Platform::Windows,
            &PathEnvironment::default(),
            |_| false,
            |_| false,
        );
        assert_eq!(target(&workspace.targets, "vscode"), before, "{reference}");
        assert!(legacy_mac_path(
            workspace
                .targets
                .iter()
                .find(|target| target.id == "vscode")
                .unwrap(),
            &home,
            Platform::Windows
        ));
    }
}

#[test]
fn migration_does_not_move_existing_custom_or_duplicate_targets() {
    for preserve in ["file", "name", "id", "path", "duplicate", "duplicate_alias"] {
        let home = home(Platform::Windows);
        let mut workspace = workspace(Platform::Windows);
        let vscode = workspace
            .targets
            .iter_mut()
            .find(|target| target.id == "vscode")
            .unwrap();
        match preserve {
            "name" => vscode.name = "My VS Code".into(),
            "id" => vscode.id = "custom-vscode".into(),
            "path" => vscode.path = home.join("custom/mcp.json").to_string_lossy().into(),
            _ => (),
        }
        let id = vscode.id.clone();
        let before = PathBuf::from(&vscode.path);
        if preserve == "duplicate" || preserve == "duplicate_alias" {
            let path = home
                .join("AppData/Roaming/Code/User/mcp.json")
                .to_string_lossy()
                .into_owned();
            workspace.targets.push(Target {
                id: "manual".into(),
                adapter_id: "vscode".into(),
                name: "manual".into(),
                path: if preserve == "duplicate_alias" {
                    path.replace('/', "\\").to_lowercase()
                } else {
                    path
                },
            });
        }
        migrate_targets(
            &mut workspace,
            &home,
            Platform::Windows,
            &PathEnvironment::default(),
            |path| preserve == "file" && path == before,
            |_| false,
        );
        assert_eq!(target(&workspace.targets, &id), before, "{preserve}");
    }
}

#[test]
fn legacy_notice_matches_only_known_default_id_and_exact_path_on_other_platforms() {
    let home = home(Platform::Linux);
    let workspace = workspace(Platform::Linux);
    let mut vscode = workspace
        .targets
        .into_iter()
        .find(|target| target.id == "vscode")
        .unwrap();
    assert!(legacy_mac_path(&vscode, &home, Platform::Linux));
    assert!(!legacy_mac_path(&vscode, &home, Platform::MacOs));
    vscode.id = "user-added".into();
    assert!(!legacy_mac_path(&vscode, &home, Platform::Linux));
    vscode.id = "vscode".into();
    vscode.path = home.join("custom/missing.json").to_string_lossy().into();
    assert!(!legacy_mac_path(&vscode, &home, Platform::Linux));
}

#[test]
fn reopening_an_old_workspace_persists_only_the_unused_target_path_migration() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let home = root.join("fixture-home");
    let data = root.join("fixture-data");
    let mut engine = crate::engine::Engine::open(home.clone(), data.clone()).unwrap();
    let current = target(&engine.workspace.targets, "cline");
    let legacy = home.join(".cline/mcp.json");
    engine
        .workspace
        .targets
        .iter_mut()
        .find(|target| target.id == "cline")
        .unwrap()
        .path = legacy.to_string_lossy().into();
    let revision = engine.workspace.revision;
    crate::storage::atomic_write(
        &data.join("workspace.json"),
        &serde_json::to_string(&engine.workspace).unwrap(),
        true,
    )
    .unwrap();
    drop(engine);

    let engine = crate::engine::Engine::open(home.clone(), data.clone()).unwrap();
    assert_eq!(target(&engine.workspace.targets, "cline"), current);
    assert_eq!(engine.workspace.revision, revision + 1);
    assert!(!legacy.exists());
    assert!(!current.exists());
    assert!(
        !home.exists(),
        "migration must not create any Agent configuration directory"
    );
    drop(engine);
    let reopened = crate::engine::Engine::open(home, data).unwrap();
    assert_eq!(
        reopened.workspace.revision,
        revision + 1,
        "migration must be idempotent"
    );
}

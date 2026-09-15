use mcp_deck::{
    commands::{dispatch, Request},
    engine::{Adoption, Engine},
    storage,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

fn setup() -> (tempfile::TempDir, Engine) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let home = root.join("home");
    fs::create_dir(&home).unwrap();
    let engine = Engine::open(home, root.join("data")).unwrap();
    (temp, engine)
}

fn target_path(engine: &Engine, id: &str) -> PathBuf {
    engine
        .workspace
        .targets
        .iter()
        .find(|t| t.id == id)
        .unwrap()
        .path
        .clone()
        .into()
}

fn put(engine: &Engine, id: &str, text: &str) {
    storage::atomic_write(&target_path(engine, id), text, false).unwrap();
}

fn select(id: &str, keys: &[&str]) -> Adoption {
    Adoption {
        target_id: id.into(),
        keys: keys.iter().map(|key| (*key).into()).collect(),
    }
}

fn reopen(engine: Engine) -> Engine {
    let home = engine.home.clone();
    let data = engine.data_dir.clone();
    drop(engine);
    Engine::open(home, data).unwrap()
}

#[test]
fn first_launch_stays_pending_until_explicit_completion_and_skip_survives_restart() {
    let (_temp, engine) = setup();
    assert!(!engine.workspace.onboarding_complete);
    assert!(engine.discover_all().is_empty());
    let mut engine = reopen(engine);
    assert!(!engine.workspace.onboarding_complete);
    engine.complete_onboarding(vec![]).unwrap();
    let engine = reopen(engine);
    assert!(engine.workspace.onboarding_complete);
    assert!(engine.workspace.services.is_empty());
    assert!(engine
        .workspace
        .targets
        .iter()
        .all(|t| !PathBuf::from(&t.path).exists()));
}

#[test]
fn old_workspaces_without_onboarding_state_do_not_show_first_launch_again() {
    let (_temp, engine) = setup();
    let path = engine.data_dir.join("workspace.json");
    let mut state = serde_json::to_value(&engine.workspace).unwrap();
    state.as_object_mut().unwrap().remove("onboardingComplete");
    storage::atomic_write(&path, &state.to_string(), true).unwrap();
    let engine = reopen(engine);
    assert!(engine.workspace.onboarding_complete);
    let saved: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved["onboardingComplete"], true);
}

#[test]
fn scan_preserves_errors_and_unsupported_items_without_changing_sources_or_state() {
    let (_temp, mut engine) = setup();
    put(&engine, "codex", "[mcp_servers.shared]\ncommand = 'node'\n");
    put(
        &engine,
        "cursor",
        r#"{"mcpServers":{"shared":{"command":"node"},"unsupported":{"type":"future"}}}"#,
    );
    put(&engine, "claude", r#"{"mcpServers":broken}"#);
    put(&engine, "gemini", r#"{"theme":"keep"}"#);
    engine.adopt("codex", vec!["shared".into()]).unwrap();
    let before = fs::read(engine.data_dir.join("workspace.json")).unwrap();
    let sources: Vec<_> = ["codex", "cursor", "claude", "gemini"]
        .iter()
        .map(|id| {
            (
                target_path(&engine, id),
                fs::read(target_path(&engine, id)).unwrap(),
            )
        })
        .collect();
    let results = engine.discover_all();
    assert_eq!(results.len(), 3);
    assert!(results
        .iter()
        .find(|r| r.target.id == "claude")
        .unwrap()
        .error
        .is_some());
    assert!(
        results
            .iter()
            .find(|r| r.target.id == "codex")
            .unwrap()
            .items[0]
            .managed
    );
    let cursor = results.iter().find(|r| r.target.id == "cursor").unwrap();
    assert!(cursor
        .items
        .iter()
        .find(|i| i.key == "shared")
        .unwrap()
        .config
        .is_some());
    let unsupported = cursor
        .items
        .iter()
        .find(|i| i.key == "unsupported")
        .unwrap();
    assert!(unsupported.error.is_some());
    assert!(unsupported.config.is_none());
    assert_eq!(
        before,
        fs::read(engine.data_dir.join("workspace.json")).unwrap()
    );
    for (path, contents) in sources {
        assert_eq!(contents, fs::read(path).unwrap());
    }
}

#[test]
fn adoption_keeps_same_named_services_separate_and_only_imports_selected_keys() {
    let (_temp, mut engine) = setup();
    let codex = "# keep\nmodel='keep'\n[mcp_servers.shared]\ncommand='node'\nenabled=false\n";
    let cursor = r#"{"other":true,"mcpServers":{"shared":{"command":"node","disabled":true},"leave":{"command":"uvx"}}}"#;
    put(&engine, "codex", codex);
    put(&engine, "cursor", cursor);
    engine
        .complete_onboarding(vec![
            select("codex", &["shared"]),
            select("cursor", &["shared"]),
        ])
        .unwrap();
    assert_eq!(engine.workspace.revision, 1);
    let mut engine = reopen(engine);
    assert!(engine.workspace.onboarding_complete);
    assert_eq!(engine.workspace.services.len(), 2);
    assert_ne!(
        engine.workspace.services[0].id,
        engine.workspace.services[1].id
    );
    assert_eq!(engine.workspace.services[0].targets, ["codex"]);
    assert_eq!(engine.workspace.services[1].targets, ["cursor"]);
    assert_eq!(
        engine.workspace.services[1].bindings["cursor"]
            .raw
            .as_ref()
            .unwrap()["disabled"],
        true
    );
    assert!(engine.preview().unwrap().changes.is_empty());
    assert_eq!(
        fs::read_to_string(target_path(&engine, "codex")).unwrap(),
        codex
    );
    assert_eq!(
        fs::read_to_string(target_path(&engine, "cursor")).unwrap(),
        cursor
    );
}

#[test]
fn a_failed_later_tool_rolls_back_the_whole_batch_and_can_be_retried() {
    let (_temp, mut engine) = setup();
    put(&engine, "codex", "[mcp_servers.first]\ncommand='node'\n");
    put(
        &engine,
        "cursor",
        r#"{"mcpServers":{"second":{"command":"node"}}}"#,
    );
    assert_eq!(engine.discover_all().len(), 2);
    // 模拟用户选择完成后，另一个程序从第二个目标移除了该配置。
    put(&engine, "cursor", r#"{"mcpServers":{}}"#);
    let before = fs::read(engine.data_dir.join("workspace.json")).unwrap();
    let result = engine.complete_onboarding(vec![
        select("codex", &["first"]),
        select("cursor", &["second"]),
    ]);
    assert!(result.unwrap_err().contains("Cursor"));
    assert!(engine.workspace.services.is_empty());
    assert!(!engine.workspace.onboarding_complete);
    assert_eq!(
        before,
        fs::read(engine.data_dir.join("workspace.json")).unwrap()
    );
    let mut engine = reopen(engine);
    assert!(!engine.workspace.onboarding_complete);
    put(
        &engine,
        "cursor",
        r#"{"mcpServers":{"second":{"command":"node"}}}"#,
    );
    engine
        .complete_onboarding(vec![
            select("codex", &["first"]),
            select("cursor", &["second"]),
        ])
        .unwrap();
    assert_eq!(engine.workspace.services.len(), 2);
}

#[test]
fn unsupported_or_already_managed_selections_cannot_partially_complete_onboarding() {
    let (_temp, mut engine) = setup();
    put(
        &engine,
        "cursor",
        r#"{"mcpServers":{"valid":{"command":"node"},"bad":{"type":"future"}}}"#,
    );
    assert!(engine
        .complete_onboarding(vec![select("cursor", &["valid", "bad"])])
        .is_err());
    assert!(engine.workspace.services.is_empty());
    assert!(!engine.workspace.onboarding_complete);
    engine.adopt("cursor", vec!["valid".into()]).unwrap();
    assert!(engine
        .complete_onboarding(vec![select("cursor", &["valid"])])
        .is_err());
    assert_eq!(engine.workspace.services.len(), 1);
    assert!(!engine.workspace.onboarding_complete);
}

#[test]
fn onboarding_commands_use_camel_case_and_return_serializable_results() {
    let (_temp, mut engine) = setup();
    put(&engine, "codex", "[mcp_servers.test]\ncommand='node'\n");
    let request = |value| serde_json::from_value::<Request>(value).unwrap();
    let scan = dispatch(&mut engine, request(json!({"op":"discoverAll"}))).unwrap();
    assert_eq!(scan[0]["target"]["adapterId"], "codex");
    assert_eq!(scan[0]["items"][0]["key"], "test");
    assert_eq!(dispatch(&mut engine, request(json!({"op":"completeOnboarding","selections":[{"targetId":"codex","keys":["test"]}]}))).unwrap(), Value::Null);
    let snapshot = dispatch(&mut engine, request(json!({"op":"snapshot"}))).unwrap();
    assert_eq!(snapshot["workspace"]["onboardingComplete"], true);
    for bad in [
        json!({"op":"completeOnboarding"}),
        json!({"op":"completeOnboarding","selections":[{"target_id":"codex","keys":["test"]}]}),
        json!({"op":"completeOnboarding","selections":[{"targetId":"codex","keys":"test"}]}),
    ] {
        assert!(serde_json::from_value::<Request>(bad).is_err());
    }
}

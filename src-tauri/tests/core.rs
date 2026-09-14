use mcp_deck::{
    adapters,
    engine::{Engine, FileEdit, Journal},
    model::*,
    storage,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn stdio() -> Config {
    Config {
        transport: Transport::Stdio,
        command: "node".into(),
        args: vec!["".into(), "two\nlines".into(), "/space in/path.js".into()],
        env: BTreeMap::from([("TOKEN".into(), "line1\nline2".into())]),
        cwd: String::new(),
        url: String::new(),
        headers: BTreeMap::new(),
    }
}
fn http() -> Config {
    Config {
        transport: Transport::Http,
        command: String::new(),
        args: vec![],
        cwd: String::new(),
        env: BTreeMap::new(),
        url: "https://example.com/mcp".into(),
        headers: BTreeMap::from([("Authorization".into(), "Bearer example-secret".into())]),
    }
}
fn setup() -> (tempfile::TempDir, Engine) {
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let root = temp.path().canonicalize().unwrap();
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let engine = Engine::open(home, root.join("data")).unwrap();
    (temp, engine)
}
fn path(engine: &Engine, target: &str) -> PathBuf {
    engine
        .workspace
        .targets
        .iter()
        .find(|t| t.id == target)
        .unwrap()
        .path
        .clone()
        .into()
}
fn add(engine: &mut Engine, key: &str) -> String {
    engine
        .save_service(ServiceInput {
            id: None,
            key: key.into(),
            name: key.into(),
            description: "test".into(),
            config: stdio(),
        })
        .unwrap()
}
fn put(engine: &Engine, target: &str, text: &str) {
    storage::atomic_write(&path(engine, target), text, false).unwrap();
}

#[test]
fn all_twelve_adapters_roundtrip_supported_transports_and_preserve_other_sections() {
    let registry = adapters::registry();
    assert_eq!(registry.len(), 12);
    for adapter in registry {
        for transport in &adapter.transports {
            let mut config = if *transport == Transport::Stdio {
                stdio()
            } else {
                http()
            };
            config.transport = transport.clone();
            let raw = adapters::encode(&adapter, &config, None).unwrap();
            let original = if adapter.id == "codex" {
                "# keep exact comment\nmodel = 'do-not-touch'\n"
            } else {
                "{\n  // keep exact comment\n  \"unrelated\" : { \"keep\" : true }\n}\n"
            };
            let output = adapters::patch(
                &adapter,
                Some(original),
                &BTreeMap::from([("a.b with space".into(), Some(raw.clone()))]),
            )
            .unwrap();
            assert!(output.contains("keep exact comment"));
            assert!(output.contains(if adapter.id == "codex" {
                "model = 'do-not-touch'"
            } else {
                "\"unrelated\" : { \"keep\" : true }"
            }));
            let parsed = adapters::parse(&adapter, &output).unwrap();
            assert_eq!(
                adapters::decode(&adapter, &parsed["a.b with space"]).unwrap(),
                config,
                "{}",
                adapter.id
            );
            let removed = adapters::patch(
                &adapter,
                Some(&output),
                &BTreeMap::from([("a.b with space".into(), None)]),
            )
            .unwrap();
            assert!(adapters::parse(&adapter, &removed).unwrap().is_empty());
        }
    }
}

#[test]
fn native_fields_are_preserved_and_readonly_shapes_are_rejected() {
    let adapter = adapters::get("cline").unwrap();
    let raw =
        json!({"command":"node","args":[],"autoApprove":["list"],"disabled":true,"timeout":60});
    let mut config = adapters::decode(&adapter, &raw).unwrap();
    assert_eq!(
        adapters::encode(&adapter, &config, Some(&raw)).unwrap(),
        raw
    );
    config.command = "python3".into();
    let updated = adapters::encode(&adapter, &config, Some(&raw)).unwrap();
    assert_eq!(updated["autoApprove"], raw["autoApprove"]);
    assert_eq!(updated["disabled"], true);
    assert!(adapters::decode(
        &adapter,
        &json!({"url":"https://example.com/mcp","type":"unsupported"})
    )
    .is_err());
    assert!(adapters::decode(&adapter, &json!({"command":"node","env":{"PORT":123}})).is_err());
    assert!(adapters::encode(&adapters::get("claude-desktop").unwrap(), &http(), None).is_err());
    assert!(adapters::parse(&adapter, "{\"mcpServers\": []}").is_err());
}

#[test]
fn apply_persists_and_rollback_restores_exact_original_text() {
    let (_temp, mut engine) = setup();
    let original="{\n  // untouched\n  \"settings\": {\"extra\":42},\n  \"mcpServers\": {\"unmanaged\":{\"command\":\"uvx\"}}\n}\n";
    put(&engine, "claude", original);
    let sid = add(&mut engine, "managed");
    engine.assign(&sid, "claude", true).unwrap();
    engine.assign(&sid, "codex", true).unwrap();
    let preview = engine.preview().unwrap();
    assert_eq!(preview.changes.len(), 2);
    assert!(preview.errors.is_empty());
    engine.apply(&preview.id).unwrap();
    let after = fs::read_to_string(path(&engine, "claude")).unwrap();
    assert!(after.contains("// untouched"));
    assert!(after.contains("\"settings\": {\"extra\":42}"));
    assert!(after.contains("\"unmanaged\":{\"command\":\"uvx\"}"));
    assert!(engine.preview().unwrap().changes.is_empty());
    let home = engine.home.clone();
    let data = engine.data_dir.clone();
    drop(engine);
    let mut engine = Engine::open(home, data).unwrap();
    assert_eq!(engine.workspace.services.len(), 1);
    assert!(engine.preview().unwrap().changes.is_empty());
    engine.rollback(&preview.id).unwrap();
    assert_eq!(
        fs::read_to_string(path(&engine, "claude")).unwrap(),
        original
    );
    assert!(!path(&engine, "codex").exists());
    assert_eq!(engine.preview().unwrap().changes.len(), 2);
}

#[test]
fn preview_hash_blocks_late_edits_without_writing_any_target() {
    let (_temp, mut engine) = setup();
    let sid = add(&mut engine, "new");
    engine.assign(&sid, "codex", true).unwrap();
    engine.assign(&sid, "cursor", true).unwrap();
    let preview = engine.preview().unwrap();
    put(&engine, "cursor", "{\"mcpServers\":{},\"late\":true}");
    assert!(engine.apply(&preview.id).is_err());
    assert!(!path(&engine, "codex").exists());
    assert!(fs::read_to_string(path(&engine, "cursor"))
        .unwrap()
        .contains("late"));
}

#[test]
fn explicit_adoption_is_noop_and_external_conflict_can_be_resolved() {
    let (_temp, mut engine) = setup();
    let original = "{\"mcpServers\":{\"known\":{\"command\":\"node\",\"custom\":42}}}";
    put(&engine, "cursor", original);
    engine.adopt("cursor", vec!["known".into()]).unwrap();
    assert!(engine.preview().unwrap().changes.is_empty());
    assert_eq!(
        fs::read_to_string(path(&engine, "cursor")).unwrap(),
        original
    );
    let sid = engine.workspace.services[0].id.clone();
    put(
        &engine,
        "cursor",
        "{\"mcpServers\":{\"known\":{\"command\":\"python3\",\"custom\":43}}}",
    );
    let p = engine.preview().unwrap();
    assert!(p.changes[0].conflict);
    assert!(engine.apply(&p.id).is_err());
    engine.resolve(&sid, "cursor", true).unwrap();
    assert_eq!(engine.workspace.services[0].config.command, "python3");
    assert!(engine.preview().unwrap().changes.is_empty());
}

#[test]
fn unmanaged_collision_requires_explicit_choice() {
    let (_temp, mut engine) = setup();
    put(
        &engine,
        "cursor",
        "{\"mcpServers\":{\"same\":{\"command\":\"python3\"}}}",
    );
    let sid = add(&mut engine, "same");
    engine.assign(&sid, "cursor", true).unwrap();
    let p = engine.preview().unwrap();
    assert!(p.changes[0].conflict);
    assert!(engine.apply(&p.id).is_err());
    engine.resolve(&sid, "cursor", false).unwrap();
    let p = engine.preview().unwrap();
    assert!(!p.changes[0].conflict);
    engine.apply(&p.id).unwrap();
}

#[test]
fn atomic_import_and_delete_undo() {
    let (_temp, mut engine) = setup();
    assert!(engine
        .import_text(
            "claude",
            "{\"mcpServers\":{\"good\":{\"command\":\"node\"},\"bad\":{\"command\":7}}}"
        )
        .is_err());
    assert!(engine.workspace.services.is_empty());
    let sid = add(&mut engine, "a");
    engine.assign(&sid, "cursor", true).unwrap();
    let p = engine.preview().unwrap();
    engine.apply(&p.id).unwrap();
    engine.remove(&sid, false).unwrap();
    assert_eq!(engine.preview().unwrap().changes[0].action, "remove");
    engine.remove(&sid, true).unwrap();
    assert!(engine.preview().unwrap().changes.is_empty());
    engine.remove(&sid, false).unwrap();
    let p = engine.preview().unwrap();
    engine.apply(&p.id).unwrap();
    assert!(adapters::parse(
        &adapters::get("cursor").unwrap(),
        &fs::read_to_string(path(&engine, "cursor")).unwrap()
    )
    .unwrap()
    .is_empty());
}

#[test]
fn crash_journal_recovers_and_preserves_external_changes() {
    let (_temp, mut engine) = setup();
    let target = path(&engine, "cursor");
    let after = "{\"mcpServers\":{}}";
    put(&engine, "cursor", after);
    let journal = Journal {
        id: id(),
        status: "prepared".into(),
        files: vec![FileEdit {
            target_id: "cursor".into(),
            path: target.to_string_lossy().into(),
            before: None,
            after: Some(after.into()),
        }],
        count: 1,
    };
    let journal_path = engine
        .data_dir
        .join("backups")
        .join(format!("{}.json", journal.id));
    storage::atomic_write(
        &journal_path,
        &serde_json::to_string(&journal).unwrap(),
        true,
    )
    .unwrap();
    let home = engine.home.clone();
    let data = engine.data_dir.clone();
    drop(engine);
    engine = Engine::open(home, data).unwrap();
    assert!(!target.exists());
    assert_eq!(engine.workspace.history[0].status, "recovered");
    let mut journal = journal;
    journal.id = id();
    storage::atomic_write(
        &engine
            .data_dir
            .join("backups")
            .join(format!("{}.json", journal.id)),
        &serde_json::to_string(&journal).unwrap(),
        true,
    )
    .unwrap();
    put(&engine, "cursor", "{\"external\":true}");
    let home = engine.home.clone();
    let data = engine.data_dir.clone();
    drop(engine);
    let mut engine = Engine::open(home, data).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "{\"external\":true}");
    assert_eq!(
        engine.workspace.history.last().unwrap().status,
        "recovery-needed"
    );
    assert!(!engine.preview().unwrap().errors.is_empty());
    engine.keep_recovery(&journal.id).unwrap();
    assert!(engine.preview().unwrap().errors.is_empty());
    assert_eq!(fs::read_to_string(&target).unwrap(), "{\"external\":true}");
    assert_eq!(
        engine.workspace.history.last().unwrap().status,
        "recovery-kept"
    );
}

#[test]
fn rollback_never_overwrites_later_changes_and_workspace_lock_is_exclusive() {
    let (_temp, mut engine) = setup();
    assert!(Engine::open(engine.home.clone(), engine.data_dir.clone()).is_err());
    let sid = add(&mut engine, "a");
    engine.assign(&sid, "cursor", true).unwrap();
    let p = engine.preview().unwrap();
    engine.apply(&p.id).unwrap();
    put(&engine, "cursor", "{\"external\":true}");
    assert!(engine.rollback(&p.id).is_err());
    assert!(fs::read_to_string(path(&engine, "cursor"))
        .unwrap()
        .contains("external"));
}

#[test]
fn redaction_preserves_structural_types() {
    let value = json!({"command":["npx","secret"],"args":["secret"],"env":{"TOKEN":"abc"},"headers":{"Auth":"secret"}});
    let masked = redact(&value);
    assert!(masked["command"].is_array());
    assert!(masked["args"].is_array());
    assert!(!masked.to_string().contains("secret"));
    assert!(!masked.to_string().contains("abc"));
}

#[test]
fn target_paths_cannot_alias_workspace_or_another_target() {
    let (_temp, mut engine) = setup();
    let mut target = engine.workspace.targets[0].clone();
    target.path = engine
        .data_dir
        .join("workspace.json")
        .to_string_lossy()
        .into();
    assert!(engine.save_target(target).is_err());
    let mut target = engine.workspace.targets[0].clone();
    target.path = path(&engine, "cursor").to_string_lossy().into();
    assert!(engine.save_target(target).is_err());
    assert!(storage::validate_path(Path::new("/tmp/../other/config.json")).is_err());
}

#[test]
fn duplicate_json_keys_are_rejected_including_escaped_names() {
    let adapter = adapters::get("cursor").unwrap();
    assert!(adapters::parse(
        &adapter,
        r#"{"mcpServers":{"a":{"command":"node"},"\u0061":{"command":"python"}}}"#
    )
    .is_err());
    assert!(adapters::parse(&adapter, r#"{"mcpServers":{},"mcpServers":{}}"#).is_err());
}

#[cfg(unix)]
#[test]
fn second_file_write_failure_restores_first_file() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, mut engine) = setup();
    let sid = add(&mut engine, "a");
    put(&engine, "cursor", "{\"mcpServers\":{}}");
    engine.assign(&sid, "codex", true).unwrap();
    engine.assign(&sid, "cursor", true).unwrap();
    let p = engine.preview().unwrap();
    let target = path(&engine, "cursor");
    let parent = target.parent().unwrap();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();
    let result = engine.apply(&p.id);
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert!(!path(&engine, "codex").exists());
    assert_eq!(fs::read_to_string(target).unwrap(), "{\"mcpServers\":{}}");
    assert_eq!(engine.workspace.history.last().unwrap().status, "recovered");
}

#[test]
fn remote_dialects_match_documented_client_examples() {
    for (adapter_id, expected) in [
        (
            "claude",
            json!({"type":"http","url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "cursor",
            json!({"url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "kiro",
            json!({"url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "gemini",
            json!({"httpUrl":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "opencode",
            json!({"type":"remote","url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "windsurf",
            json!({"serverUrl":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "cline",
            json!({"type":"streamableHttp","url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "roo",
            json!({"type":"streamable-http","url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "codex",
            json!({"url":"https://example.com/mcp","http_headers":{"Authorization":"Bearer example-secret"}}),
        ),
        (
            "copilot",
            json!({"type":"http","url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"},"tools":["*"]}),
        ),
        (
            "vscode",
            json!({"type":"http","url":"https://example.com/mcp","headers":{"Authorization":"Bearer example-secret"}}),
        ),
    ] {
        let adapter = adapters::get(adapter_id).unwrap();
        assert_eq!(
            adapters::encode(&adapter, &http(), None).unwrap(),
            expected,
            "{adapter_id}"
        );
        assert_eq!(
            adapters::decode(&adapter, &expected).unwrap(),
            http(),
            "{adapter_id}"
        );
    }
    assert!(adapters::decode(
        &adapters::get("roo").unwrap(),
        &json!({"url":"https://example.com/mcp"})
    )
    .is_err());
    let cline = adapters::decode(
        &adapters::get("cline").unwrap(),
        &json!({"url":"https://example.com/mcp"}),
    )
    .unwrap();
    assert_eq!(cline.transport, Transport::Sse);
    let opencode = adapters::encode(&adapters::get("opencode").unwrap(), &stdio(), None).unwrap();
    assert_eq!(
        opencode["command"],
        json!(["node", "", "two\nlines", "/space in/path.js"])
    );
    assert_eq!(opencode["environment"], json!({"TOKEN":"line1\nline2"}));
}

#[test]
fn recovery_needed_journal_survives_interrupted_history_save() {
    let (_temp, engine) = setup();
    put(&engine, "cursor", "{\"external\": true}");
    let journal = Journal {
        id: id(),
        status: "recovery-needed".into(),
        files: vec![FileEdit {
            target_id: "cursor".into(),
            path: path(&engine, "cursor").to_string_lossy().into(),
            before: None,
            after: Some("{}".into()),
        }],
        count: 1,
    };
    storage::atomic_write(
        &engine
            .data_dir
            .join("backups")
            .join(format!("{}.json", journal.id)),
        &serde_json::to_string(&journal).unwrap(),
        true,
    )
    .unwrap();
    let home = engine.home.clone();
    let data = engine.data_dir.clone();
    drop(engine);
    let mut engine = Engine::open(home, data).unwrap();
    assert_eq!(engine.workspace.history.len(), 1);
    assert!(!engine.preview().unwrap().errors.is_empty());
    engine.keep_recovery(&journal.id).unwrap();
    let home = engine.home.clone();
    let data = engine.data_dir.clone();
    drop(engine);
    let mut engine = Engine::open(home, data).unwrap();
    assert!(engine.preview().unwrap().errors.is_empty());
    assert_eq!(engine.workspace.history.len(), 1);
}

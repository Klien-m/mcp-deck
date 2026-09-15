use mcp_deck::{
    commands::{dispatch, Request},
    engine::Engine,
};
use serde_json::{json, Value};
use std::fs;

fn request(value: Value) -> Request {
    serde_json::from_value(value).unwrap()
}

#[test]
fn command_contract_accepts_camel_case_arguments_and_rejects_invalid_inputs() {
    let config = json!({"transport":"stdio","command":"node"});
    for value in [
        json!({"op":"snapshot"}),
        json!({"op":"saveService","input":{"id":null,"key":"a","name":"A","description":"","config":config}}),
        json!({"op":"assign","serviceId":"a","targetId":"codex","enabled":true}),
        json!({"op":"remove","serviceId":"a","undo":false}),
        json!({"op":"discover","targetId":"codex"}),
        json!({"op":"adopt","targetId":"codex","keys":["a"]}),
        json!({"op":"importText","adapterId":"codex","text":""}),
        json!({"op":"saveTarget","target":{"id":"codex","adapterId":"codex","name":"Codex","path":"/qa/config.toml"}}),
        json!({"op":"preview"}),
        json!({"op":"preview","includeDetails":true,"reveal":false}),
        json!({"op":"apply","id":"plan"}),
        json!({"op":"resolve","serviceId":"a","targetId":"codex","useDisk":true}),
        json!({"op":"rollback","id":"journal"}),
        json!({"op":"keepRecovery","id":"journal"}),
        json!({"op":"export","adapterId":"codex","serviceIds":["a"],"includeSecrets":false}),
        json!({"op":"saveExport","adapterId":"codex","serviceIds":["a"],"includeSecrets":false,"path":"/qa/export.toml"}),
        json!({"op":"checks","serviceId":"a"}),
    ] {
        assert!(
            serde_json::from_value::<Request>(value.clone()).is_ok(),
            "{value}"
        );
    }
    for value in [
        json!({"op":"snapshott"}),
        json!({"op":"assign","serviceId":"a","enabled":true}),
        json!({"op":"assign","serviceId":"a","targetId":"codex","enabled":"true"}),
        json!({"op":"preview","includeDetails":"true"}),
        json!({"op":"checks","service_id":"a"}),
    ] {
        assert!(
            serde_json::from_value::<Request>(value.clone()).is_err(),
            "{value}"
        );
    }
}

#[test]
fn dispatch_preserves_detailed_preview_plan_and_export_path_protection() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let home = root.join("home");
    fs::create_dir(&home).unwrap();
    let mut engine = Engine::open(home, root.join("data")).unwrap();
    let id = dispatch(
        &mut engine,
        request(json!({
            "op":"saveService",
            "input":{"id":null,"key":"qa","name":"QA","description":"",
                "config":{"transport":"stdio","command":"node","env":{"TOKEN":"fixture-token"}}}
        })),
    )
    .unwrap();
    assert!(id.is_string());
    assert_eq!(
        dispatch(
            &mut engine,
            request(json!({
                "op":"assign","serviceId":id,"targetId":"codex","enabled":true
            }))
        )
        .unwrap(),
        Value::Null
    );

    let preview = dispatch(
        &mut engine,
        request(json!({"op":"preview","includeDetails":true})),
    )
    .unwrap();
    assert_eq!(
        preview["fullChanges"][0]["after"]["env"]["TOKEN"],
        "fixture-token"
    );
    assert_ne!(
        preview["changes"][0]["after"]["env"]["TOKEN"],
        "fixture-token"
    );
    assert_eq!(
        dispatch(
            &mut engine,
            request(json!({"op":"apply","id":preview["id"]}))
        )
        .unwrap(),
        Value::Null
    );
    let aligned = dispatch(&mut engine, request(json!({"op":"preview"}))).unwrap();
    assert!(aligned.get("fullChanges").is_none());
    assert_eq!(aligned["changes"], json!([]));

    let snapshot = dispatch(&mut engine, request(json!({"op":"snapshot"}))).unwrap();
    assert!(snapshot["dataDir"].is_string());
    assert_eq!(snapshot["workspace"]["services"][0]["id"], id);
    let target = engine
        .workspace
        .targets
        .iter()
        .find(|t| t.id == "codex")
        .unwrap()
        .path
        .clone();
    let original = fs::read_to_string(&target).unwrap();
    assert!(dispatch(&mut engine, request(json!({
        "op":"saveExport","adapterId":"codex","serviceIds":[id],"includeSecrets":false,"path":target
    }))).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), original);

    let output = root.join("template.toml");
    assert_eq!(dispatch(&mut engine, request(json!({
        "op":"saveExport","adapterId":"codex","serviceIds":[id],"includeSecrets":false,"path":output
    }))).unwrap(), Value::Null);
    let exported = fs::read_to_string(output).unwrap();
    assert!(exported.contains("[mcp_servers.qa]"));
    assert!(!exported.contains("fixture-token"));
}

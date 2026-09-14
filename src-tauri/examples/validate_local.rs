//! Read real configurations, exercise imports/previews in temporary workspaces, never apply.
use mcp_deck::{
    adapters,
    engine::Engine,
    model::{Result, ServiceInput, Transport},
    storage,
};
use serde_json::{json, Value};
use std::{env, path::Path};

fn fingerprint(path: &str) -> Result<String> {
    storage::read_config(Path::new(path)).map(|text| storage::fingerprint(text.as_deref()))
}

fn run() -> Result<Value> {
    let args: Vec<_> = env::args().skip(1).collect();
    let home = match args.as_slice() {
        [] => dirs::home_dir().ok_or("无法确定用户目录")?,
        [flag, path] if flag == "--home" => Path::new(path)
            .canonicalize()
            .map_err(|_| "指定的配置根目录不存在")?,
        _ => return Err("用法：validate_local [--home 配置根目录]".into()),
    };
    let scratch = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("缺少项目目录")?
        .join(".local-dev");
    storage::private_dir(&scratch)?;
    let temp = tempfile::Builder::new()
        .prefix("validate-local-")
        .tempdir_in(&scratch)
        .map_err(|_| "无法创建临时验证工作区")?;
    let mut engine = Engine::open(home.clone(), temp.path().join("adopt"))?;
    let targets = engine.snapshot().targets;
    let before: Vec<_> = targets
        .iter()
        .map(|target| fingerprint(&target.target.path))
        .collect();
    let mut results = vec![];
    let mut invariants_passed = true;

    for target in &targets {
        let adapter = adapters::get(&target.target.adapter_id)?;
        let mut row = json!({
            "tool": adapter.name,
            "adapter": adapter.id,
            "exists": target.exists,
            "entries": target.count,
        });
        if !target.exists && target.error.is_none() {
            row["status"] = json!("not-found");
            results.push(row);
            continue;
        }
        if target.error.is_some() {
            // Parser messages can contain source fragments; never copy them into reports.
            row["status"] = json!("read-or-parse-error");
            results.push(row);
            continue;
        }
        let discovered = engine.discover(&target.target.id)?;
        let keys: Vec<_> = discovered
            .iter()
            .filter(|entry| entry.config.is_some())
            .map(|entry| entry.key.clone())
            .collect();
        row["supported"] = json!(keys.len());
        row["unsupported"] = json!(discovered.len() - keys.len());
        row["transports"] = json!({
            "stdio": discovered.iter().filter(|d| d.config.as_ref().is_some_and(|c| c.transport == Transport::Stdio)).count(),
            "http": discovered.iter().filter(|d| d.config.as_ref().is_some_and(|c| c.transport == Transport::Http)).count(),
            "sse": discovered.iter().filter(|d| d.config.as_ref().is_some_and(|c| c.transport == Transport::Sse)).count(),
        });
        if !keys.is_empty() {
            engine.adopt(&target.target.id, keys)?;
        }
        let preview = engine.preview()?;
        let no_op = preview.changes.is_empty() && preview.errors.is_empty();
        invariants_passed &= no_op;
        row["adoptionNoChanges"] = json!(no_op);

        // Edit one imported service in the temporary library to exercise real diff generation.
        // Restore it immediately; no apply/rollback or process/network commands are used here.
        if let Some(service) = engine
            .workspace
            .services
            .iter()
            .find(|s| s.targets.contains(&target.target.id))
            .cloned()
        {
            let mut config = service.config.clone();
            if config.transport == Transport::Stdio {
                config.args.push("--mcp-deck-preview-only".into());
            } else {
                config
                    .headers
                    .insert("X-Mcp-Deck-Preview".into(), "1".into());
            }
            engine.save_service(ServiceInput {
                id: Some(service.id.clone()),
                name: service.name.clone(),
                key: service.key.clone(),
                description: service.description.clone(),
                config,
            })?;
            let edited = engine.preview()?;
            let update = edited.errors.is_empty()
                && edited.file_count == 1
                && edited.changes.len() == 1
                && edited.changes[0].action == "update"
                && !edited.changes[0].conflict;
            let expected_block = !adapter.supports_cwd
                && !service.config.cwd.is_empty()
                && !edited.errors.is_empty()
                && edited.file_count == 0
                && edited.changes.is_empty();
            row["editPreview"] = json!(if update {
                "one-update"
            } else if expected_block {
                "blocked-unsupported-cwd"
            } else {
                "unexpected-result"
            });
            invariants_passed &= update || expected_block;
            engine.save_service(ServiceInput {
                id: Some(service.id),
                name: service.name,
                key: service.key,
                description: service.description,
                config: service.config,
            })?;
            let restored = engine.preview()?;
            let restored_no_op = restored.changes.is_empty() && restored.errors.is_empty();
            invariants_passed &= restored_no_op;
            row["restoredNoChanges"] = json!(restored_no_op);
        }

        if !discovered.is_empty() {
            let text = storage::read_config(Path::new(&target.target.path))?
                .ok_or("检测期间配置被删除，请重新运行")?;
            let mut imported = Engine::open(
                home.clone(),
                temp.path().join(format!("import-{}", adapter.id)),
            )?;
            let imported_count = imported.import_text(adapter.id, &text);
            row["textImport"] = match &imported_count {
                Ok(count) => json!({"status": "imported", "count": count}),
                Err(_) => json!({"status": "rejected", "count": 0}),
            };
            let atomic = imported_count.is_ok() || imported.workspace.services.is_empty();
            let unassigned = imported
                .workspace
                .services
                .iter()
                .all(|s| s.targets.is_empty());
            let imported_preview = imported.preview()?;
            let no_writes =
                imported_preview.changes.is_empty() && imported_preview.errors.is_empty();
            let should_import = discovered.iter().all(|entry| entry.config.is_some())
                && discovered.len() <= 100
                && text.len() <= 1024 * 1024;
            let expected_import = if should_import {
                imported_count.as_ref().ok() == Some(&discovered.len())
            } else {
                imported_count.is_err()
            };
            invariants_passed &= atomic && unassigned && no_writes && expected_import;
            row["textImportAtomicAndUnassigned"] = json!(atomic && unassigned && no_writes);
        }
        row["status"] = json!("checked");
        results.push(row);
    }

    for ((target, initial), row) in targets.iter().zip(&before).zip(&mut results) {
        let after = fingerprint(&target.target.path);
        let unchanged = match (initial, after) {
            (Ok(initial), Ok(after)) => Some(initial == &after),
            _ => None,
        };
        invariants_passed &= unchanged != Some(false);
        row["sourceUnchanged"] = json!(unchanged);
    }
    let adopted = engine.workspace.services.len();
    // Drop file locks before removing the temporary workspaces, including all imported secrets.
    drop(engine);
    temp.close().map_err(|_| "无法清理临时验证工作区")?;
    Ok(json!({
        "mode": "configuration-only",
        "scope": if dirs::home_dir().as_ref() == Some(&home) { "current-user" } else { "custom-home" },
        "adopted": adopted,
        "invariantsPassed": invariants_passed,
        "temporaryWorkspacesRemoved": true,
        "targets": results,
    }))
}

fn main() {
    match run() {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            if report["invariantsPassed"] != true {
                std::process::exit(2);
            }
        }
        Err(_) => {
            // Keep failures generic: adapter/engine error strings may include private names.
            eprintln!(
                "验证未完成，请检查目录权限、配置结构或是否有并发修改；未执行应用或恢复操作。"
            );
            std::process::exit(1);
        }
    }
}

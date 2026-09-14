pub mod adapters;
pub mod engine;
pub mod model;
pub mod storage;

#[cfg(feature = "desktop")]
mod desktop {
    use super::*;
    use model::{Result, ServiceInput, Target};
    use serde::Deserialize;
    use serde_json::{json, Value};
    use std::{path::PathBuf, sync::Mutex};

    #[derive(Deserialize)]
    #[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
    pub enum Request {
        Snapshot,
        SaveService {
            input: ServiceInput,
        },
        Assign {
            service_id: String,
            target_id: String,
            enabled: bool,
        },
        Remove {
            service_id: String,
            undo: bool,
        },
        Discover {
            target_id: String,
        },
        Adopt {
            target_id: String,
            keys: Vec<String>,
        },
        ImportText {
            adapter_id: String,
            text: String,
        },
        SaveTarget {
            target: Target,
        },
        Preview {
            #[serde(default)]
            reveal: bool,
            #[serde(default)]
            include_details: bool,
        },
        Apply {
            id: String,
        },
        Resolve {
            service_id: String,
            target_id: String,
            use_disk: bool,
        },
        Rollback {
            id: String,
        },
        KeepRecovery {
            id: String,
        },
        Export {
            adapter_id: String,
            service_ids: Vec<String>,
            include_secrets: bool,
        },
        SaveExport {
            adapter_id: String,
            service_ids: Vec<String>,
            include_secrets: bool,
            path: String,
        },
        Checks {
            service_id: String,
        },
    }

    #[tauri::command]
    pub fn dispatch(
        state: tauri::State<'_, Mutex<Result<engine::Engine>>>,
        request: Request,
    ) -> Result<Value> {
        let mut lock = state.lock().map_err(|_| "应用状态异常，请重新启动")?;
        let engine = lock.as_mut().map_err(|e| e.clone())?;
        match request {
            Request::Snapshot => Ok(json!(engine.snapshot())),
            Request::SaveService { input } => Ok(json!(engine.save_service(input)?)),
            Request::Assign {
                service_id,
                target_id,
                enabled,
            } => {
                engine.assign(&service_id, &target_id, enabled)?;
                Ok(Value::Null)
            }
            Request::Remove { service_id, undo } => {
                engine.remove(&service_id, undo)?;
                Ok(Value::Null)
            }
            Request::Discover { target_id } => Ok(json!(engine.discover(&target_id)?)),
            Request::Adopt { target_id, keys } => {
                engine.adopt(&target_id, keys)?;
                Ok(Value::Null)
            }
            Request::ImportText { adapter_id, text } => {
                Ok(json!(engine.import_text(&adapter_id, &text)?))
            }
            Request::SaveTarget { target } => {
                engine.save_target(target)?;
                Ok(Value::Null)
            }
            Request::Preview {
                reveal,
                include_details,
            } => Ok(json!(if include_details {
                engine.preview_with_details()?
            } else {
                engine.preview_values(reveal)?
            })),
            Request::Apply { id } => {
                engine.apply(&id)?;
                Ok(Value::Null)
            }
            Request::Resolve {
                service_id,
                target_id,
                use_disk,
            } => {
                engine.resolve(&service_id, &target_id, use_disk)?;
                Ok(Value::Null)
            }
            Request::Rollback { id } => {
                engine.rollback(&id)?;
                Ok(Value::Null)
            }
            Request::KeepRecovery { id } => {
                engine.keep_recovery(&id)?;
                Ok(Value::Null)
            }
            Request::Export {
                adapter_id,
                service_ids,
                include_secrets,
            } => Ok(json!(engine.export(
                &adapter_id,
                &service_ids,
                include_secrets
            )?)),
            Request::SaveExport {
                adapter_id,
                service_ids,
                include_secrets,
                path,
            } => {
                let path = PathBuf::from(path);
                storage::validate_path(&path)?;
                if path.starts_with(&engine.data_dir)
                    || engine
                        .workspace
                        .targets
                        .iter()
                        .any(|t| std::path::Path::new(&t.path) == path)
                {
                    return Err("导出不能覆盖数据目录或正在管理的配置，请选择其他文件".into());
                }
                let text = engine.export(&adapter_id, &service_ids, include_secrets)?;
                storage::atomic_write(&path, &text, true)?;
                Ok(Value::Null)
            }
            Request::Checks { service_id } => engine.checks(&service_id),
        }
    }
}

#[cfg(feature = "desktop")]
pub fn run() {
    use std::{path::PathBuf, sync::Mutex};
    // Debug builds always use project-local fixtures unless an explicit test home is supplied.
    // Release builds use the user's real home and never inherit a compiled-in fixture path.
    #[cfg(debug_assertions)]
    if std::env::var_os("MCP_DECK_HOME").is_none() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".local-dev");
        std::env::set_var("MCP_DECK_HOME", root.join("fixture-home"));
        std::env::set_var("MCP_DECK_DATA_DIR", root.join("fixture-data"));
    }
    let home = std::env::var_os("MCP_DECK_HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .expect("无法确定用户目录");
    let data = std::env::var_os("MCP_DECK_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if std::env::var_os("MCP_DECK_HOME").is_some() {
                return home.join(".mcp-deck-data");
            }
            dirs::data_dir()
                .unwrap_or_else(|| home.join(".local/share"))
                .join("com.mcpdeck.desktop")
        });
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(engine::Engine::open(home, data)))
        .invoke_handler(tauri::generate_handler![desktop::dispatch])
        .run(tauri::generate_context!())
        .expect("MCP Deck 无法启动");
}

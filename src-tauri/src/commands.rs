//! IPC request contract and application dispatch, independent of the desktop runtime.
use crate::{
    engine::Engine,
    model::{Result, ServiceInput, Target},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

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

pub fn dispatch(engine: &mut Engine, request: Request) -> Result<Value> {
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
            engine.save_export(&adapter_id, &service_ids, include_secrets, &path)?;
            Ok(Value::Null)
        }
        Request::Checks { service_id } => engine.checks(&service_id),
    }
}

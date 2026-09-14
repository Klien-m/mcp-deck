use crate::{adapters::Adapter, model::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetStatus {
    #[serde(flatten)]
    pub target: Target,
    pub exists: bool,
    pub count: usize,
    pub error: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub workspace: Workspace,
    pub adapters: Vec<Adapter>,
    pub targets: Vec<TargetStatus>,
    pub data_dir: String,
    pub isolated: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub key: String,
    pub config: Option<Config>,
    pub preview: Value,
    pub error: Option<String>,
    pub managed: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEdit {
    pub target_id: String,
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
pub(super) struct Plan {
    pub(super) id: String,
    pub(super) revision: u64,
    pub(super) files: Vec<FileEdit>,
    pub(super) change_count: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub changes: Vec<Change>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_changes: Option<Vec<Change>>,
    pub errors: Vec<String>,
    pub file_count: usize,
}

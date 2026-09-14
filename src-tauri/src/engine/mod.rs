mod diagnostics;
mod discovery;
mod planner;
mod services;
mod sync;
mod targets;
mod transaction;
mod transfer;
mod types;

use targets::{find_target, read_target};
pub use transaction::Journal;
use types::Plan;
pub use types::{Discovery, FileEdit, Preview, Snapshot, TargetStatus};

use crate::{adapters, model::*, storage};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

pub struct Engine {
    pub workspace: Workspace,
    pub data_dir: PathBuf,
    pub home: PathBuf,
    plan: Option<Plan>,
    _lock: File,
}

impl Engine {
    pub fn open(home: PathBuf, data_dir: PathBuf) -> Result<Self> {
        storage::private_dir(&data_dir)?;
        storage::private_dir(&data_dir.join("backups"))?;
        let lock = storage::lock(&data_dir.join("workspace.lock"))?;
        let path = data_dir.join("workspace.json");
        let workspace = if path.exists() {
            let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let state: Workspace = serde_json::from_str(&data)
                .map_err(|_| "工作区文件损坏；已停止加载以避免覆盖，请检查备份")?;
            if state.version != 1 {
                return Err("工作区版本不兼容，已停止加载".into());
            }
            state
        } else {
            Workspace {
                version: 1,
                revision: 0,
                services: vec![],
                targets: adapters::default_targets(&home),
                history: vec![],
            }
        };
        let mut engine = Self {
            workspace,
            data_dir,
            home,
            plan: None,
            _lock: lock,
        };
        engine.transact(|transaction| transaction.recover())?;
        engine.save()?;
        Ok(engine)
    }

    fn save(&self) -> Result<()> {
        transaction::save_workspace(&self.workspace, &self.data_dir)
    }

    fn commit(&mut self, next: Workspace) -> Result<()> {
        transaction::commit_workspace(&mut self.workspace, &self.data_dir, next)?;
        self.plan = None;
        Ok(())
    }

    // Publish only after the complete use case and durable workspace write succeed.
    fn update<T>(&mut self, operation: impl FnOnce(&mut Workspace) -> Result<T>) -> Result<T> {
        let mut next = self.workspace.clone();
        let result = operation(&mut next)?;
        self.commit(next)?;
        Ok(result)
    }

    fn target(&self, id: &str) -> Result<Target> {
        find_target(&self.workspace, id)
    }

    fn read_target(&self, target: &Target) -> Result<(Option<String>, BTreeMap<String, Value>)> {
        read_target(target)
    }

    pub fn snapshot(&self) -> Snapshot {
        let targets = self
            .workspace
            .targets
            .iter()
            .map(|t| {
                let result = self.read_target(t);
                TargetStatus {
                    target: t.clone(),
                    exists: Path::new(&t.path).is_file(),
                    count: result.as_ref().map(|(_, e)| e.len()).unwrap_or(0),
                    error: result.err(),
                }
            })
            .collect();
        Snapshot {
            workspace: self.workspace.clone(),
            adapters: adapters::registry(),
            targets,
            data_dir: self.data_dir.to_string_lossy().into(),
            isolated: std::env::var_os("MCP_DECK_HOME").is_some(),
        }
    }

    pub fn save_service(&mut self, input: ServiceInput) -> Result<String> {
        self.update(|workspace| services::save(workspace, input))
    }

    pub fn assign(&mut self, service_id: &str, target_id: &str, enabled: bool) -> Result<()> {
        self.update(|workspace| services::assign(workspace, service_id, target_id, enabled))
    }

    pub fn remove(&mut self, service_id: &str, undo: bool) -> Result<()> {
        self.update(|workspace| services::remove(workspace, service_id, undo))
    }

    pub fn discover(&self, target_id: &str) -> Result<Vec<Discovery>> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        discovery::discover(&self.workspace, &target, entries)
    }

    pub fn adopt(&mut self, target_id: &str, keys: Vec<String>) -> Result<()> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        self.update(|workspace| discovery::adopt(workspace, &target, &entries, keys))
    }

    pub fn import_text(&mut self, adapter_id: &str, text: &str) -> Result<usize> {
        self.update(|workspace| discovery::import_text(workspace, adapter_id, text))
    }

    pub fn save_target(&mut self, target: Target) -> Result<()> {
        let data_dir = self.data_dir.clone();
        self.update(|workspace| targets::save(workspace, &data_dir, target))
    }

    pub fn resolve(&mut self, service_id: &str, target_id: &str, use_disk: bool) -> Result<()> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        self.update(|workspace| {
            services::resolve(workspace, &target, &entries, service_id, use_disk)
        })
    }

    pub fn export(
        &self,
        adapter_id: &str,
        service_ids: &[String],
        include_secrets: bool,
    ) -> Result<String> {
        transfer::export(&self.workspace, adapter_id, service_ids, include_secrets)
    }

    pub fn save_export(
        &self,
        adapter_id: &str,
        service_ids: &[String],
        include_secrets: bool,
        path: &Path,
    ) -> Result<()> {
        transfer::save_export(
            &self.workspace,
            &self.data_dir,
            adapter_id,
            service_ids,
            include_secrets,
            path,
        )
    }

    pub fn checks(&self, service_id: &str) -> Result<Value> {
        diagnostics::checks(&self.workspace, &self.home, service_id)
    }
}

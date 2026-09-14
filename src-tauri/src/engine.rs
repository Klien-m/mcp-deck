use crate::{
    adapters::{self, Adapter},
    model::*,
    storage,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::{Path, PathBuf},
};

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
#[derive(Clone, Serialize, Deserialize)]
pub struct Journal {
    pub id: String,
    pub status: String,
    pub files: Vec<FileEdit>,
    pub count: usize,
}
#[derive(Clone)]
struct Plan {
    id: String,
    revision: u64,
    files: Vec<FileEdit>,
    changes: Vec<Change>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub changes: Vec<Change>,
    pub errors: Vec<String>,
    pub file_count: usize,
}

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
        engine.recover()?;
        engine.save()?;
        Ok(engine)
    }

    fn save(&self) -> Result<()> {
        storage::atomic_write(
            &self.data_dir.join("workspace.json"),
            &serde_json::to_string_pretty(&self.workspace).map_err(|e| e.to_string())?,
            true,
        )
    }
    fn commit(&mut self, next: Workspace) -> Result<()> {
        let old = std::mem::replace(&mut self.workspace, next);
        self.workspace.revision += 1;
        if let Err(e) = self.save() {
            self.workspace = old;
            return Err(e);
        }
        self.plan = None;
        Ok(())
    }
    fn target(&self, id: &str) -> Result<Target> {
        self.workspace
            .targets
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| "目标工具不存在".into())
    }
    fn read_target(&self, target: &Target) -> Result<(Option<String>, BTreeMap<String, Value>)> {
        storage::validate_path(Path::new(&target.path))?;
        let text = storage::read_config(Path::new(&target.path))?;
        let entries = match &text {
            Some(s) => adapters::parse(&adapters::get(&target.adapter_id)?, s)?,
            None => BTreeMap::new(),
        };
        Ok((text, entries))
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
        input.config.validate()?;
        if input.name.trim().is_empty()
            || input.name.len() > 120
            || input.key.trim().is_empty()
            || input.key.len() > 120
            || input.key.chars().any(char::is_control)
        {
            return Err("服务名称与配置键不能为空、过长或包含控制字符".into());
        }
        if input.description.len() > 1000 {
            return Err("描述不能超过 1000 字节".into());
        }
        let mut next = self.workspace.clone();
        let service_id = match input.id {
            Some(id) => {
                let service = next
                    .services
                    .iter_mut()
                    .find(|s| s.id == id && !s.deleted)
                    .ok_or("服务不存在")?;
                if service.key != input.key {
                    return Err("现有服务的配置键不可修改，请新建服务后分配".into());
                }
                service.name = input.name.trim().into();
                service.description = input.description;
                service.config = input.config;
                id
            }
            None => {
                if next.services.len() >= 500 {
                    return Err("内部版最多管理 500 个服务（含待移除项）".into());
                }
                let sid = id();
                next.services.push(Service {
                    id: sid.clone(),
                    key: input.key,
                    name: input.name.trim().into(),
                    description: input.description,
                    config: input.config,
                    targets: vec![],
                    bindings: BTreeMap::new(),
                    native: BTreeMap::new(),
                    deleted: false,
                    deleted_targets: vec![],
                });
                sid
            }
        };
        self.commit(next)?;
        Ok(service_id)
    }

    pub fn assign(&mut self, service_id: &str, target_id: &str, enabled: bool) -> Result<()> {
        let target = self.target(target_id)?;
        let service = self
            .workspace
            .services
            .iter()
            .find(|s| s.id == service_id && !s.deleted)
            .ok_or("服务不存在")?;
        if enabled {
            if self.workspace.services.iter().any(|s| {
                s.id != service_id
                    && s.key == service.key
                    && (s.targets.contains(&target.id)
                        || s.bindings.get(&target.id).is_some_and(|b| b.raw.is_some()))
            }) {
                return Err("该工具中已有另一个同配置键服务，请先移除旧绑定并应用".into());
            }
            adapters::encode(
                &adapters::get(&target.adapter_id)?,
                &service.config,
                service.bindings.get(target_id).and_then(|b| b.raw.as_ref()),
            )?;
        }
        let mut next = self.workspace.clone();
        let service = next
            .services
            .iter_mut()
            .find(|s| s.id == service_id)
            .unwrap();
        service.targets.retain(|t| t != target_id);
        if enabled {
            service.targets.push(target_id.into());
        }
        self.commit(next)
    }

    pub fn remove(&mut self, service_id: &str, undo: bool) -> Result<()> {
        let mut next = self.workspace.clone();
        let service = next
            .services
            .iter_mut()
            .find(|s| s.id == service_id)
            .ok_or("服务不存在")?;
        if undo {
            service.deleted = false;
            service.targets = service.deleted_targets.clone();
        } else {
            service.deleted = true;
            service.deleted_targets = service.targets.clone();
            service.targets.clear();
        }
        self.commit(next)
    }

    pub fn discover(&self, target_id: &str) -> Result<Vec<Discovery>> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        let adapter = adapters::get(&target.adapter_id)?;
        Ok(entries
            .into_iter()
            .map(|(key, raw)| {
                let decoded = adapters::decode(&adapter, &raw);
                let managed = self.workspace.services.iter().any(|s| {
                    s.key == key && s.bindings.get(target_id).is_some_and(|b| b.raw.is_some())
                });
                Discovery {
                    key,
                    config: decoded.as_ref().ok().cloned(),
                    preview: redact(&raw),
                    error: decoded.err(),
                    managed,
                }
            })
            .collect())
    }

    pub fn adopt(&mut self, target_id: &str, keys: Vec<String>) -> Result<()> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        let adapter = adapters::get(&target.adapter_id)?;
        let mut next = self.workspace.clone();
        let mut unique = BTreeSet::new();
        for key in keys {
            if !unique.insert(key.clone()) {
                continue;
            }
            if next.services.iter().any(|s| {
                s.key == key
                    && (s.targets.contains(&target.id)
                        || s.bindings.get(target_id).is_some_and(|b| b.raw.is_some()))
            }) {
                return Err(format!("{key} 已在该目标纳入管理"));
            }
            let raw = entries.get(&key).ok_or("配置已变化，请刷新发现列表")?;
            let config = adapters::decode(&adapter, raw)?;
            next.services.push(Service {
                id: id(),
                name: key.clone(),
                key,
                description: format!("导入自 {}", target.name),
                config,
                targets: vec![target.id.clone()],
                bindings: BTreeMap::from([(
                    target.id.clone(),
                    Binding {
                        raw: Some(raw.clone()),
                    },
                )]),
                native: BTreeMap::from([(adapter.id.into(), raw.clone())]),
                deleted: false,
                deleted_targets: vec![],
            });
        }
        if next.services.len() > 500 {
            return Err("服务数量超过内部版上限".into());
        }
        self.commit(next)
    }

    pub fn import_text(&mut self, adapter_id: &str, text: &str) -> Result<usize> {
        if text.len() > 1024 * 1024 {
            return Err("导入内容不能超过 1 MB".into());
        }
        let adapter = adapters::get(adapter_id)?;
        let entries = adapters::parse(&adapter, text)?;
        if entries.is_empty() || entries.len() > 100 {
            return Err("一次导入需要包含 1–100 个服务".into());
        }
        let mut next = self.workspace.clone();
        for (key, raw) in &entries {
            let config = adapters::decode(&adapter, raw)?;
            next.services.push(Service {
                id: id(),
                name: key.clone(),
                key: key.clone(),
                description: format!("导入 {} 格式；尚未分配", adapter.name),
                config,
                targets: vec![],
                bindings: BTreeMap::new(),
                native: BTreeMap::from([(adapter.id.into(), raw.clone())]),
                deleted: false,
                deleted_targets: vec![],
            });
        }
        if next.services.len() > 500 {
            return Err("服务数量超过内部版上限".into());
        }
        self.commit(next)?;
        Ok(entries.len())
    }

    pub fn save_target(&mut self, mut target: Target) -> Result<()> {
        adapters::get(&target.adapter_id)?;
        storage::validate_path(Path::new(&target.path))?;
        target.path = Path::new(&target.path)
            .components()
            .collect::<PathBuf>()
            .to_string_lossy()
            .into();
        if Path::new(&target.path).starts_with(&self.data_dir) {
            return Err("目标配置不能位于 MCP Deck 数据目录内".into());
        }
        if target.name.trim().is_empty() || target.name.len() > 100 {
            return Err("请输入目标名称（不超过 100 字节）".into());
        }
        if self
            .workspace
            .targets
            .iter()
            .any(|t| t.id != target.id && t.path == target.path)
        {
            return Err("该路径已由另一个目标管理".into());
        }
        let mut next = self.workspace.clone();
        if let Some(old) = next.targets.iter_mut().find(|t| t.id == target.id) {
            if (old.path != target.path || old.adapter_id != target.adapter_id)
                && self.workspace.services.iter().any(|s| {
                    s.targets.contains(&target.id)
                        || s.bindings.get(&target.id).is_some_and(|b| b.raw.is_some())
                })
            {
                return Err("该目标仍有关联配置，请先取消分配并应用，再修改路径".into());
            }
            *old = target;
        } else {
            if next.targets.len() >= 40 {
                return Err("最多支持 40 个配置目标".into());
            }
            next.targets.push(Target { id: id(), ..target });
        }
        self.commit(next)
    }

    pub fn preview(&mut self) -> Result<Preview> {
        self.preview_values(false)
    }

    pub fn preview_values(&mut self, reveal: bool) -> Result<Preview> {
        let mut changes = vec![];
        let mut errors = vec![];
        let mut files = vec![];
        if self
            .workspace
            .history
            .iter()
            .any(|h| h.status == "recovery-needed")
        {
            errors.push("存在未解决的恢复记录，请先在同步记录中处理".into());
        }
        for target in &self.workspace.targets {
            let relevant: Vec<_> = self
                .workspace
                .services
                .iter()
                .filter(|s| {
                    s.targets.contains(&target.id)
                        || s.bindings.get(&target.id).is_some_and(|b| b.raw.is_some())
                })
                .collect();
            if relevant.is_empty() {
                continue;
            }
            let (before, entries) = match self.read_target(target) {
                Ok(r) => r,
                Err(e) => {
                    errors.push(format!("{}：{e}", target.name));
                    continue;
                }
            };
            let adapter = adapters::get(&target.adapter_id)?;
            let mut patches = BTreeMap::new();
            for service in relevant {
                let expected = service
                    .bindings
                    .get(&target.id)
                    .and_then(|b| b.raw.as_ref());
                let actual = entries.get(&service.key);
                let wanted = !service.deleted && service.targets.contains(&target.id);
                let desired = if wanted {
                    let base = expected.or_else(|| service.native.get(&target.adapter_id));
                    match adapters::encode(&adapter, &service.config, base) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            errors.push(format!("{} / {}：{e}", service.name, target.name));
                            continue;
                        }
                    }
                } else {
                    None
                };
                let conflict = actual != expected;
                if desired.as_ref() == actual && !conflict {
                    continue;
                }
                let message = if conflict {
                    if expected.is_none() {
                        "发现同名已有配置，需要明确选择保留哪一版"
                    } else {
                        "磁盘配置与上次同步基线不同"
                    }
                } else {
                    ""
                };
                let action = if !wanted {
                    "remove"
                } else if actual.is_none() {
                    "add"
                } else {
                    "update"
                };
                changes.push(Change {
                    service_id: service.id.clone(),
                    target_id: target.id.clone(),
                    target_name: target.name.clone(),
                    key: service.key.clone(),
                    action: action.into(),
                    before: actual.map(|v| if reveal { v.clone() } else { redact(v) }),
                    after: desired
                        .as_ref()
                        .map(|v| if reveal { v.clone() } else { redact(v) }),
                    conflict,
                    message: message.into(),
                });
                if patches.insert(service.key.clone(), desired).is_some() {
                    errors.push(format!("{} 中存在重复配置键 {}", target.name, service.key));
                }
            }
            if !patches.is_empty() {
                match adapters::patch(&adapter, before.as_deref(), &patches) {
                    Ok(after) => files.push(FileEdit {
                        target_id: target.id.clone(),
                        path: target.path.clone(),
                        before,
                        after: Some(after),
                    }),
                    Err(e) => errors.push(format!("{}：{e}", target.name)),
                }
            }
        }
        let plan_id = id();
        let file_count = files.len();
        if errors.is_empty() && !changes.iter().any(|c| c.conflict) {
            self.plan = Some(Plan {
                id: plan_id.clone(),
                revision: self.workspace.revision,
                files,
                changes: changes.clone(),
            });
        } else {
            self.plan = None;
        }
        Ok(Preview {
            id: plan_id,
            changes,
            errors,
            file_count,
        })
    }

    pub fn resolve(&mut self, service_id: &str, target_id: &str, use_disk: bool) -> Result<()> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        let mut next = self.workspace.clone();
        let service = next
            .services
            .iter_mut()
            .find(|s| s.id == service_id)
            .ok_or("服务不存在")?;
        let raw = entries.get(&service.key).cloned();
        if use_disk {
            match &raw {
                Some(v) => {
                    service.config = adapters::decode(&adapters::get(&target.adapter_id)?, v)?;
                    service.deleted = false;
                    if !service.targets.contains(&target.id) {
                        service.targets.push(target.id.clone());
                    }
                }
                None => {
                    service.targets.retain(|t| t != target_id);
                }
            }
        }
        service.bindings.insert(target_id.into(), Binding { raw });
        self.commit(next)
    }

    fn journal_path(&self, journal_id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(journal_id).map_err(|_| "无效的记录 ID")?;
        Ok(self
            .data_dir
            .join("backups")
            .join(format!("{journal_id}.json")))
    }
    fn save_journal(&self, journal: &Journal) -> Result<()> {
        storage::atomic_write(
            &self.journal_path(&journal.id)?,
            &serde_json::to_string_pretty(journal).map_err(|e| e.to_string())?,
            true,
        )
    }
    fn load_journal(&self, journal_id: &str) -> Result<Journal> {
        let path = self.journal_path(journal_id)?;
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
            return Err("备份文件过大".into());
        }
        serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|_| "恢复记录无法读取".into())
    }
    fn restore_files(files: &[FileEdit]) -> Vec<String> {
        let mut failures = vec![];
        for file in files.iter().rev() {
            let path = Path::new(&file.path);
            let result = (|| {
                let current = storage::read_config(path)?;
                if current == file.before {
                    return Ok(());
                }
                storage::write_checked(path, file.after.as_deref(), file.before.as_deref())
            })();
            if let Err(e) = result {
                failures.push(e);
            }
        }
        failures
    }

    fn recover(&mut self) -> Result<()> {
        for entry in fs::read_dir(self.data_dir.join("backups")).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            let Some(jid) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let mut journal = self.load_journal(jid)?;
            if journal.status != "prepared" {
                // The journal may have been saved immediately before a crash interrupted the workspace save.
                if journal.status == "recovery-needed"
                    && !self.workspace.history.iter().any(|h| h.id == journal.id)
                {
                    self.workspace.history.push(History {
                        id: journal.id,
                        at: now(),
                        status: "recovery-needed".into(),
                        summary: "上次恢复尚未完成，请选择恢复原文件或保留当前文件".into(),
                        paths: journal.files.iter().map(|f| f.path.clone()).collect(),
                        count: journal.count,
                    });
                }
                continue;
            }
            if self
                .workspace
                .history
                .iter()
                .any(|h| h.id == journal.id && h.status == "applied")
            {
                journal.status = "applied".into();
                self.save_journal(&journal)?;
                continue;
            }
            let failures = Self::restore_files(&journal.files);
            journal.status = if failures.is_empty() {
                "recovered"
            } else {
                "recovery-needed"
            }
            .into();
            self.save_journal(&journal)?;
            self.workspace.history.push(History {
                id: journal.id.clone(),
                at: now(),
                status: journal.status,
                summary: if failures.is_empty() {
                    "已恢复上次中断的写入".into()
                } else {
                    format!("检测到外部修改，部分文件需要处理：{}", failures.join("；"))
                },
                paths: journal.files.iter().map(|f| f.path.clone()).collect(),
                count: journal.count,
            });
        }
        Ok(())
    }

    fn execute(&mut self, plan: Plan, summary: String) -> Result<()> {
        if plan.files.is_empty() {
            return Err("当前没有需要应用的变更".into());
        }
        let total: usize = plan
            .files
            .iter()
            .map(|f| {
                f.before.as_ref().map_or(0, String::len) + f.after.as_ref().map_or(0, String::len)
            })
            .sum();
        if total > 12 * 1024 * 1024 {
            return Err("本次配置总量过大，请分批应用".into());
        }
        for file in &plan.files {
            if storage::read_config(Path::new(&file.path))? != file.before {
                return Err("配置在预览后发生变化，已阻止写入，请重新预览".into());
            }
        }
        let mut journal = Journal {
            id: plan.id.clone(),
            status: "prepared".into(),
            files: plan.files.clone(),
            count: plan.changes.len(),
        };
        self.save_journal(&journal)?;
        let operation = (|| {
            for file in &plan.files {
                storage::write_checked(
                    Path::new(&file.path),
                    file.before.as_deref(),
                    file.after.as_deref(),
                )?;
            }
            let mut next = self.workspace.clone();
            for file in &plan.files {
                let target = self.target(&file.target_id)?;
                let entries = if let Some(text) = &file.after {
                    adapters::parse(&adapters::get(&target.adapter_id)?, text)?
                } else {
                    BTreeMap::new()
                };
                for service in &mut next.services {
                    if service.targets.contains(&target.id)
                        || service.bindings.contains_key(&target.id)
                    {
                        service.bindings.insert(
                            target.id.clone(),
                            Binding {
                                raw: entries.get(&service.key).cloned(),
                            },
                        );
                    }
                }
            }
            next.history.push(History {
                id: plan.id.clone(),
                at: now(),
                status: "applied".into(),
                summary,
                paths: plan.files.iter().map(|f| f.path.clone()).collect(),
                count: plan.changes.len(),
            });
            self.commit(next)
        })();
        if let Err(error) = operation {
            let failures = Self::restore_files(&plan.files);
            journal.status = if failures.is_empty() {
                "recovered"
            } else {
                "recovery-needed"
            }
            .into();
            self.save_journal(&journal)?;
            let mut next = self.workspace.clone();
            next.history.push(History {
                id: journal.id,
                at: now(),
                status: journal.status,
                summary: format!(
                    "写入失败：{error}。{}",
                    if failures.is_empty() {
                        "已恢复原文件".into()
                    } else {
                        failures.join("；")
                    }
                ),
                paths: plan.files.iter().map(|f| f.path.clone()).collect(),
                count: plan.changes.len(),
            });
            self.commit(next)?;
            return Err(error);
        }
        journal.status = "applied".into();
        // Workspace commit is authoritative; recovery recognizes its transaction ID if this fails.
        let _ = self.save_journal(&journal);
        Ok(())
    }

    pub fn apply(&mut self, plan_id: &str) -> Result<()> {
        let plan = self.plan.take().ok_or("预览已失效，请重新预览")?;
        if plan.id != plan_id || plan.revision != self.workspace.revision {
            return Err("服务库在预览后发生变化，请重新预览".into());
        }
        self.execute(plan, "已写入配置；请在目标工具中刷新或授权".into())
    }

    pub fn rollback(&mut self, journal_id: &str) -> Result<()> {
        let journal = self.load_journal(journal_id)?;
        if !["applied", "recovery-needed"].contains(&journal.status.as_str()) {
            return Err("该记录无需恢复".into());
        }
        let mut files = vec![];
        for file in &journal.files {
            let target = self.target(&file.target_id)?;
            if target.path != file.path {
                return Err("目标路径已变化，无法自动恢复旧路径".into());
            }
            let current = storage::read_config(Path::new(&file.path))?;
            if current == file.before {
                continue;
            }
            if current != file.after {
                return Err(format!("检测到后来修改，无法覆盖恢复：{}", file.path));
            }
            files.push(FileEdit {
                target_id: file.target_id.clone(),
                path: file.path.clone(),
                before: current,
                after: file.before.clone(),
            });
        }
        if !files.is_empty() {
            self.execute(
                Plan {
                    id: id(),
                    revision: self.workspace.revision,
                    files,
                    changes: vec![],
                },
                format!(
                    "已恢复记录 {} 的文件；服务库保留，可重新预览",
                    &journal_id[..8]
                ),
            )?;
        }
        let mut next = self.workspace.clone();
        if let Some(h) = next.history.iter_mut().find(|h| h.id == journal_id) {
            h.status = "rolled-back".into();
        }
        self.commit(next)?;
        self.save_journal(&Journal {
            status: "rolled-back".into(),
            ..journal
        })
    }

    pub fn export(
        &self,
        adapter_id: &str,
        service_ids: &[String],
        include_secrets: bool,
    ) -> Result<String> {
        let adapter = adapters::get(adapter_id)?;
        let mut patches = BTreeMap::new();
        for service in self
            .workspace
            .services
            .iter()
            .filter(|s| !s.deleted && (service_ids.is_empty() || service_ids.contains(&s.id)))
        {
            let raw = adapters::encode(&adapter, &service.config, service.native.get(adapter_id))?;
            let raw = if include_secrets { raw } else { redact(&raw) };
            if patches.insert(service.key.clone(), Some(raw)).is_some() {
                return Err("存在相同配置键，请选择一个服务单独导出".into());
            }
        }
        adapters::patch(&adapter, None, &patches)
    }

    pub fn keep_recovery(&mut self, journal_id: &str) -> Result<()> {
        let mut journal = self.load_journal(journal_id)?;
        if journal.status != "recovery-needed" {
            return Err("该记录没有待解决的恢复问题".into());
        }
        let mut next = self.workspace.clone();
        for file in &journal.files {
            let target = self.target(&file.target_id)?;
            if target.path != file.path {
                return Err("路径已变化，无法重建恢复基线".into());
            }
            let (_, entries) = self.read_target(&target)?;
            for service in &mut next.services {
                if service.targets.contains(&target.id) || service.bindings.contains_key(&target.id)
                {
                    service.bindings.insert(
                        target.id.clone(),
                        Binding {
                            raw: entries.get(&service.key).cloned(),
                        },
                    );
                }
            }
        }
        if let Some(h) = next.history.iter_mut().find(|h| h.id == journal_id) {
            h.status = "recovery-kept".into();
            h.summary = "用户选择保留当前磁盘文件；已重建基线，后续变更仍需预览".into();
        }
        self.commit(next)?;
        journal.status = "recovery-kept".into();
        self.save_journal(&journal)
    }

    pub fn checks(&self, service_id: &str) -> Result<Value> {
        let service = self
            .workspace
            .services
            .iter()
            .find(|s| s.id == service_id)
            .ok_or("服务不存在")?;
        let mut issues = vec![];
        if let Err(e) = service.config.validate() {
            issues.push(e);
        }
        for (key, value) in service
            .config
            .env
            .iter()
            .chain(service.config.headers.iter())
        {
            if value.trim().is_empty() || value.contains("<已隐藏>") {
                issues.push(format!("{key} 尚未填写"));
            }
            if value.contains("${") || value.contains("{env:") {
                issues.push(format!("{key} 使用变量引用，需在目标工具的运行环境中确认"));
            }
        }
        let mut executable = None;
        if service.config.transport == Transport::Stdio {
            let command = &service.config.command;
            let candidates = if Path::new(command).is_absolute() {
                vec![PathBuf::from(command)]
            } else {
                let mut dirs = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                    .collect::<Vec<_>>();
                dirs.extend([
                    PathBuf::from("/opt/homebrew/bin"),
                    PathBuf::from("/usr/local/bin"),
                    self.home.join(".local/bin"),
                ]);
                dirs.into_iter().map(|p| p.join(command)).collect()
            };
            executable = candidates
                .into_iter()
                .find(|p| p.is_file())
                .map(|p| p.to_string_lossy().into_owned());
            if executable.is_none() {
                issues.push("未发现启动命令，请安装依赖或使用可执行文件的完整路径".into());
            }
        }
        Ok(
            json!({"issues":issues,"executable":executable,"note":"仅检查配置字段与命令位置；未启动进程、未执行网络连接。最终加载和认证状态请在目标工具中确认。"}),
        )
    }
}

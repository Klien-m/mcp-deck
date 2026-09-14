//! Journal, target files and workspace publication are one recovery protocol.
use super::{find_target, read_target, FileEdit, Plan};
use crate::{adapters, model::*, storage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Journal {
    pub id: String,
    pub status: String,
    pub files: Vec<FileEdit>,
    pub count: usize,
}

pub(super) fn save_workspace(workspace: &Workspace, data_dir: &Path) -> Result<()> {
    storage::atomic_write(
        &data_dir.join("workspace.json"),
        &serde_json::to_string_pretty(workspace).map_err(|error| error.to_string())?,
        true,
    )
}

pub(super) fn commit_workspace(
    workspace: &mut Workspace,
    data_dir: &Path,
    next: Workspace,
) -> Result<()> {
    let old = std::mem::replace(workspace, next);
    workspace.revision += 1;
    if let Err(error) = save_workspace(workspace, data_dir) {
        *workspace = old;
        return Err(error);
    }
    Ok(())
}

pub(super) struct Transaction<'a> {
    workspace: &'a mut Workspace,
    data_dir: &'a Path,
}

impl<'a> Transaction<'a> {
    pub(super) fn new(workspace: &'a mut Workspace, data_dir: &'a Path) -> Self {
        Self {
            workspace,
            data_dir,
        }
    }

    fn commit(&mut self, next: Workspace) -> Result<()> {
        commit_workspace(self.workspace, self.data_dir, next)
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

    pub(super) fn recover(&mut self) -> Result<()> {
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

    pub(super) fn execute(&mut self, plan: Plan, summary: String) -> Result<()> {
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
            count: plan.change_count,
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
                let target = find_target(self.workspace, &file.target_id)?;
                let entries = if let Some(text) = &file.after {
                    adapters::parse(&adapters::get(&target.adapter_id)?, text)?
                } else {
                    BTreeMap::new()
                };
                update_bindings(&mut next, &target.id, &entries);
            }
            next.history.push(History {
                id: plan.id.clone(),
                at: now(),
                status: "applied".into(),
                summary,
                paths: plan.files.iter().map(|f| f.path.clone()).collect(),
                count: plan.change_count,
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
                count: plan.change_count,
            });
            self.commit(next)?;
            return Err(error);
        }
        journal.status = "applied".into();
        // Workspace commit is authoritative; recovery recognizes its transaction ID if this fails.
        let _ = self.save_journal(&journal);
        Ok(())
    }

    pub(super) fn rollback(&mut self, journal_id: &str) -> Result<()> {
        let journal = self.load_journal(journal_id)?;
        if !["applied", "recovery-needed"].contains(&journal.status.as_str()) {
            return Err("该记录无需恢复".into());
        }
        let mut files = vec![];
        for file in &journal.files {
            let target = find_target(self.workspace, &file.target_id)?;
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
                    change_count: 0,
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

    pub(super) fn keep_recovery(&mut self, journal_id: &str) -> Result<()> {
        let mut journal = self.load_journal(journal_id)?;
        if journal.status != "recovery-needed" {
            return Err("该记录没有待解决的恢复问题".into());
        }
        let mut next = self.workspace.clone();
        for file in &journal.files {
            let target = find_target(self.workspace, &file.target_id)?;
            if target.path != file.path {
                return Err("路径已变化，无法重建恢复基线".into());
            }
            let (_, entries) = read_target(&target)?;
            update_bindings(&mut next, &target.id, &entries);
        }
        if let Some(h) = next.history.iter_mut().find(|h| h.id == journal_id) {
            h.status = "recovery-kept".into();
            h.summary = "用户选择保留当前磁盘文件；已重建基线，后续变更仍需预览".into();
        }
        self.commit(next)?;
        journal.status = "recovery-kept".into();
        self.save_journal(&journal)
    }
}

fn update_bindings(workspace: &mut Workspace, target_id: &str, entries: &BTreeMap<String, Value>) {
    for service in &mut workspace.services {
        // Keep empty bindings: they record a managed entry that has been removed.
        if service.targets.iter().any(|id| id == target_id)
            || service.bindings.contains_key(target_id)
        {
            service.bindings.insert(
                target_id.into(),
                Binding {
                    raw: entries.get(&service.key).cloned(),
                },
            );
        }
    }
}

//! 目标文件、备份日志和工作区历史共同构成恢复协议。
//! 正常顺序：prepared 日志落盘 → 逐文件校验写入 → 工作区提交 → 日志标记 applied。
//! 崩溃恢复以工作区中的已提交事务 ID 为依据，不能仅凭 prepared 日志就回退文件。

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
/// 持久化事务日志，包含恢复所需的完整文本；仅保存在受限权限的备份目录。
pub struct Journal {
    pub id: String,
    /// prepared 表示写入待确认；applied、recovered、recovery-needed 等状态必须兼容已有备份。
    pub status: String,
    pub files: Vec<FileEdit>,
    pub count: usize,
}

/// 以私有文件权限原子替换工作区 JSON，不自行推进修订号。
pub(super) fn save_workspace(workspace: &Workspace, data_dir: &Path) -> Result<()> {
    storage::atomic_write(
        &data_dir.join("workspace.json"),
        &serde_json::to_string_pretty(workspace).map_err(|error| error.to_string())?,
        true,
    )
}

/// 递增一次修订并持久化，失败时恢复内存中的旧状态并返回错误。
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

/// 借用同一工作区与数据目录执行事务，不创建第二份独立引擎状态。
pub(super) struct Transaction<'a> {
    workspace: &'a mut Workspace,
    data_dir: &'a Path,
}

impl<'a> Transaction<'a> {
    /// 绑定当前工作区；提交与恢复期间复用调用方持有的锁。
    pub(super) fn new(workspace: &'a mut Workspace, data_dir: &'a Path) -> Self {
        Self {
            workspace,
            data_dir,
        }
    }

    /// 复用普通编辑的提交规则，让恢复和文件写入也维护同一修订序列。
    fn commit(&mut self, next: Workspace) -> Result<()> {
        commit_workspace(self.workspace, self.data_dir, next)
    }

    /// 先验证 UUID，再拼接日志路径，避免将调用方提供的 ID 解释成任意路径。
    fn journal_path(&self, journal_id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(journal_id).map_err(|_| "无效的记录 ID")?;
        Ok(self
            .data_dir
            .join("backups")
            .join(format!("{journal_id}.json")))
    }

    /// 完整备份使用私有权限保存；prepared 状态必须先于目标写入落盘。
    fn save_journal(&self, journal: &Journal) -> Result<()> {
        storage::atomic_write(
            &self.journal_path(&journal.id)?,
            &serde_json::to_string_pretty(journal).map_err(|e| e.to_string())?,
            true,
        )
    }

    /// 按记录 ID 读取受大小限制的备份；损坏日志报错，不能猜测原始文件内容。
    fn load_journal(&self, journal_id: &str) -> Result<Journal> {
        let path = self.journal_path(journal_id)?;
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
            return Err("备份文件过大".into());
        }
        serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|_| "恢复记录无法读取".into())
    }

    /// 按写入的反向顺序恢复；已是原始值则跳过，后来外部改动则保留并汇总失败。
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

    /// 启动时处理未完成事务并补齐恢复历史；最终工作区由 Engine::open 统一保存。
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
                // 可能在保存 recovery-needed 日志后、工作区历史落盘前崩溃，需要补回入口。
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
            // 历史已提交为 applied，说明文件及绑定已发布；仅补写日志，不能误回滚。
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

    /// 执行带备份的多文件写入；失败时尝试反向恢复，并记录仍需人工处理的文件。
    /// 文件替换逐个进行，并非跨文件系统事务，因此中断必须依赖日志恢复。
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
        // 先检查所有文件，再开始任何写入；每个 write_checked 仍会在写前再次核对。
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
            // 目标替换或工作区提交失败都走同一补偿路径；冲突文件留给恢复界面处理。
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
        // 工作区提交已是成功依据；此处补写日志失败时，重启可凭相同事务 ID 确认完成。
        let _ = self.save_journal(&journal);
        Ok(())
    }

    /// 确认目标路径和当前文本仍匹配记录后，用新事务写回旧文本。
    /// 已经恢复的文件可跳过，发生后续修改的文件不能强制覆盖。
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

    /// 对 recovery-needed 记录采用当前磁盘基线，保留服务库期望值供后续预览。
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

/// 只更新与此目标有关的服务基线；不修改服务配置或期望分配列表。
fn update_bindings(workspace: &mut Workspace, target_id: &str, entries: &BTreeMap<String, Value>) {
    for service in &mut workspace.services {
        // raw=None 的绑定仍有意义：它记录该托管条目已不存在，不能直接删除此基线。
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

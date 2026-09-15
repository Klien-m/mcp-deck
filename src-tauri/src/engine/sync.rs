//! 同步协调：读取目标快照、缓存当前计划，并通过统一事务入口应用或恢复。
//! 显示脱敏只影响 Preview，实际文件计划始终保留完整原文与完整期望值。

use super::{planner, transaction, Engine, Plan, Preview};
use crate::model::*;

impl Engine {
    /// 生成默认脱敏预览；成功计算新草稿后替换当前计划。
    pub fn preview(&mut self) -> Result<Preview> {
        self.preview_values(false)
    }

    /// 一次计算生成可本地切换的两份显示内容，避免切换明文时重建计划。
    pub fn preview_with_details(&mut self) -> Result<Preview> {
        // 先复制同一份完整差异再脱敏，保证两种显示模式共享计划 ID 和磁盘快照。
        let mut preview = self.preview_values(true)?;
        preview.full_changes = Some(preview.changes.clone());
        for change in &mut preview.changes {
            change.before = change.before.as_ref().map(redact);
            change.after = change.after.as_ref().map(redact);
        }
        Ok(preview)
    }

    /// 基于当前修订生成新计划 ID；草稿有错误或冲突时清除可应用计划。
    /// reveal 仅决定返回内容是否脱敏，不改变实际写入文本。
    pub fn preview_values(&mut self, reveal: bool) -> Result<Preview> {
        let snapshots = self
            .workspace
            .targets
            .iter()
            .filter(|target| {
                planner::relevant_services(&self.workspace, target)
                    .next()
                    .is_some()
            })
            .map(|target| planner::TargetSnapshot {
                target,
                contents: self.read_target(target),
            })
            .collect();
        let planner::Draft {
            mut changes,
            errors,
            files,
        } = planner::build(&self.workspace, snapshots)?;
        let plan_id = id();
        let file_count = files.len();
        if errors.is_empty() && !changes.iter().any(|change| change.conflict) {
            self.plan = Some(Plan {
                id: plan_id.clone(),
                revision: self.workspace.revision,
                files,
                change_count: changes.len(),
            });
        } else {
            self.plan = None;
        }
        if !reveal {
            for change in &mut changes {
                change.before = change.before.as_ref().map(redact);
                change.after = change.after.as_ref().map(redact);
            }
        }
        Ok(Preview {
            id: plan_id,
            changes,
            full_changes: None,
            errors,
            file_count,
        })
    }

    /// 统一执行事务，并按实际修订变化使缓存失效，不只根据 Result 判断是否提交过。
    pub(super) fn transact(
        &mut self,
        operation: impl FnOnce(&mut transaction::Transaction<'_>) -> Result<()>,
    ) -> Result<()> {
        let revision = self.workspace.revision;
        let result = operation(&mut transaction::Transaction::new(
            &mut self.workspace,
            &self.data_dir,
        ));
        // 后续日志更新可能在工作区提交后失败；只要修订已变化，旧预览就必须失效。
        if self.workspace.revision != revision {
            self.plan = None;
        }
        result
    }

    /// 消费当前计划并校验 ID 与修订；执行失败后也需重新预览，不能重用旧计划。
    pub fn apply(&mut self, plan_id: &str) -> Result<()> {
        let plan = self.plan.take().ok_or("预览已失效，请重新预览")?;
        if plan.id != plan_id || plan.revision != self.workspace.revision {
            return Err("服务库在预览后发生变化，请重新预览".into());
        }
        self.transact(|transaction| {
            transaction.execute(plan, "已写入配置；请在目标工具中刷新或授权".into())
        })
    }

    /// 以新的反向事务恢复记录中的原文件，保留服务库的期望配置。
    pub fn rollback(&mut self, journal_id: &str) -> Result<()> {
        self.transact(|transaction| transaction.rollback(journal_id))
    }

    /// 接受未完成恢复后的磁盘现状并重建绑定基线，不覆盖当前文件。
    pub fn keep_recovery(&mut self, journal_id: &str) -> Result<()> {
        self.transact(|transaction| transaction.keep_recovery(journal_id))
    }
}

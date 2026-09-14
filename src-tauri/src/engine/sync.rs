use super::{planner, transaction, Engine, Plan, Preview};
use crate::model::*;

impl Engine {
    pub fn preview(&mut self) -> Result<Preview> {
        self.preview_values(false)
    }

    pub fn preview_with_details(&mut self) -> Result<Preview> {
        // Both display modes must describe the same plan and disk snapshot.
        let mut preview = self.preview_values(true)?;
        preview.full_changes = Some(preview.changes.clone());
        for change in &mut preview.changes {
            change.before = change.before.as_ref().map(redact);
            change.after = change.after.as_ref().map(redact);
        }
        Ok(preview)
    }

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

    pub(super) fn transact(
        &mut self,
        operation: impl FnOnce(&mut transaction::Transaction<'_>) -> Result<()>,
    ) -> Result<()> {
        let revision = self.workspace.revision;
        let result = operation(&mut transaction::Transaction::new(
            &mut self.workspace,
            &self.data_dir,
        ));
        // A transaction may publish a workspace before a later journal update fails.
        if self.workspace.revision != revision {
            self.plan = None;
        }
        result
    }

    pub fn apply(&mut self, plan_id: &str) -> Result<()> {
        let plan = self.plan.take().ok_or("预览已失效，请重新预览")?;
        if plan.id != plan_id || plan.revision != self.workspace.revision {
            return Err("服务库在预览后发生变化，请重新预览".into());
        }
        self.transact(|transaction| {
            transaction.execute(plan, "已写入配置；请在目标工具中刷新或授权".into())
        })
    }

    pub fn rollback(&mut self, journal_id: &str) -> Result<()> {
        self.transact(|transaction| transaction.rollback(journal_id))
    }

    pub fn keep_recovery(&mut self, journal_id: &str) -> Result<()> {
        self.transact(|transaction| transaction.keep_recovery(journal_id))
    }
}

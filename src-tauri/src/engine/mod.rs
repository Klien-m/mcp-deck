//! 工作区用例门面：管理加载、进程锁和原子提交，将领域变更委托给子模块。
//! 普通编辑只更新服务库；目标文件写入必须经过 sync / transaction 的预览与事务流程。

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
pub use types::{Adoption, Discovery, FileEdit, Preview, Snapshot, TargetDiscovery, TargetStatus};

use crate::{adapters, model::*, storage};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

/// 一个打开的工作区及其当前预览计划。桌面宿主通过 Mutex 串行调用。
/// 公开 workspace 供读取和测试使用；业务修改应通过用例入口以维护修订和计划失效规则。
pub struct Engine {
    pub workspace: Workspace,
    pub data_dir: PathBuf,
    pub home: PathBuf,
    // 缓存最近一次无错误、无冲突的计划；应用时还需确认存在实际文件变更。
    plan: Option<Plan>,
    // 保留文件句柄以持有跨进程排他锁，Engine 被释放后锁随句柄关闭而释放。
    _lock: File,
}

impl Engine {
    /// 创建私有目录并加锁，加载现有版本，再恢复中断事务并保存恢复结果。
    /// 损坏或不兼容的工作区直接报错，不用空服务库覆盖原文件。
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
                onboarding_complete: false,
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
        let mut next = engine.workspace.clone();
        if adapters::migrate_default_targets(&mut next, &engine.home) {
            engine.commit(next)?;
        }
        engine.save()?;
        Ok(engine)
    }

    /// 保存当前状态而不增加修订号，用于初始化及启动恢复结果落盘。
    fn save(&self) -> Result<()> {
        transaction::save_workspace(&self.workspace, &self.data_dir)
    }

    /// 成功落盘后清除旧计划；失败时保留原工作区与可用计划。
    fn commit(&mut self, next: Workspace) -> Result<()> {
        transaction::commit_workspace(&mut self.workspace, &self.data_dir, next)?;
        self.plan = None;
        Ok(())
    }

    /// 在副本上执行完整用例，成功后统一提交；批量操作中途失败不会泄露部分变更。
    fn update<T>(&mut self, operation: impl FnOnce(&mut Workspace) -> Result<T>) -> Result<T> {
        let mut next = self.workspace.clone();
        let result = operation(&mut next)?;
        self.commit(next)?;
        Ok(result)
    }

    /// 按稳定目标 ID 查找配置，路径显示名称不参与身份判断。
    fn target(&self, id: &str) -> Result<Target> {
        find_target(&self.workspace, id)
    }

    /// 读取一个目标的原始文本和解析条目，保留“文件不存在”的独立状态。
    fn read_target(&self, target: &Target) -> Result<(Option<String>, BTreeMap<String, Value>)> {
        read_target(target)
    }

    /// 读取服务库与各目标的磁盘概况；单个目标读取失败记录在其 error 中。
    /// 此结果不代表客户端已加载、授权或成功连接服务。
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
                    warning: adapters::legacy_default_path_notice(t, &self.home),
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

    /// 创建或编辑服务，返回稳定服务 ID；不直接修改已分配目标文件。
    pub fn save_service(&mut self, input: ServiceInput) -> Result<String> {
        self.update(|workspace| services::save(workspace, input))
    }

    /// 暂存分配或取消分配；目标文件与旧绑定基线留待预览和应用处理。
    pub fn assign(&mut self, service_id: &str, target_id: &str, enabled: bool) -> Result<()> {
        self.update(|workspace| services::assign(workspace, service_id, target_id, enabled))
    }

    /// 软删除或撤销软删除，保留服务及绑定以生成可恢复的移除计划。
    pub fn remove(&mut self, service_id: &str, undo: bool) -> Result<()> {
        self.update(|workspace| services::remove(workspace, service_id, undo))
    }

    /// 只读列出指定目标的条目，逐项报告可解码性和已有管理状态。
    pub fn discover(&self, target_id: &str) -> Result<Vec<Discovery>> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        discovery::discover(&self.workspace, &target, entries)
    }

    /// 查询全部登记路径，返回有 MCP 条目或读取错误的工具；不写入任何状态。
    pub fn discover_all(&self) -> Vec<TargetDiscovery> {
        discovery::discover_all(&self.workspace)
    }

    /// 所选工具的纳管与引导状态一次提交；空选择表示跳过。
    pub fn complete_onboarding(&mut self, selections: Vec<Adoption>) -> Result<()> {
        self.update(|workspace| discovery::complete_onboarding(workspace, selections))
    }

    /// 重新读取磁盘后按键纳入管理；整个批次成功后才提交，不改来源文件。
    pub fn adopt(&mut self, target_id: &str, keys: Vec<String>) -> Result<()> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        self.update(|workspace| discovery::adopt(workspace, &target, &entries, keys))
    }

    /// 批量导入指定适配器格式，返回新增数量；导入项暂不关联目标。
    pub fn import_text(&mut self, adapter_id: &str, text: &str) -> Result<usize> {
        self.update(|workspace| discovery::import_text(workspace, adapter_id, text))
    }

    /// 校验并保存目标配置；有关联条目的目标不能直接迁移路径或适配器。
    pub fn save_target(&mut self, target: Target) -> Result<()> {
        let data_dir = self.data_dir.clone();
        self.update(|workspace| targets::save(workspace, &data_dir, target))
    }

    /// 确认冲突基线；use_disk 同时采用磁盘配置，否则保留服务库期望值。
    /// 该选择只改变服务库及基线，仍需重新预览后才会写目标文件。
    pub fn resolve(&mut self, service_id: &str, target_id: &str, use_disk: bool) -> Result<()> {
        let target = self.target(target_id)?;
        let (_, entries) = self.read_target(&target)?;
        self.update(|workspace| {
            services::resolve(workspace, &target, &entries, service_id, use_disk)
        })
    }

    /// 生成指定目标格式的文本；空 ID 列表表示全部未删除服务。
    pub fn export(
        &self,
        adapter_id: &str,
        service_ids: &[String],
        include_secrets: bool,
    ) -> Result<String> {
        transfer::export(&self.workspace, adapter_id, service_ids, include_secrets)
    }

    /// 导出到独立文件，拒绝覆盖工作区数据目录或已管理目标；不替换预览。
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

    /// 检查配置字段和命令位置，不启动进程、不执行网络连通性测试。
    pub fn checks(&self, service_id: &str) -> Result<Value> {
        diagnostics::checks(&self.workspace, &self.home, service_id)
    }
}

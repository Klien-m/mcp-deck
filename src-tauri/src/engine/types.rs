//! 引擎对外响应及内部计划类型；camelCase / flatten 等序列化约定与前端 types.ts 对齐。

use crate::{adapters::Adapter, model::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 目标读取概况；Target 字段在 IPC JSON 中平铺，便于前端按 ID 使用。
pub struct TargetStatus {
    #[serde(flatten)]
    pub target: Target,
    /// 路径当前是否为文件；存在不代表其内容能解析或服务可运行。
    pub exists: bool,
    /// 成功解析的全部服务条目数，包含未托管项；失败时为 0，需同时检查 error。
    pub count: usize,
    pub error: Option<String>,
    /// 路径兼容提示不等于读取失败，保留成功解析的计数和状态。
    pub warning: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 服务库、适配器与目标磁盘概况；包含完整配置，只应在本机可信界面使用。
pub struct Snapshot {
    pub workspace: Workspace,
    pub adapters: Vec<Adapter>,
    pub targets: Vec<TargetStatus>,
    pub data_dir: String,
    /// 由显式 MCP_DECK_HOME 环境变量标记，用于显示隔离工作区提示。
    pub isolated: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 单项发现结果；解码失败仍保留脱敏预览，避免不支持项在界面中消失。
pub struct Discovery {
    pub key: String,
    /// 仅成功解码时存在，可能含完整凭据；preview 才是脱敏展示值。
    pub config: Option<Config>,
    pub preview: Value,
    pub error: Option<String>,
    /// 该键在目标上已有非空绑定；纳入时还会重新检查分配占用与最新磁盘内容。
    pub managed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 聚合发现按工具分组；单个文件读取失败不阻断其他工具。
pub struct TargetDiscovery {
    pub target: Target,
    pub items: Vec<Discovery>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
/// 用户明确选择的来源与配置键；提交时重新读取来源文件。
pub struct Adoption {
    pub target_id: String,
    pub keys: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 事务级完整文件前后文本，用于乐观并发校验、备份和精确恢复。
pub struct FileEdit {
    pub target_id: String,
    pub path: String,
    /// None 表示文件原本不存在；Some(空串) 是真实存在的空文件。
    pub before: Option<String>,
    /// None 表示恢复为文件不存在；普通配置移除只编辑服务节点，通常仍保留文件。
    pub after: Option<String>,
}
/// 内部文件计划，不随工作区持久化；Engine 的同步缓存及反向恢复事务共用此结构。
pub(super) struct Plan {
    pub(super) id: String,
    /// 生成计划时的工作区修订，应用前必须与当前修订一致。
    pub(super) revision: u64,
    pub(super) files: Vec<FileEdit>,
    pub(super) change_count: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 面向界面的差异响应；有错误或冲突时仍可展示，但无可执行计划。
pub struct Preview {
    /// 本次预览 ID；仅在后端保留同一可执行计划时可以用于 apply。
    pub id: String,
    /// 默认接口及详细双份接口为脱敏值；兼容 reveal=true 的单份接口可为完整值。
    pub changes: Vec<Change>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// 仅详细预览提供，与 changes 同源同序同计划；前端切换显示无需再次请求。
    pub full_changes: Option<Vec<Change>>,
    /// 阻断本次应用的读取、兼容性、重复键或未处理恢复问题。
    pub errors: Vec<String>,
    /// 计划草稿涉及的文件数，与服务条目变更数独立。
    pub file_count: usize,
}

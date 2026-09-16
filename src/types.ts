/** 前端使用的持久化数据与响应形状；字段对应 Rust model.rs 和 engine/types.rs 的 JSON 表示。 */
/** 公共传输类型，原生工具可能使用不同字段名或 type 字符串。 */
export type Transport = "stdio" | "http" | "sse";
/** 服务库完整期望配置；stdio 与远程字段互斥，不能直接将脱敏预览作为配置保存。 */
export interface Config {
  transport: Transport;
  command: string;
  args: string[];
  cwd: string;
  env: Record<string, string>;
  url: string;
  headers: Record<string, string>;
}
/** 带稳定内部 ID 的服务；不同来源允许同名配置键，目标占用由后端校验。 */
export interface Service {
  id: string;
  /** 目标文件内的配置键，编辑已有服务时不可修改。 */
  key: string;
  name: string;
  description: string;
  config: Config;
  /** 期望分配的目标 ID，只有应用同步后才反映到目标文件。 */
  targets: string[];
  /** 按目标 ID 保存的确认基线；raw 可为 null，表示已确认条目不存在。 */
  bindings: Record<string, { raw: unknown }>;
  /** 按适配器 ID 保存来源原生条目，以保留未进入公共模型的专属字段。 */
  native: Record<string, unknown>;
  /** 软删除项隐藏于服务库列表，但仍参与目标移除和撤销处理。 */
  deleted: boolean;
}
/** 只包含用户可编辑字段，不允许通过表单直接修改分配、基线或历史。 */
export interface ServiceInput {
  /** null 表示创建，字符串表示编辑现有服务。 */
  id: string | null;
  key: string;
  name: string;
  description: string;
  config: Config;
}
/** 适配器展示信息与写入能力；同一适配器可以拥有多个配置目标。 */
export interface Adapter {
  id: string;
  name: string;
  rootKey: string;
  format: string;
  transports: Transport[];
  supportsCwd: boolean;
  docs: string;
  note: string;
}
/** 一个明确的配置文件及其适配器，不代表客户端完整的继承配置。 */
export interface Target {
  id: string;
  adapterId: string;
  name: string;
  path: string;
}
/** 目标读取概况；exists 不等于成功解析，count 为 0 时也需检查 error。 */
export interface TargetStatus extends Target {
  exists: boolean;
  count: number;
  error: string | null;
  /** 旧路径等兼容提示；不代表配置读取或解析失败。 */
  warning?: string | null;
}
/** 与持久化日志兼容的状态；仅 recovery-needed 阻断新同步并允许选择保留磁盘。 */
export type HistoryStatus =
  | "applied"
  | "recovered"
  | "recovery-needed"
  | "rolled-back"
  | "recovery-kept";
/** 工作区中的事务摘要，完整前后文件文本由后端备份日志持有。 */
export interface History {
  id: string;
  /** Unix 秒数，传给 JavaScript Date 前需乘以 1000。 */
  at: number;
  status: HistoryStatus;
  summary: string;
  paths: string[];
  /** 服务条目变更数；反向恢复事务可为 0，不等于 paths.length。 */
  count: number;
}
/** 加载界面所需的完整快照；workspace.targets 是登记信息，targets 附加最新读取概况。 */
export interface Snapshot {
  workspace: {
    /** 持久化格式版本，不随每次保存递增。 */
    version: number;
    /** 工作区提交序号，用于后端拒绝过期计划。 */
    revision: number;
    /** 新工作区完成或跳过首次引导后持久化为 true。 */
    onboardingComplete: boolean;
    services: Service[];
    targets: Target[];
    history: History[];
  };
  adapters: Adapter[];
  targets: TargetStatus[];
  dataDir: string;
  isolated: boolean;
}
/** 服务条目级操作类型，与后端 Change.action 对齐。 */
export type ChangeAction = "add" | "update" | "remove";
/** 单个服务在单个目标中的差异；原生值按适配器不同，作为展示数据处理。 */
export interface Change {
  serviceId: string;
  targetId: string;
  targetName: string;
  key: string;
  action: ChangeAction;
  /** 本次磁盘条目；null 表示不存在，不能据此推断磁盘基线是否已确认。 */
  before: unknown;
  /** 期望写入的条目；null 表示移除该键。 */
  after: unknown;
  /** 当前磁盘值不同于上次确认基线，必须先做显式选择。 */
  conflict: boolean;
  message: string;
}
/** 一次预览对应一个计划 ID；展示内容存在不代表该计划可应用。 */
export interface Preview {
  id: string;
  /** 应用使用的默认／详细接口返回脱敏差异；兼容 reveal=true 调用可能返回完整值。 */
  changes: Change[];
  /** 详细预览才提供，与 changes 同计划、同顺序；显示切换无需 IPC。 */
  fullChanges?: Change[];
  /** 阻断本次应用的问题；服务冲突另见 changes 中的 conflict。 */
  errors: string[];
  /** 涉及文件数，不是服务变更数。 */
  fileCount: number;
}
/** 单项发现结果；config=null 表示无法接管，原条目仍在源文件中保留。 */
export interface Discovery {
  key: string;
  /** 解码成功时包含完整配置，展示摘要请使用已脱敏的 preview。 */
  config: Config | null;
  preview: unknown;
  error: string | null;
  managed: boolean;
}
/** 聚合扫描只返回有 MCP 或读取错误的工具。 */
export interface TargetDiscovery {
  target: Target;
  items: Discovery[];
  error: string | null;
}
export interface Adoption {
  targetId: string;
  keys: string[];
}
/** 静态诊断结果，不能用作 MCP 加载成功或服务连通性的判断。 */
export interface Checks {
  issues: string[];
  executable: string | null;
  note: string;
  /** 逐项静态结果；旧版本后端可只返回 issues。ok 也不表示已连接。 */
  items?: {
    code: string;
    level: "ok" | "warning" | "error";
    message: string;
    hint?: string;
  }[];
}

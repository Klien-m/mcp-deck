/** 类型化 Tauri IPC 边界；命令名、字段和结果须与 Rust commands::Request / dispatch 同步维护。 */
import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  Adoption,
  Checks,
  Discovery,
  Preview,
  ServiceInput,
  Snapshot,
  Target,
  TargetDiscovery,
} from "./types";

/**
 * 按命令关联参数与返回类型，避免调用方手写任意返回类型。
 * 此映射仅提供编译期约束；运行时输入仍由 Rust 反序列化和用例校验。
 */
export interface CommandMap {
  /** 只读获取服务库及目标概况，配置包含完整本地值。 */
  snapshot: { args: Record<string, never>; result: Snapshot };
  /** 创建／编辑服务库条目，成功返回服务 ID；不会直接写入目标文件。 */
  saveService: { args: { input: ServiceInput }; result: string };
  assign: {
    args: { serviceId: string; targetId: string; enabled: boolean };
    result: null;
  };
  /** undo=false 软删除，true 撤销软删除，目标文件变更另行应用。 */
  remove: { args: { serviceId: string; undo: boolean }; result: null };
  /** 只读发现；不支持项仍返回错误与脱敏预览。 */
  discover: { args: { targetId: string }; result: Discovery[] };
  discoverAll: { args: Record<string, never>; result: TargetDiscovery[] };
  /** 多工具纳管与完成标记原子保存；空选择表示跳过。 */
  completeOnboarding: { args: { selections: Adoption[] }; result: null };
  /** 按当前磁盘键批量纳入管理并建立绑定，成功返回 null。 */
  adopt: { args: { targetId: string; keys: string[] }; result: null };
  /** 批量导入指定格式的文本，返回新增数量；不会自动分配目标。 */
  importText: { args: { adapterId: string; text: string }; result: number };
  saveTarget: { args: { target: Target }; result: null };
  /** 会替换后端当前计划。includeDetails 返回双份同源差异；reveal 为单份接口兼容选项。 */
  preview: {
    args: { reveal?: boolean; includeDetails?: boolean };
    result: Preview;
  };
  /** id 必须匹配当前计划；应用前还检查工作区修订与目标原文。 */
  apply: { args: { id: string }; result: null };
  /** useDisk 决定期望配置是否采用磁盘值；两种选择都确认新基线并使旧计划失效。 */
  resolve: {
    args: { serviceId: string; targetId: string; useDisk: boolean };
    result: null;
  };
  /** 以历史记录 ID 恢复原始文件，保留服务库当前期望配置。 */
  rollback: { args: { id: string }; result: null };
  /** 对未完成恢复记录采用当前磁盘作为基线，不覆盖目标文件。 */
  keepRecovery: { args: { id: string }; result: null };
  /** 只生成文本；空 serviceIds 表示全部未删除服务，includeSecrets 控制脱敏。 */
  export: {
    args: { adapterId: string; serviceIds: string[]; includeSecrets: boolean };
    result: string;
  };
  /** 重新生成并保存独立导出文件；后端禁止覆盖服务库目录和托管目标。 */
  saveExport: {
    args: {
      adapterId: string;
      serviceIds: string[];
      includeSecrets: boolean;
      path: string;
    };
    result: null;
  };
  /** 只检查字段及命令文件位置，不启动进程或测试网络连接。 */
  checks: { args: { serviceId: string }; result: Checks };
}

export type Command = keyof CommandMap;
export type CommandArgs<K extends Command> = CommandMap[K]["args"];
export type CommandResult<K extends Command> = CommandMap[K]["result"];
/** 浏览器开发预览不具备本机文件管理能力，实际 IPC 仅在 Tauri 宿主内执行。 */
export const native = isTauri();
/**
 * 发送一个命令并保留参数／结果推导；只有 snapshot 和 preview 允许省略参数。
 * 写入调用应统一经过 useWorkspace，避免绕开操作锁、错误展示和刷新。
 */
export function request<K extends "snapshot" | "preview">(
  op: K,
  args?: CommandArgs<K>,
): Promise<CommandResult<K>>;
export function request<K extends Command>(
  op: K,
  args: CommandArgs<K>,
): Promise<CommandResult<K>>;
export async function request(
  op: Command,
  args: CommandArgs<Command> = {},
): Promise<CommandResult<Command>> {
  if (!native)
    throw new Error(
      "请在 MCP Deck 桌面应用内使用真实配置管理。浏览器仅用于开发预览。",
    );
  return invoke("dispatch", { request: { ...args, op } });
}

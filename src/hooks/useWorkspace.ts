/** 工作区数据与操作协调层：组件通过具体 action 修改配置，共享忙碌、错误和刷新状态。 */
import { useCallback, useEffect, useRef, useState } from "react";
import { request } from "../api";
import type { CommandArgs, CommandResult } from "../api";
import type { Adoption, Preview, ServiceInput, Snapshot, Target } from "../types";

/** 可产生持久化副作用的命令集合；只读查询和预览有独立调用路径。 */
type Mutation =
  | "saveService"
  | "saveTarget"
  | "assign"
  | "remove"
  | "adopt"
  | "completeOnboarding"
  | "importText"
  | "apply"
  | "resolve"
  | "rollback"
  | "keepRecovery"
  | "saveExport";
/** ok=false 既可能是忙碌时拒绝，也可能是操作失败；错误由统一状态展示。 */
type Outcome<T> = { ok: true; value: T } | { ok: false };

/**
 * 加载完整快照与同步预览，向上层暴露业务动作。
 * includeDetails 由同步弹窗是否打开决定；数据变更后保持相应详细程度。
 */
export function useWorkspace({
  includeDetails,
  onNotice,
}: {
  includeDetails: boolean;
  onNotice: (message: string) => void;
}) {
  const [data, setData] = useState<Snapshot | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  // fatal 记录启动失败，error 记录交互失败；存在旧数据时仍保留可见界面。
  const [fatal, setFatal] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  // ref 同步设锁，避免同一渲染内的连点绕过异步更新的 busy。
  const operation = useRef(false);
  // 递增代次只抑制过期结果与后续请求，不会取消已经发往后端的 IPC。
  const refreshId = useRef(0);

  /** 先读快照再生成预览，两者都成功且仍为最新请求时才一起发布。 */
  const refresh = useCallback(async (details = false) => {
    const id = ++refreshId.current;
    const snapshot = await request("snapshot");
    // 预览会替换后端内存计划，已过期的快照请求不得继续触发预览。
    if (id !== refreshId.current) return;
    const nextPreview = await request("preview", { includeDetails: details });
    if (id === refreshId.current) {
      setData(snapshot);
      setPreview(nextPreview);
    }
  }, []);

  useEffect(() => {
    let active = true;
    refresh().catch((e) => active && setFatal(String(e)));
    // 清理兼容卸载及 StrictMode 的再次初始化，防止旧结果覆盖当前界面。
    return () => {
      active = false;
      refreshId.current++;
    };
  }, [refresh]);

  /**
   * 编辑器、同步操作和导出共享同步入口锁；占用期间直接拒绝，不排队重放写操作。
   * 只有整个 work 成功才通知，finally 始终释放锁，确保失败后可以重试。
   */
  async function run<T>(
    work: () => Promise<T>,
    message?: string,
  ): Promise<Outcome<T>> {
    if (operation.current) return { ok: false };
    operation.current = true;
    refreshId.current++;
    setBusy(true);
    setError("");
    try {
      const value = await work();
      if (message) onNotice(message);
      return { ok: true, value };
    } catch (e) {
      setError(String(e));
      return { ok: false };
    } finally {
      operation.current = false;
      setBusy(false);
    }
  }

  /**
   * 先执行命令再刷新展示。刷新失败也会返回失败，但不能据此认定之前的写入已回滚。
   * 因此这里不自动重试命令，避免新建或导入被重复执行。
   */
  function mutate<K extends Mutation>(
    op: K,
    args: CommandArgs<K>,
    message: string,
  ) {
    return run<CommandResult<K>>(async () => {
      const value = await request(op, args);
      // 独立导出不改工作区，无需刷新或替换当前同步计划。
      if (op !== "saveExport") await refresh(includeDetails);
      return value;
    }, message);
  }

  // 对外返回具体业务方法；保存服务返回 ID，其余动作返回是否完成，供弹窗决定是否关闭。
  const actions = {
    /** 保存并刷新后返回服务 ID；失败或被锁拒绝返回 null，保留编辑器草稿。 */
    async saveService(input: ServiceInput) {
      const result = await mutate("saveService", { input }, "服务已保存");
      return result.ok ? result.value : null;
    },
    async saveTarget(target: Target) {
      return (await mutate("saveTarget", { target }, "目标路径已保存")).ok;
    },
    async assign(serviceId: string, targetId: string, enabled: boolean) {
      return (
        await mutate(
          "assign",
          { serviceId, targetId, enabled },
          "分配已暂存，预览后写入",
        )
      ).ok;
    },
    async remove(serviceId: string, undo = false) {
      return (
        await mutate(
          "remove",
          { serviceId, undo },
          undo ? "已恢复服务" : "已移除服务；目标文件变更将在应用后生效",
        )
      ).ok;
    },
    async adopt(targetId: string, keys: string[]) {
      return (
        await mutate("adopt", { targetId, keys }, "已纳入服务库，保留原配置")
      ).ok;
    },
    async completeOnboarding(selections: Adoption[]) {
      return (
        await mutate(
          "completeOnboarding",
          { selections },
          selections.length
            ? "已纳入所选 MCP，保留各工具原配置"
            : "可随时通过「发现本机配置」纳入已有 MCP",
        )
      ).ok;
    },
    async importText(adapterId: string, text: string) {
      return (
        await mutate(
          "importText",
          { adapterId, text },
          "已导入服务库，请按需分配",
        )
      ).ok;
    },
    async apply(id: string) {
      return (
        await mutate("apply", { id }, "配置已写入，请在目标工具中刷新或授权")
      ).ok;
    },
    async resolve(serviceId: string, targetId: string, useDisk: boolean) {
      return (
        await mutate(
          "resolve",
          { serviceId, targetId, useDisk },
          useDisk
            ? "已采用磁盘版本；其他目标变更请重新检查"
            : "已确认保留服务库版本，请检查新的预览",
        )
      ).ok;
    },
    async rollback(id: string) {
      return (await mutate("rollback", { id }, "已恢复文件，服务库期望值保留"))
        .ok;
    },
    async keepRecovery(id: string) {
      return (
        await mutate("keepRecovery", { id }, "已保留磁盘现状，并重建同步基线")
      ).ok;
    },
    async saveExport(options: CommandArgs<"saveExport">) {
      return (await mutate("saveExport", options, "配置已导出")).ok;
    },
    /** 手动重新读取磁盘状态；启动错误可经同一入口重新尝试加载。 */
    async reload() {
      setFatal("");
      const result = await run(() => refresh(includeDetails), "已重新读取配置");
      return result.ok;
    },
    /** 打开预览前获取同计划的完整与脱敏数据，持锁期间阻止其他写操作。 */
    async loadPreview() {
      return (
        await run(async () => {
          const id = refreshId.current;
          const nextPreview = await request("preview", {
            includeDetails: true,
          });
          if (id === refreshId.current) setPreview(nextPreview);
        })
      ).ok;
    },
  };

  return {
    data,
    preview,
    fatal,
    error,
    busy,
    clearError: () => setError(""),
    actions,
  };
}

/** 由 Hook 实际返回值推导动作契约，避免组件另行维护宽泛的命令执行接口。 */
export type WorkspaceActions = ReturnType<typeof useWorkspace>["actions"];

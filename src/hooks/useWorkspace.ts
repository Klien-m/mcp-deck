import { useCallback, useEffect, useRef, useState } from "react";
import { request } from "../api";
import type { CommandArgs, CommandResult } from "../api";
import type { Preview, ServiceInput, Snapshot, Target } from "../types";

type Mutation =
  | "saveService"
  | "saveTarget"
  | "assign"
  | "remove"
  | "adopt"
  | "importText"
  | "apply"
  | "resolve"
  | "rollback"
  | "keepRecovery"
  | "saveExport";
type Outcome<T> = { ok: true; value: T } | { ok: false };

export function useWorkspace({
  includeDetails,
  onNotice,
}: {
  includeDetails: boolean;
  onNotice: (message: string) => void;
}) {
  const [data, setData] = useState<Snapshot | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [fatal, setFatal] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const operation = useRef(false);
  const refreshId = useRef(0);

  const refresh = useCallback(async (details = false) => {
    const id = ++refreshId.current;
    const snapshot = await request("snapshot");
    // Preview replaces the backend's active plan, so skip superseded refreshes.
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
    return () => {
      active = false;
      refreshId.current++;
    };
  }, [refresh]);

  // Editors, sync operations and file exports share the same synchronous gate.
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

  function mutate<K extends Mutation>(
    op: K,
    args: CommandArgs<K>,
    message: string,
  ) {
    return run<CommandResult<K>>(async () => {
      const value = await request(op, args);
      // Export writes a separate file without changing the workspace or active plan.
      if (op !== "saveExport") await refresh(includeDetails);
      return value;
    }, message);
  }

  const actions = {
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
    async reload() {
      setFatal("");
      const result = await run(() => refresh(includeDetails), "已重新读取配置");
      return result.ok;
    },
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

export type WorkspaceActions = ReturnType<typeof useWorkspace>["actions"];

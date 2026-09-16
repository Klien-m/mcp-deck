import { useEffect, useState } from "react";
import type { Change, Service, TargetStatus } from "../../types";
import { pendingActionNames } from "../sync/labels";

/** 纯界面状态与派生列表：筛选、搜索、当前选择和目标计数；不读写 IPC。 */
export function useServiceLibrary(
  allServices: Service[],
  targets: TargetStatus[],
  changes: Change[],
  planningFailed = false,
) {
  const [selected, setSelected] = useState({ id: "", version: 0 });
  const [filter, setFilter] = useState("all");
  const [query, setQuery] = useState("");

  // 侧栏只展示有分配的目标；当前目标移除或变空后回到“全部”，避免停留在失效筛选。
  useEffect(() => {
    setFilter((current) =>
      current === "all" ||
      current === "pending" ||
      (targets.some((t) => t.id === current) &&
        allServices.some(
          (service) => !service.deleted && service.targets.includes(current),
        ))
        ? current
        : "all",
    );
  }, [allServices, targets]);

  const services = allServices.filter((s) => !s.deleted);
  const toolTargets = targets
    .map((t) => ({
      ...t,
      serviceCount: services.filter((s) => s.targets.includes(t.id)).length,
    }))
    .filter((t) => t.serviceCount > 0);
  // 同一服务可在多个目标有变更，集合按服务去重，而同步按钮仍展示条目级数量。
  const pendingIds = new Set(changes.map((c) => c.serviceId));
  // 读取失败时无法生成移除差异，仍保留有旧绑定的取消分配和软删除项。
  // 成功应用移除后 raw 会更新为 null，因此已完成的删除不会一直留在“待应用”。
  for (const s of allServices) {
    if (Object.entries(s.bindings).some(([id, b]) => b.raw != null && (s.deleted || !s.targets.includes(id)))) {
      pendingIds.add(s.id);
    }
  }
  const visible = (filter === "pending" ? allServices : services).filter(
    (s) =>
      (filter === "all" ||
        (filter === "pending" && pendingIds.has(s.id)) ||
        s.targets.includes(filter)) &&
      `${s.name} ${s.key} ${s.description}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  // 搜索隐藏当前选择时暂用第一条可见项；选择身份保留，清除搜索后可恢复。
  const service = visible.find((s) => s.id === selected.id) || visible[0];
  const status = (s: Service) => {
    const ownChanges = changes.filter((c) => c.serviceId === s.id);
    const unreadable = targets.some(
      (t) => t.error &&
        (s.targets.includes(t.id) || s.bindings[t.id]?.raw != null),
    );
    if (unreadable) return s.deleted ? "待移除 · 读取失败" : "配置读取失败";
    if (ownChanges.some((c) => c.conflict)) {
      return s.deleted ? "待移除 · 配置冲突" : "配置冲突";
    }
    if (s.deleted && pendingIds.has(s.id)) return "待移除";
    if (ownChanges.length) {
      return [...new Set(ownChanges.map((c) => pendingActionNames[c.action]))].join(" / ");
    }
    if (pendingIds.has(s.id)) return "待移除";
    if (planningFailed && (s.targets.length || Object.values(s.bindings).some((b) => b.raw != null))) {
      return "同步状态待确认";
    }
    return s.targets.length ? "配置已对齐" : "尚未分配";
  };

  // App 将 version 纳入详情 key；再次点击同一行也会重置详情标签和检查状态。
  const select = (id: string) =>
    setSelected((current) => ({ id, version: current.version + 1 }));
  /** 新建或编辑成功后清除筛选并定位保存项，避免已保存服务被当前条件隐藏。 */
  function revealSaved(id: string) {
    select(id);
    setFilter("all");
    setQuery("");
  }

  return {
    services,
    toolTargets,
    pendingIds,
    visible,
    service,
    status,
    selected,
    filter,
    query,
    select,
    setFilter,
    setQuery,
    revealSaved,
  };
}

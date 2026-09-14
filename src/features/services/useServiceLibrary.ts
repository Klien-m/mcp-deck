import { useEffect, useState } from "react";
import type { Change, Service, TargetStatus } from "../../types";

export function useServiceLibrary(
  allServices: Service[],
  targets: TargetStatus[],
  changes: Change[],
) {
  const [selected, setSelected] = useState({ id: "", version: 0 });
  const [filter, setFilter] = useState("all");
  const [query, setQuery] = useState("");

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
  const pendingIds = new Set(changes.map((c) => c.serviceId));
  const conflicts = new Set(
    changes.filter((c) => c.conflict).map((c) => c.serviceId),
  );
  const visible = services.filter(
    (s) =>
      (filter === "all" ||
        (filter === "pending" && pendingIds.has(s.id)) ||
        s.targets.includes(filter)) &&
      `${s.name} ${s.key} ${s.description}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  const service = visible.find((s) => s.id === selected.id) || visible[0];
  const status = (s: Service) =>
    conflicts.has(s.id)
      ? "配置冲突"
      : pendingIds.has(s.id)
        ? "待应用"
        : s.targets.length
          ? "配置已对齐"
          : "尚未分配";

  // Reselecting a row also returns its detail panel to the overview tab.
  const select = (id: string) =>
    setSelected((current) => ({ id, version: current.version + 1 }));
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

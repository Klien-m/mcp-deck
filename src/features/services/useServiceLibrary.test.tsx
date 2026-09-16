import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { Change, Service, TargetStatus } from "../../types";
import { useServiceLibrary } from "./useServiceLibrary";

const target: TargetStatus = {
  id: "codex", adapterId: "codex", name: "Codex", path: "/home/test/.codex/config.toml",
  exists: true, count: 1, error: null,
};
const service: Service = {
  id: "git", key: "git", name: "Git", description: "Git 工具",
  config: { transport: "stdio", command: "python3", args: ["-m", "mcp_server_git"], cwd: "", env: {}, url: "", headers: {} },
  targets: [target.id], bindings: { codex: { raw: { command: "python3" } } }, native: {}, deleted: false,
};
const change: Change = {
  serviceId: service.id, targetId: target.id, targetName: target.name, key: service.key,
  action: "remove", before: { command: "python3" }, after: null, conflict: false, message: "",
};

afterEach(cleanup);

describe("服务库同步状态", () => {
  it("软删除项只在待应用中保留，完成目标移除后不再显示", () => {
    const removed = { ...service, deleted: true, targets: [] };
    const { result, rerender } = renderHook(
      ({ services, changes }) => useServiceLibrary(services, [target], changes),
      { initialProps: { services: [removed] as Service[], changes: [change] } },
    );
    expect(result.current.services).toEqual([]);
    expect(result.current.visible).toEqual([]);
    expect(result.current.toolTargets).toEqual([]);
    act(() => result.current.setFilter("pending"));
    expect(result.current.visible.map((s) => s.id)).toEqual([service.id]);
    expect(result.current.status(removed)).toBe("待移除");
    rerender({ services: [{ ...removed, bindings: { codex: { raw: null } } }], changes: [] });
    expect(result.current.visible).toEqual([]);
    expect(result.current.pendingIds.size).toBe(0);
  });

  it("读取失败无法生成计划时仍显示待移除项，不误报配置已对齐", () => {
    const removed = { ...service, deleted: true, targets: [] };
    const broken = { ...target, error: "无法解析 TOML" };
    const active = { ...service, id: "other-service" };
    const { result } = renderHook(() => useServiceLibrary([removed, active], [broken], []));
    act(() => result.current.setFilter("pending"));
    expect(result.current.visible).toEqual([removed]);
    expect(result.current.status(removed)).toBe("待移除 · 读取失败");
    expect(result.current.status(active)).toBe("配置读取失败");
  });

  it("取消最后一个分配仍显示待移除，正常侧栏不保留空目标", () => {
    const unassigned = { ...service, targets: [] };
    const { result, rerender } = renderHook(
      ({ changes, targets }) => useServiceLibrary([unassigned], targets, changes),
      { initialProps: { changes: [change], targets: [target] } },
    );
    expect(result.current.toolTargets).toEqual([]);
    expect(result.current.status(unassigned)).toBe("待移除");
    act(() => result.current.setFilter("pending"));
    expect(result.current.visible).toEqual([unassigned]);
    rerender({ changes: [], targets: [{ ...target, error: "无法读取文件" }] });
    expect(result.current.visible).toEqual([unassigned]);
    expect(result.current.status(unassigned)).toBe("配置读取失败");
  });

  it("区分新增、更新、移除、冲突、对齐与尚未分配，不由无关目标错误污染状态", () => {
    const unrelated = { ...target, id: "other", error: "读取失败" };
    const { result, rerender } = renderHook(
      ({ changes }) => useServiceLibrary([service], [target, unrelated], changes),
      { initialProps: { changes: [] as Change[] } },
    );
    expect(result.current.status(service)).toBe("配置已对齐");
    expect(result.current.status({ ...service, targets: [], bindings: {} })).toBe("尚未分配");
    rerender({ changes: [{ ...change, action: "add" }] });
    expect(result.current.status(service)).toBe("待新增");
    rerender({ changes: [{ ...change, action: "update" }] });
    expect(result.current.status(service)).toBe("待更新");
    rerender({ changes: [{ ...change, action: "add" }, { ...change, targetId: "other" }] });
    expect(result.current.status(service)).toBe("待新增 / 待移除");
    rerender({ changes: [{ ...change, conflict: true }] });
    expect(result.current.status(service)).toBe("配置冲突");
  });

  it("计划未完成时不能将缺少差异的已分配服务标记为对齐", () => {
    const { result } = renderHook(() => useServiceLibrary([service], [target], [], true));
    expect(result.current.status(service)).toBe("同步状态待确认");
    expect(result.current.status({ ...service, targets: [], bindings: {} })).toBe("尚未分配");
  });

  it("旧路径提醒不是读取失败，真实读取错误优先于路径提醒", () => {
    const warned = { ...target, warning: "旧配置路径需要确认" };
    const { result, rerender } = renderHook(
      ({ targets }) => useServiceLibrary([service], targets, []),
      { initialProps: { targets: [warned] } },
    );
    expect(result.current.status(service)).toBe("配置路径待确认");
    rerender({ targets: [{ ...warned, error: "无权读取文件" }] });
    expect(result.current.status(service)).toBe("配置读取失败");
    rerender({ targets: [{ ...warned, id: "other" }] });
    expect(result.current.status(service)).toBe("配置已对齐");
  });
});

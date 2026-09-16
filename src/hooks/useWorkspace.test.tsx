import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "./useWorkspace";
import type { Preview, ServiceInput, Snapshot } from "../types";

const request = vi.hoisted(() => vi.fn());
vi.mock("../api", () => ({ request }));

const snapshot: Snapshot = {
  workspace: { version: 1, revision: 1, onboardingComplete: true, services: [], targets: [], history: [] },
  adapters: [], targets: [], dataDir: "/fixture/data", isolated: true,
};
const preview: Preview = { id: "plan-1", changes: [], errors: [], fileCount: 0 };
const input: ServiceInput = {
  id: null, key: "git", name: "Git", description: "",
  config: { transport: "stdio", command: "python3", args: ["-m", "mcp_server_git"], cwd: "", env: {}, url: "", headers: {} },
};
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => { resolve = r; });
  return { promise, resolve };
}
function mount() {
  const onNotice = vi.fn();
  return { ...renderHook(() => useWorkspace({ includeDetails: false, onNotice })), onNotice };
}

beforeEach(() => {
  request.mockReset().mockImplementation(async (op: string) => {
    if (op === "snapshot") return snapshot;
    if (op === "preview") return preview;
    if (op === "saveService") return "saved-service";
    return null;
  });
});
afterEach(cleanup);

describe("工作区写入与刷新结果", () => {
  it.each(["snapshot", "preview"])("保存成功但 %s 失败仍返回 ID，只重试读取", async (failedOp) => {
    const { result, onNotice } = mount();
    await waitFor(() => expect(result.current.data).toBe(snapshot));
    const original = request.getMockImplementation()!;
    let fail = true;
    request.mockImplementation(async (op, ...args) => {
      if (op === failedOp && fail) throw new Error("read failed");
      return original(op, ...args);
    });
    let id: string | null = null;
    await act(async () => { id = await result.current.actions.saveService(input); });
    expect(id).toBe("saved-service");
    expect(onNotice).toHaveBeenCalledWith("服务已保存");
    expect(result.current.error).toBe("");
    expect(result.current.refreshWarning).toContain("无需重复操作");
    expect(result.current.busy).toBe(false);
    act(() => result.current.clearError());
    expect(result.current.refreshWarning).not.toBe("");
    await act(async () => {
      expect(await result.current.actions.saveService(input)).toBeNull();
      expect(await result.current.actions.apply("plan-1")).toBe(false);
    });
    expect(request.mock.calls.filter(([op]) => op === "saveService")).toHaveLength(1);
    expect(request.mock.calls.some(([op]) => op === "apply")).toBe(false);
    fail = false;
    await act(async () => { expect(await result.current.actions.reload()).toBe(true); });
    expect(result.current.refreshWarning).toBe("");
    expect(request.mock.calls.filter(([op]) => op === "saveService")).toHaveLength(1);
    await act(async () => { expect(await result.current.actions.assign("saved-service", "codex", true)).toBe(true); });
  });

  it("手动刷新失败后旧快照不能继续用于写入，重新读取可恢复", async () => {
    const { result } = mount();
    await waitFor(() => expect(result.current.data).toBe(snapshot));
    request.mockRejectedValueOnce(new Error("读取失败"));
    await act(async () => { expect(await result.current.actions.reload()).toBe(false); });
    expect(result.current.refreshWarning).toContain("上次状态");
    await act(async () => { expect(await result.current.actions.apply("plan-1")).toBe(false); });
    expect(request.mock.calls.some(([op]) => op === "apply")).toBe(false);
    await act(async () => { expect(await result.current.actions.reload()).toBe(true); });
    expect(result.current.refreshWarning).toBe("");
  });

  it("写入失败保留失败结果，不报告已提交或刷新失败", async () => {
    const { result, onNotice } = mount();
    await waitFor(() => expect(result.current.data).toBe(snapshot));
    request.mockRejectedValueOnce(new Error("保存被拒绝"));
    await act(async () => { expect(await result.current.actions.saveService(input)).toBeNull(); });
    expect(result.current.error).toContain("保存被拒绝");
    expect(result.current.refreshWarning).toBe("");
    expect(onNotice).not.toHaveBeenCalled();
    expect(result.current.busy).toBe(false);
  });

  it("写入及刷新进行中连续点击仅提交一次", async () => {
    const { result } = mount();
    await waitFor(() => expect(result.current.data).toBe(snapshot));
    const save = deferred<string>();
    request.mockImplementationOnce(() => save.promise);
    await act(async () => {
      const first = result.current.actions.saveService(input);
      expect(await result.current.actions.saveService(input)).toBeNull();
      save.resolve("saved-service");
      expect(await first).toBe("saved-service");
    });
    expect(request.mock.calls.filter(([op]) => op === "saveService")).toHaveLength(1);
  });

  it("导入已提交但刷新失败关闭导入流程，重复点击不会再次导入", async () => {
    const { result } = mount();
    await waitFor(() => expect(result.current.data).toBe(snapshot));
    request.mockResolvedValueOnce(2).mockRejectedValueOnce(new Error("refresh failed"));
    await act(async () => {
      expect(await result.current.actions.importText("cursor", "{}" )).toBe(true);
      expect(await result.current.actions.importText("cursor", "{}" )).toBe(false);
    });
    expect(request.mock.calls.filter(([op]) => op === "importText")).toHaveLength(1);
  });

  it("刷新失败后重新打开预览先读取完整快照，不沿用旧计划", async () => {
    const { result } = mount();
    await waitFor(() => expect(result.current.data).toBe(snapshot));
    request.mockResolvedValueOnce(null).mockRejectedValueOnce(new Error("refresh failed"));
    await act(async () => { expect(await result.current.actions.apply("plan-1")).toBe(true); });
    request.mockClear();
    await act(async () => { expect(await result.current.actions.loadPreview()).toBe(true); });
    expect(request.mock.calls).toEqual([["snapshot"], ["preview", { includeDetails: true }]]);
    expect(result.current.refreshWarning).toBe("");
  });
});

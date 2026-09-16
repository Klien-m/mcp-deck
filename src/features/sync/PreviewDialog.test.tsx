import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { request } from "../../api";
import type { Change, Preview } from "../../types";
import { PreviewDialog } from "./PreviewDialog";

vi.mock("../../api", () => ({ request: vi.fn() }));
const change: Change = {
  serviceId: "git", targetId: "codex", targetName: "Codex", key: "git", action: "add",
  before: null, after: { command: "python3", env: { TOKEN: "<已隐藏>" } }, conflict: false, message: "",
};
const preview: Preview = {
  id: "plan-1", changes: [change], errors: [], fileCount: 1,
  fullChanges: [{ ...change, after: { command: "python3", env: { TOKEN: "test-secret" } } }],
};
const callbacks = () => ({
  onApply: vi.fn().mockResolvedValue(true), onResolve: vi.fn().mockResolvedValue(true),
  onClose: vi.fn(), onApplied: vi.fn(),
});

afterEach(() => { cleanup(); vi.clearAllMocks(); });

describe("同步预览显示切换", () => {
  it("明文切换只展示同一计划，保持应用按钮节点和状态，不触发 IPC 或写操作", async () => {
    const handlers = callbacks();
    const view = render(<PreviewDialog {...handlers} preview={preview} error="" busy={false} />);
    const apply = screen.getByRole("button", { name: "应用 1 项变更" });
    const initialMarkup = apply.outerHTML;
    expect(screen.getByText("将向 Codex 新增 git")).toBeTruthy();
    expect(screen.queryByText(/test-secret/)).toBeNull();
    const toggle = screen.getByRole("checkbox", { name: "显示完整差异" });
    fireEvent.click(toggle);
    expect(screen.getByText(/test-secret/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "应用 1 项变更" })).toBe(apply);
    expect(apply.outerHTML).toBe(initialMarkup);
    fireEvent.click(toggle);
    expect(screen.queryByText(/test-secret/)).toBeNull();
    expect(request).not.toHaveBeenCalled();
    expect(handlers.onApply).not.toHaveBeenCalled();
    expect(handlers.onResolve).not.toHaveBeenCalled();
    fireEvent.click(toggle);
    view.rerender(<PreviewDialog {...handlers} preview={{ ...preview, id: "plan-2" }} error="" busy={false} />);
    expect(screen.queryByText(/test-secret/)).toBeNull();
    fireEvent.click(apply);
    await waitFor(() => expect(handlers.onApplied).toHaveBeenCalledTimes(1));
    expect(handlers.onApply).toHaveBeenCalledExactlyOnceWith("plan-2");
  });

  it("零变更和冲突计划在显示切换后仍不可应用，移除方向始终可见", () => {
    const handlers = callbacks();
    const view = render(<PreviewDialog {...handlers} preview={{ ...preview, changes: [], fullChanges: [], fileCount: 0 }} error="" busy={false} />);
    const apply = screen.getByRole("button", { name: "应用 0 项变更" });
    expect(apply.hasAttribute("disabled")).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: "显示完整差异" }));
    expect(screen.getByRole("button", { name: "应用 0 项变更" })).toBe(apply);
    expect(apply.hasAttribute("disabled")).toBe(true);
    const removal = { ...change, action: "remove" as const, after: null, conflict: true, message: "外部配置已修改" };
    view.rerender(<PreviewDialog {...handlers} preview={{ ...preview, id: "conflict", changes: [removal], fullChanges: [removal] }} error="" busy={false} />);
    expect(screen.getByText("将从 Codex 移除 git")).toBeTruthy();
    const blocked = screen.getByRole("button", { name: "应用 1 项变更" });
    expect(blocked.hasAttribute("disabled")).toBe(true);
    fireEvent.click(blocked);
    expect(handlers.onApply).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "采用磁盘版本" }));
    expect(handlers.onResolve).toHaveBeenCalledExactlyOnceWith("git", "codex", true);
  });

  it("刷新失败后的旧预览阻止应用与冲突选择，但允许查看明文和返回", () => {
    const handlers = callbacks();
    const conflicted = { ...change, conflict: true, message: "外部修改" };
    render(<PreviewDialog {...handlers} preview={{ ...preview, changes: [conflicted], fullChanges: [{ ...preview.fullChanges![0], conflict: true }] }} error="" busy={false} stale />);
    expect(screen.getByText("此预览已过期，请重新读取配置")).toBeTruthy();
    expect(screen.getByRole("button", { name: "应用 1 项变更" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "采用磁盘版本" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "保留服务库版本" }).hasAttribute("disabled")).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: "显示完整差异" }));
    expect(screen.getByText(/test-secret/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "返回" }));
    expect(handlers.onClose).toHaveBeenCalledTimes(1);
    expect(handlers.onApply).not.toHaveBeenCalled();
    expect(handlers.onResolve).not.toHaveBeenCalled();
  });

  it("零变更的旧预览不能宣称配置已对齐", () => {
    render(<PreviewDialog {...callbacks()} preview={{ ...preview, changes: [], fullChanges: [], fileCount: 0 }} error="" busy={false} stale />);
    expect(screen.getByText("此预览已过期，请重新读取配置")).toBeTruthy();
    expect(screen.queryByText("配置已对齐")).toBeNull();
    expect(screen.queryByText("没有需要写入的服务变更。")).toBeNull();
  });
});

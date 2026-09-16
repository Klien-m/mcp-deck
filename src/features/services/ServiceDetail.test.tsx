import { useState } from "react";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Adapter, Change, Checks, Service, TargetStatus } from "../../types";
import { request } from "../../api";
import { ServiceDetail } from "./ServiceDetail";
import { ServiceList } from "./ServiceList";
import { useServiceLibrary } from "./useServiceLibrary";

vi.mock("../../api", () => ({ request: vi.fn() }));
const target: TargetStatus = {
  id: "codex", adapterId: "codex", name: "Codex", path: "/home/test/.codex/config.toml",
  exists: true, count: 1, error: null,
};
const adapter: Adapter = {
  id: "codex", name: "Codex", rootKey: "mcp_servers", format: "toml",
  transports: ["stdio", "http"], supportsCwd: true, docs: "", note: "",
};
const service: Service = {
  id: "git", key: "git", name: "Git", description: "Git 工具",
  config: { transport: "stdio", command: "python3", args: [], cwd: "", env: {}, url: "", headers: {} },
  targets: ["codex"], bindings: { codex: { raw: { command: "python3" } } }, native: {}, deleted: false,
};
const change: Change = {
  serviceId: "git", targetId: "codex", targetName: "Codex", key: "git",
  action: "remove", before: { command: "python3" }, after: null, conflict: false, message: "",
};
const callbacks = () => ({
  onAssign: vi.fn().mockResolvedValue(true), onRemove: vi.fn().mockResolvedValue(true),
  onUndoRemove: vi.fn().mockResolvedValue(true), onEdit: vi.fn(), onExport: vi.fn(), onRemoved: vi.fn(),
});

afterEach(() => { cleanup(); vi.resetAllMocks(); });

describe("服务详情的待应用操作", () => {
  it("待移除服务可从待应用列表进入，并通过持久撤销入口恢复", async () => {
    const undo = vi.fn().mockResolvedValue(true);
    function Library() {
      const [removed, setRemoved] = useState(true);
      const services = [{ ...service, deleted: removed, targets: removed ? [] : ["codex"] }];
      const changes = removed ? [change] : [];
      const library = useServiceLibrary(services, [target], changes);
      return <>
        <button onClick={() => library.setFilter("pending")}>待应用筛选</button>
        <ServiceList visible={library.visible} selectedId={library.service?.id} targets={[target]}
          pendingIds={library.pendingIds} status={library.status} filter={library.filter}
          query={library.query} serviceCount={library.services.length} adapterCount={1}
          onSelect={library.select} onFilter={library.setFilter} onQuery={library.setQuery}
          onExport={() => {}} onAdd={() => {}} />
        {library.service && <ServiceDetail {...callbacks()} service={library.service} adapters={[adapter]}
          targets={[target]} changes={changes} busy={false} status={library.status(library.service)}
          onUndoRemove={async () => { await undo(); setRemoved(false); return true; }} />}
      </>;
    }
    render(<Library />);
    expect(screen.queryByRole("button", { name: "撤销移除" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "待应用筛选" }));
    expect(screen.getByText("确认同步后才会从目标配置文件移除此服务。应用前可以撤销。")).toBeTruthy();
    expect(screen.getByRole("switch").hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "编辑配置" }).hasAttribute("disabled")).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "撤销移除" }));
    await waitFor(() => expect(screen.queryByRole("button", { name: "撤销移除" })).toBeNull());
    expect(undo).toHaveBeenCalledTimes(1);
    expect(screen.getByText("当前没有待应用服务")).toBeTruthy();
  });

  it("目标读取失败显示路径和原因，不能显示配置已对齐或已连接", () => {
    render(<ServiceDetail {...callbacks()} service={service} adapters={[adapter]}
      targets={[{ ...target, error: "无法解析 TOML" }]} changes={[]} busy={false} status="配置读取失败" />);
    expect(screen.getByText("配置读取失败 · 无法确认同步状态")).toBeTruthy();
    expect(screen.getByText("无法解析 TOML")).toBeTruthy();
    expect(screen.getByText(target.path)).toBeTruthy();
    expect(screen.getByText("未检测 · 需在目标工具确认")).toBeTruthy();
    expect(screen.queryByText("配置已对齐")).toBeNull();
  });

  it("取消分配明确展示移除方向，只有用户切换才调用分配回调", () => {
    const handlers = callbacks();
    render(<ServiceDetail {...handlers} service={{ ...service, targets: [] }} adapters={[adapter]}
      targets={[target]} changes={[change]} busy={false} status="待移除" />);
    expect(screen.getByText("将从 Codex 移除 git · 待应用")).toBeTruthy();
    expect(handlers.onAssign).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("switch", { name: "Git 分配到 Codex" }));
    expect(handlers.onAssign).toHaveBeenCalledExactlyOnceWith("codex", true);
  });

  it("刷新失败后旧目标行不宣称对齐，编辑和分配均暂停", () => {
    render(<ServiceDetail {...callbacks()} service={service} adapters={[adapter]}
      targets={[target]} changes={[]} busy={false} stale status="状态待刷新" />);
    expect(screen.getByText("状态待刷新 · 请重新读取配置")).toBeTruthy();
    expect(screen.queryByText("配置已对齐")).toBeNull();
    expect(screen.getByRole("switch").hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("button", { name: "编辑配置" }).hasAttribute("disabled")).toBe(true);
  });

  it("同一服务配置被磁盘版本替换后清除已显示的检查，相同内容刷新则保留", async () => {
    vi.mocked(request).mockResolvedValueOnce({ issues: ["旧配置检查结果"], executable: null, note: "静态检查" });
    const handlers = callbacks();
    const detail = (item: Service) => <ServiceDetail {...handlers} service={item} adapters={[adapter]}
      targets={[target]} changes={[]} busy={false} status="配置已对齐" />;
    const view = render(detail(service));
    fireEvent.mouseDown(screen.getByRole("tab", { name: "配置检查" }), { button: 0, ctrlKey: false });
    await screen.findByText("旧配置检查结果", { exact: false });
    view.rerender(detail({ ...service, config: { ...service.config } }));
    expect(screen.getByText("旧配置检查结果", { exact: false })).toBeTruthy();
    view.rerender(detail({ ...service, config: { ...service.config, command: "node" } }));
    expect(screen.queryByText("旧配置检查结果", { exact: false })).toBeNull();
    expect(screen.getByRole("tab", { name: "概览" }).getAttribute("aria-selected")).toBe("true");
    expect(request).toHaveBeenCalledExactlyOnceWith("checks", { serviceId: service.id });
  });

  it("配置改变后旧检查请求晚到不能覆盖新配置的检查结果", async () => {
    let finishOld!: (result: Checks) => void;
    let finishNew!: (result: Checks) => void;
    vi.mocked(request)
      .mockImplementationOnce(() => new Promise<Checks>((resolve) => { finishOld = resolve; }))
      .mockImplementationOnce(() => new Promise<Checks>((resolve) => { finishNew = resolve; }));
    const handlers = callbacks();
    const detail = (item: Service) => <ServiceDetail {...handlers} service={item} adapters={[adapter]}
      targets={[target]} changes={[]} busy={false} status="配置已对齐" />;
    const view = render(detail(service));
    const check = () => fireEvent.mouseDown(screen.getByRole("tab", { name: "配置检查" }), { button: 0, ctrlKey: false });
    check();
    view.rerender(detail({ ...service, config: { ...service.config, args: ["new-server.js"] } }));
    check();
    await act(async () => finishNew({ issues: ["新配置检查结果"], executable: null, note: "静态检查" }));
    expect(screen.getByText("新配置检查结果", { exact: false })).toBeTruthy();
    await act(async () => finishOld({ issues: ["旧配置检查结果"], executable: null, note: "静态检查" }));
    expect(screen.queryByText("旧配置检查结果", { exact: false })).toBeNull();
    expect(screen.getByText("新配置检查结果", { exact: false })).toBeTruthy();
    expect(request).toHaveBeenCalledTimes(2);
  });
});

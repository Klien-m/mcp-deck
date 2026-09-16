import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import type { Snapshot } from "./types";

const request = vi.hoisted(() => vi.fn());
vi.mock("./api", () => ({ request, native: true }));
vi.mock("./hooks/useAppVersion", () => ({ useAppVersion: () => "0.4.2" }));
// 本测试保留真实 App/Hook/引导弹窗；仅替换与场景无关的设置页。
vi.mock("./features/settings/SettingsPage", () => ({ SettingsPage: () => null }));
const empty: Snapshot = {
  workspace: { version: 1, revision: 0, onboardingComplete: false, services: [], targets: [], history: [] },
  adapters: [], targets: [], dataDir: "/fixture/data", isolated: true,
};
let completed = false;
let failRefresh = false;
beforeEach(() => {
  completed = false;
  failRefresh = false;
  request.mockReset().mockImplementation(async (op: string) => {
    if (op === "snapshot") {
      if (failRefresh) throw new Error("暂时无法读取");
      return { ...empty, workspace: { ...empty.workspace, onboardingComplete: completed } };
    }
    if (op === "preview") return { id: "current-plan", changes: [], errors: [], fileCount: 0 };
    if (op === "discoverAll") return [];
    if (op === "completeOnboarding") { completed = true; failRefresh = true; return null; }
    throw new Error(`unexpected command ${op}`);
  });
});
afterEach(cleanup);

it("首次引导已完成但刷新失败时能退出引导并重新读取，不重复提交", async () => {
  render(<App />);
  await screen.findByRole("dialog", { name: "将已有 MCP 纳入管理" });
  fireEvent.click(await screen.findByRole("button", { name: "进入工作区" }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  expect(screen.getByRole("alert").textContent).toContain("无需重复操作");
  expect(screen.getByText("0.4.2 · 内部试用")).toBeTruthy();
  expect(request.mock.calls.filter(([op]) => op === "completeOnboarding")).toHaveLength(1);
  failRefresh = false;
  await act(async () => { fireEvent.click(screen.getByRole("button", { name: "重新读取配置" })); });
  await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(request.mock.calls.filter(([op]) => op === "completeOnboarding")).toHaveLength(1);
});

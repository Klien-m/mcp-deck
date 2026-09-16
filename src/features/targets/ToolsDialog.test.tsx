import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { Adapter, TargetStatus } from "../../types";
import { ToolsDialog } from "./ToolsDialog";

afterEach(cleanup);

it("路径提醒保留发现入口，真实读取错误才禁用发现", () => {
  const adapter: Adapter = {
    id: "codex", name: "Codex", rootKey: "mcp_servers", format: "toml",
    transports: ["stdio"], supportsCwd: true, docs: "", note: "配置说明",
  };
  const target: TargetStatus = {
    id: "legacy", adapterId: "codex", name: "旧路径目标", path: "/legacy/config.toml",
    exists: true, count: 1, error: null, warning: "此路径来自旧版默认位置，请确认",
  };
  const discover = vi.fn();
  render(<ToolsDialog adapters={[adapter]} targets={[target, { ...target, id: "broken", name: "损坏目标", error: "TOML 解析失败" }]}
    dataDir="/fixture/data" error="" onClose={() => {}} onEdit={() => {}} onDiscover={discover} />);
  const readable = within(screen.getByText("旧路径目标").closest(".tool-setting") as HTMLElement);
  expect(readable.getByText("配置路径待确认")).toBeTruthy();
  expect(readable.getByText(target.warning!)).toBeTruthy();
  const enabled = readable.getByRole("button", { name: "发现" });
  expect(enabled.hasAttribute("disabled")).toBe(false);
  fireEvent.click(enabled);
  expect(discover).toHaveBeenCalledExactlyOnceWith("legacy");
  const broken = within(screen.getByText("损坏目标").closest(".tool-setting") as HTMLElement);
  expect(broken.getByText("配置读取失败")).toBeTruthy();
  expect(broken.getByText("TOML 解析失败")).toBeTruthy();
  expect(broken.getByRole("button", { name: "发现" }).hasAttribute("disabled")).toBe(true);
});

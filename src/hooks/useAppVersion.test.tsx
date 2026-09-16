import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { useAppVersion } from "./useAppVersion";

const api = vi.hoisted(() => ({ getVersion: vi.fn(), isTauri: vi.fn() }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: api.getVersion }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: api.isTauri }));
beforeEach(() => {
  api.isTauri.mockReset().mockReturnValue(true);
  api.getVersion.mockReset().mockResolvedValue("0.4.2");
});
afterEach(cleanup);

it("使用安装包版本，而不是源码中固定的版本", async () => {
  const { result } = renderHook(useAppVersion);
  await waitFor(() => expect(result.current).toBe("0.4.2"));
  expect(api.getVersion).toHaveBeenCalledTimes(1);
});
it("版本读取失败显示未知，不误报旧版本", async () => {
  api.getVersion.mockRejectedValue(new Error("unavailable"));
  const { result } = renderHook(useAppVersion);
  await waitFor(() => expect(result.current).toBe("版本未知"));
});
it("浏览器预览不调用原生版本接口", () => {
  api.isTauri.mockReturnValue(false);
  const { result } = renderHook(useAppVersion);
  expect(result.current).toBe("开发预览");
  expect(api.getVersion).not.toHaveBeenCalled();
});

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, act } from "@testing-library/react";
import { ThemeProvider } from "./theme";
import { SettingsPage } from "./features/settings/SettingsPage";

const native = vi.hoisted(() => ({ setTheme: vi.fn(), setBackgroundColor: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => native }));

let media: MediaQueryList;
function systemDark(dark: boolean) {
  Object.defineProperty(media, "matches", { configurable: true, value: dark });
  media.dispatchEvent(new Event("change"));
}
function mount() {
  return render(<ThemeProvider><SettingsPage onBack={() => {}} /></ThemeProvider>);
}
const choose = (name: string) => fireEvent.click(screen.getByRole("radio", { name }));

beforeEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
  document.documentElement.className = "";
  delete document.documentElement.dataset.palette;
  document.documentElement.style.setProperty("--window-background", "#ffffff");
  media = Object.assign(new EventTarget(), { matches: false, media: "(prefers-color-scheme: dark)" }) as MediaQueryList;
  vi.stubGlobal("matchMedia", vi.fn(() => media));
  native.setTheme.mockReset().mockResolvedValue(undefined);
  native.setBackgroundColor.mockReset().mockResolvedValue(undefined);
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("工作区外观偏好", () => {
  it("读取已有深色 / 经典绿偏好并同步原生窗口", async () => {
    localStorage.setItem("mcp-deck.theme", "dark");
    localStorage.setItem("mcp-deck.palette", "green");
    mount();
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.dataset.palette).toBe("green");
    expect(screen.getByRole("radio", { name: "深色" }).getAttribute("aria-checked")).toBe("true");
    await waitFor(() => expect(native.setTheme).toHaveBeenCalledWith("dark"));
    await waitFor(() => expect(native.setBackgroundColor).toHaveBeenCalledWith("#ffffff"));
  });

  it("独立保存模式和配色，重新挂载仍保留选择", async () => {
    const view = mount();
    choose("浅色");
    choose("经典绿");
    choose("深色");
    expect(localStorage.getItem("mcp-deck.theme")).toBe("dark");
    expect(localStorage.getItem("mcp-deck.palette")).toBe("green");
    view.unmount();
    mount();
    expect(screen.getByRole("radio", { name: "深色" }).getAttribute("aria-checked")).toBe("true");
    expect(screen.getByRole("radio", { name: "经典绿" }).getAttribute("aria-checked")).toBe("true");
    await waitFor(() => expect(native.setTheme).toHaveBeenLastCalledWith("dark"));
  });

  it("无效偏好回退到系统模式，只有系统模式跟随媒体查询", async () => {
    localStorage.setItem("mcp-deck.theme", "invalid");
    localStorage.setItem("mcp-deck.palette", "invalid");
    mount();
    expect(screen.getByRole("radio", { name: "跟随系统" }).getAttribute("aria-checked")).toBe("true");
    expect(document.documentElement.dataset.palette).toBe("graphite");
    act(() => systemDark(true));
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    choose("浅色");
    act(() => { systemDark(false); systemDark(true); });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    await waitFor(() => expect(native.setTheme).toHaveBeenLastCalledWith("light"));
  });

  it("从显式深色切回系统时清除 macOS 原生主题覆盖", async () => {
    native.setTheme.mockImplementation(async (theme: string | null) => {
      systemDark(theme === "dark");
    });
    localStorage.setItem("mcp-deck.theme", "dark");
    mount();
    await waitFor(() => expect(document.documentElement.classList.contains("dark")).toBe(true));
    choose("跟随系统");
    await waitFor(() => expect(native.setTheme).toHaveBeenLastCalledWith(null));
    await waitFor(() => expect(document.documentElement.classList.contains("dark")).toBe(false));
  });

  it("存储不可写时仍可在当前会话切换", async () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("unavailable"); });
    mount();
    choose("深色");
    choose("经典绿");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.dataset.palette).toBe("green");
    await waitFor(() => expect(native.setTheme).toHaveBeenLastCalledWith("dark"));
  });

  it("原生同步失败保留网页主题并显示可恢复提示", async () => {
    native.setTheme.mockRejectedValue(new Error("denied"));
    mount();
    choose("深色");
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("标题栏外观未能更新"));
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    native.setTheme.mockResolvedValue(undefined);
    choose("浅色");
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
  });
});

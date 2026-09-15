import { createContext, useContext, useEffect, useLayoutEffect, useState, type ReactNode } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type Theme = "light" | "dark" | "system";
export type Palette = "graphite" | "green";
const storageKey = "mcp-deck.theme";
const paletteKey = "mcp-deck.palette";
const systemQuery = "(prefers-color-scheme: dark)";
const isTheme = (value: unknown): value is Theme =>
  value === "light" || value === "dark" || value === "system";

function readTheme(): Theme {
  try {
    const value = localStorage.getItem(storageKey);
    return isTheme(value) ? value : "system";
  } catch {
    return "system";
  }
}

function readPalette(): Palette {
  try {
    return localStorage.getItem(paletteKey) === "green" ? "green" : "graphite";
  } catch {
    return "graphite";
  }
}

const ThemeContext = createContext<{
  theme: Theme;
  resolvedTheme: "light" | "dark";
  setTheme: (theme: Theme) => void;
  palette: Palette;
  setPalette: (palette: Palette) => void;
  appearanceError: string;
} | null>(null);

/** 外观偏好与业务配置独立；系统变化和其他窗口的偏好更新均实时生效。 */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, updateTheme] = useState<Theme>(readTheme);
  const [palette, updatePalette] = useState<Palette>(readPalette);
  const [appearanceError, setAppearanceError] = useState("");
  const [systemDark, setSystemDark] = useState(() => window.matchMedia(systemQuery).matches);
  const resolvedTheme = theme === "system" ? (systemDark ? "dark" : "light") : theme;

  useLayoutEffect(() => {
    document.documentElement.classList.toggle("dark", resolvedTheme === "dark");
    document.documentElement.style.colorScheme = resolvedTheme;
    document.documentElement.dataset.palette = palette;
  }, [resolvedTheme, palette]);

  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    const nativeWindow = getCurrentWindow();
    // 系统模式必须清除原生外观覆盖，避免 WebView 的媒体查询跟随应用覆盖值。
    const background = getComputedStyle(document.documentElement).getPropertyValue("--window-background").trim();
    void nativeWindow.setTheme(theme === "system" ? null : theme)
      .then(() => { if (active) return nativeWindow.setBackgroundColor(background); })
      .then(() => { if (active) setAppearanceError(""); })
      .catch(() => { if (active) setAppearanceError("标题栏外观未能更新，请重新启动应用后重试。"); });
    return () => { active = false; };
  }, [theme, resolvedTheme, palette]);

  useEffect(() => {
    const media = window.matchMedia(systemQuery);
    const onChange = () => setSystemDark(media.matches);
    const onStorage = (event: StorageEvent) => {
      if (event.key === storageKey || event.key === null)
        updateTheme(isTheme(event.newValue) ? event.newValue : "system");
      if (event.key === paletteKey || event.key === null)
        updatePalette(event.newValue === "green" ? "green" : "graphite");
    };
    onChange();
    media.addEventListener("change", onChange);
    window.addEventListener("storage", onStorage);
    return () => {
      media.removeEventListener("change", onChange);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  function setTheme(next: Theme) {
    updateTheme(next);
    try {
      localStorage.setItem(storageKey, next);
    } catch {
      // 存储不可用时仍允许本次会话切换主题。
    }
  }

  function setPalette(next: Palette) {
    updatePalette(next);
    try {
      localStorage.setItem(paletteKey, next);
    } catch {
      // 与主题模式一致，存储不可用时保留本次会话的选择。
    }
  }

  return <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme, palette, setPalette, appearanceError }}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error("useTheme 必须在 ThemeProvider 内使用");
  return context;
}

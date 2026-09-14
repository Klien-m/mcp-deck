import { invoke, isTauri } from "@tauri-apps/api/core";
export const native = isTauri();
export async function request<T>(
  op: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (!native)
    throw new Error(
      "请在 MCP Deck 桌面应用内使用真实配置管理。浏览器仅用于开发预览。",
    );
  return invoke<T>("dispatch", { request: { op, ...args } });
}

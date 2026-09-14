import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  Checks,
  Discovery,
  Preview,
  ServiceInput,
  Snapshot,
  Target,
} from "./types";

export interface CommandMap {
  snapshot: { args: Record<string, never>; result: Snapshot };
  saveService: { args: { input: ServiceInput }; result: string };
  assign: {
    args: { serviceId: string; targetId: string; enabled: boolean };
    result: null;
  };
  remove: { args: { serviceId: string; undo: boolean }; result: null };
  discover: { args: { targetId: string }; result: Discovery[] };
  adopt: { args: { targetId: string; keys: string[] }; result: null };
  importText: { args: { adapterId: string; text: string }; result: number };
  saveTarget: { args: { target: Target }; result: null };
  preview: {
    args: { reveal?: boolean; includeDetails?: boolean };
    result: Preview;
  };
  apply: { args: { id: string }; result: null };
  resolve: {
    args: { serviceId: string; targetId: string; useDisk: boolean };
    result: null;
  };
  rollback: { args: { id: string }; result: null };
  keepRecovery: { args: { id: string }; result: null };
  export: {
    args: { adapterId: string; serviceIds: string[]; includeSecrets: boolean };
    result: string;
  };
  saveExport: {
    args: {
      adapterId: string;
      serviceIds: string[];
      includeSecrets: boolean;
      path: string;
    };
    result: null;
  };
  checks: { args: { serviceId: string }; result: Checks };
}

export type Command = keyof CommandMap;
export type CommandArgs<K extends Command> = CommandMap[K]["args"];
export type CommandResult<K extends Command> = CommandMap[K]["result"];
export const native = isTauri();
export function request<K extends "snapshot" | "preview">(
  op: K,
  args?: CommandArgs<K>,
): Promise<CommandResult<K>>;
export function request<K extends Command>(
  op: K,
  args: CommandArgs<K>,
): Promise<CommandResult<K>>;
export async function request(
  op: Command,
  args: CommandArgs<Command> = {},
): Promise<CommandResult<Command>> {
  if (!native)
    throw new Error(
      "请在 MCP Deck 桌面应用内使用真实配置管理。浏览器仅用于开发预览。",
    );
  return invoke("dispatch", { request: { ...args, op } });
}

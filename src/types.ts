export type Transport = "stdio" | "http" | "sse";
export interface Config {
  transport: Transport;
  command: string;
  args: string[];
  cwd: string;
  env: Record<string, string>;
  url: string;
  headers: Record<string, string>;
}
export interface Service {
  id: string;
  key: string;
  name: string;
  description: string;
  config: Config;
  targets: string[];
  bindings: Record<string, { raw: unknown }>;
  native: Record<string, unknown>;
  deleted: boolean;
}
export interface Adapter {
  id: string;
  name: string;
  rootKey: string;
  format: string;
  transports: Transport[];
  supportsCwd: boolean;
  docs: string;
  note: string;
}
export interface Target {
  id: string;
  adapterId: string;
  name: string;
  path: string;
}
export interface TargetStatus extends Target {
  exists: boolean;
  count: number;
  error: string | null;
}
export interface History {
  id: string;
  at: number;
  status: string;
  summary: string;
  paths: string[];
  count: number;
}
export interface Snapshot {
  workspace: {
    version: number;
    revision: number;
    services: Service[];
    targets: Target[];
    history: History[];
  };
  adapters: Adapter[];
  targets: TargetStatus[];
  dataDir: string;
  isolated: boolean;
}
export interface Change {
  serviceId: string;
  targetId: string;
  targetName: string;
  key: string;
  action: string;
  before: unknown;
  after: unknown;
  conflict: boolean;
  message: string;
}
export interface Preview {
  id: string;
  changes: Change[];
  fullChanges?: Change[];
  errors: string[];
  fileCount: number;
}
export interface Discovery {
  key: string;
  config: Config | null;
  preview: unknown;
  error: string | null;
  managed: boolean;
}
export interface Checks {
  issues: string[];
  executable: string | null;
  note: string;
}

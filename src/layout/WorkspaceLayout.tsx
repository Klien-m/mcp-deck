import { Button } from "@/components/ui/button";
import type { ReactNode } from "react";
import { RefreshCw } from "lucide-react";

/** 固定外壳：顶部操作栏、三栏内容、底部状态及覆盖层；业务内容由插槽传入。 */
export function WorkspaceLayout({
  page,
  version,
  refreshWarning,
  children,
  overlays,
  isolated,
  busy,
  pendingCount,
  issueCount,
  revision,
  onRefresh,
  onPreview,
}: {
  page: "workspace" | "settings";
  version: string;
  refreshWarning: string;
  children: ReactNode;
  overlays: ReactNode;
  isolated: boolean;
  busy: boolean;
  pendingCount: number;
  issueCount: number;
  revision: number;
  onRefresh: () => void;
  onPreview: () => void;
}) {
  return (
    <div className="app-shell">
      <header className="topbar">
        <span className="topbar-title">
          MCP Deck <span className="version-chip">{version} · 内部试用</span>
        </span>
        <div className="toolbar">
          <span className="muted">
            {isolated ? "隔离测试工作区" : "本机工作区"}
          </span>
          <Button variant="ghost" size="icon-sm"
            className="icon-button"
            title="刷新磁盘状态"
            aria-label="刷新磁盘状态"
            disabled={busy}
            onClick={() => onRefresh()}
          >
            <RefreshCw size={16} className={busy ? "spin" : ""} />
          </Button>
          <Button variant="default" className="primary" disabled={busy} onClick={onPreview}>
            <RefreshCw size={14} />
            同步预览 <span className="count-badge">{pendingCount}</span>
          </Button>
        </div>
      </header>
      {refreshWarning && (
        <div className="refresh-warning" role="alert">
          <span>{refreshWarning}</span>
          <Button variant="outline" size="sm" disabled={busy} onClick={onRefresh}>
            重新读取配置
          </Button>
        </div>
      )}
      <main className={`columns ${page === "settings" ? "settings-layout" : ""}`}>{children}</main>
      <footer className="statusbar">
        <span>
          <span className="dot" />
          {isolated ? "隔离测试目录" : "本地工作区"} · 配置变更需手动应用
        </span>
        <span>
          {refreshWarning
            ? "界面状态待刷新"
            : issueCount
            ? `${issueCount} 个配置问题`
            : busy
              ? "正在处理…"
              : `已保存 · 修订 ${revision}`}
        </span>
      </footer>
      {overlays}
    </div>
  );
}

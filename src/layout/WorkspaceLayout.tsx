import type { ReactNode } from "react";
import { RefreshCw } from "lucide-react";

export function WorkspaceLayout({
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
          MCP Deck <span className="version-chip">0.1 · 内部试用</span>
        </span>
        <div className="toolbar">
          <span className="muted">
            {isolated ? "隔离测试工作区" : "本机工作区"}
          </span>
          <button
            className="icon-button"
            title="刷新磁盘状态"
            aria-label="刷新磁盘状态"
            disabled={busy}
            onClick={() => onRefresh()}
          >
            <RefreshCw size={16} className={busy ? "spin" : ""} />
          </button>
          <button className="primary" disabled={busy} onClick={onPreview}>
            <RefreshCw size={14} />
            同步预览 <span className="count-badge">{pendingCount}</span>
          </button>
        </div>
      </header>
      <main className="columns">{children}</main>
      <footer className="statusbar">
        <span>
          <span className="dot" />
          {isolated ? "隔离测试目录" : "本地工作区"} · 配置变更需手动应用
        </span>
        <span>
          {issueCount
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

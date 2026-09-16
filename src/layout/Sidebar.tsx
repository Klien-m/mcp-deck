import { Button } from "@/components/ui/button";
import {
  Download,
  FolderSearch,
  Grid2X2,
  History as HistoryIcon,
  RefreshCw,
  Settings2,
  Palette,
  SlidersHorizontal,
} from "lucide-react";
import { ToolIcon } from "../components";
import type { TargetStatus } from "../types";

/** 展示工作区筛选、已分配目标与功能入口；计数由服务库派生，组件不持有业务状态。 */
export function Sidebar({
  page,
  onSettings,
  filter,
  onFilter,
  serviceCount,
  pendingCount,
  toolTargets,
  detected,
  onDiscover,
  onImport,
  onHistory,
  onTools,
}: {
  page: "workspace" | "settings";
  onSettings: () => void;
  filter: string;
  onFilter: (filter: string) => void;
  serviceCount: number;
  pendingCount: number;
  toolTargets: (TargetStatus & { serviceCount: number })[];
  detected: number;
  onDiscover: () => void;
  onImport: () => void;
  onHistory: () => void;
  onTools: () => void;
}) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <span className="brand-mark">
          <img src="/app-icon.png" alt="" />
        </span>
        MCP Deck
      </div>
      <div className="nav-label">工作空间</div>
      <Button variant="ghost"
        className={`nav ${page === "workspace" && filter === "all" ? "active" : ""}`}
        onClick={() => onFilter("all")}
      >
        <Grid2X2 size={17} />
        <span>全部服务</span>
        <small>{serviceCount}</small>
      </Button>
      <Button variant="ghost"
        className={`nav ${page === "workspace" && filter === "pending" ? "active" : ""}`}
        onClick={() => onFilter("pending")}
      >
        <RefreshCw size={17} />
        <span>待应用</span>
        <small>{pendingCount} 个服务</small>
      </Button>
      <div className="nav-label target-label">
        <span>目标工具</span>
        <Button variant="ghost" size="icon-sm"
          className="icon-button"
          aria-label="管理目标工具"
          onClick={() => onTools()}
        >
          <SlidersHorizontal size={14} />
        </Button>
      </div>
      <div className="tool-nav">
        {toolTargets.map((t) => (
          <Button variant="ghost"
            key={t.id}
            className={`nav ${page === "workspace" && filter === t.id ? "active" : ""}`}
            onClick={() => onFilter(t.id)}
            title={`${t.name} · ${t.error ? "配置读取失败" : t.warning ? "配置路径待确认" : t.exists ? "已发现配置" : "未发现配置"}`}
          >
            <ToolIcon id={t.adapterId} />
            <span>{t.name}</span>
            <small className={t.error || t.warning ? "warning" : ""}>
              {t.error ? "!" : t.warning ? "待确认" : t.serviceCount}
            </small>
          </Button>
        ))}
      </div>
      <div className="sidebar-bottom">
        <Button variant="ghost" className="nav" onClick={() => onDiscover()}>
          <FolderSearch size={17} />
          <span>发现本机配置</span>
        </Button>
        <Button variant="ghost"
          className="nav"
          onClick={() => {
            onImport();
          }}
        >
          <Download size={17} />
          <span>导入配置</span>
        </Button>
        <Button variant="ghost" className="nav" onClick={() => onHistory()}>
          <HistoryIcon size={17} />
          <span>同步记录</span>
        </Button>
        <Button variant="ghost" className="nav" onClick={() => onTools()}>
          <Settings2 size={17} />
          <span>工具与路径</span>
        </Button>
        <Button variant="ghost" className={`nav ${page === "settings" ? "active" : ""}`} onClick={onSettings}>
          <Palette size={17} />
          <span>偏好设置</span>
        </Button>
        <div className="workspace-note">
          <span className="dot" />
          {detected} 个目标已发现配置
        </div>
      </div>
    </aside>
  );
}

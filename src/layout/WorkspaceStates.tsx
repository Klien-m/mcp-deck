import { Button } from "@/components/ui/button";
import {
  ArrowLeft,
  ArrowRight,
  FolderSearch,
  Layers,
  Plus,
} from "lucide-react";
import { native } from "../api";
import { ToolIcon } from "../components";
import type { Adapter } from "../types";

/** 首次快照未就绪时展示加载或错误；浏览器环境明确提示需要桌面宿主。 */
export function Startup({
  fatal,
  onRetry,
}: {
  fatal: string;
  onRetry: () => void;
}) {
  return (
    <main className="startup">
      <div className="brand-mark">
        <img src="/app-icon.png" alt="" />
      </div>
      <h1>MCP Deck</h1>
      <p>{fatal || (native ? "正在读取本地工作区…" : "请启动桌面应用")}</p>
      {fatal && (
        <>
          <p className="muted">
            {native
              ? "工作区加载失败时，原配置不会被覆盖。"
              : "网页没有本机配置权限。开发时运行 npm run tauri dev。"}
          </p>
          <Button variant="outline"
            onClick={() => {
              onRetry();
            }}
          >
            重新加载
          </Button>
        </>
      )}
    </main>
  );
}

/** 当前筛选无可见服务时的入口引导；不据此推断整个工作区一定为空。 */
export function EmptyWorkspace({
  adapters,
  filter,
  onDiscover,
  onAdd,
  onShowAll,
}: {
  adapters: Adapter[];
  filter: string;
  onDiscover: () => void;
  onAdd: () => void;
  onShowAll: () => void;
}) {
  return (
    <div className="welcome">
      <div className="welcome-icon">
        <Layers size={35} />
      </div>
      <span className="eyebrow">一个服务库，连接你的工具</span>
      <h2>让 MCP 配置井然有序。</h2>
      <p>
        发现已有配置，统一维护，按需分配。
        <br />
        每一次修改，都可以先预览再应用。
      </p>
      <div className="welcome-actions">
        <Button variant="default" className="primary" onClick={() => onDiscover()}>
          <FolderSearch size={17} />
          发现本机配置
          <ArrowRight size={16} />
        </Button>
        <Button variant="outline" onClick={() => onAdd()}>
          <Plus size={17} />
          添加服务
        </Button>
      </div>
      <div className="welcome-tools">
        {adapters.map((a) => (
          <ToolIcon key={a.id} id={a.id} />
        ))}
      </div>
      <span className="muted">
        已内置 {adapters.length} 种适配器 · 本地配置管理
      </span>
      {filter !== "all" && (
        <Button variant="ghost" className="text-button" onClick={() => onShowAll()}>
          <ArrowLeft size={14} />
          返回全部服务
        </Button>
      )}
    </div>
  );
}

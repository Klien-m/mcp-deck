import { useEffect, useRef } from "react";
import { Plus, Plug, Search, Upload, X } from "lucide-react";
import { ToolIcon } from "../../components";
import type { Service, TargetStatus } from "../../types";

/** 受控服务列表；筛选、选择和导出范围由上层决定，本组件只维护搜索框引用。 */
export function ServiceList({
  visible,
  selectedId,
  targets,
  pendingIds,
  status,
  filter,
  query,
  serviceCount,
  adapterCount,
  onSelect,
  onFilter,
  onQuery,
  onExport,
  onAdd,
}: {
  visible: Service[];
  selectedId?: string;
  targets: Pick<TargetStatus, "id" | "adapterId">[];
  pendingIds: Set<string>;
  status: (service: Service) => string;
  filter: string;
  query: string;
  serviceCount: number;
  adapterCount: number;
  onSelect: (id: string) => void;
  onFilter: (filter: string) => void;
  onQuery: (query: string) => void;
  onExport: () => void;
  onAdd: () => void;
}) {
  const search = useRef<HTMLInputElement>(null);
  // 仅主界面响应 Cmd/Ctrl+K，避免弹窗输入期间抢走焦点。
  useEffect(() => {
    const listener = (e: KeyboardEvent) => {
      if (
        (e.metaKey || e.ctrlKey) &&
        e.key.toLowerCase() === "k" &&
        !document.querySelector("dialog[open]")
      ) {
        e.preventDefault();
        search.current?.focus();
      }
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

  return (
    <section className="list-panel" aria-label="服务库">
      <div className="list-header">
        <div className="list-heading">
          <h1>
            {filter === "pending" ? "待应用" : "服务库"}{" "}
            <small>{visible.length} 个</small>
          </h1>
          <div className="inline">
            <button
              className="icon-button"
              aria-label="导出服务库"
              onClick={() => onExport()}
              disabled={!serviceCount}
            >
              <Upload size={17} />
            </button>
            <button
              className="icon-button"
              aria-label="添加服务"
              onClick={() => onAdd()}
            >
              <Plus size={19} />
            </button>
          </div>
        </div>
        <div className="search">
          <Search size={16} />
          <input
            ref={search}
            value={query}
            onChange={(e) => onQuery(e.target.value)}
            placeholder="搜索 MCP 服务…"
            aria-label="搜索 MCP 服务"
          />
          {query ? (
            <button
              className="icon-button"
              onClick={() => onQuery("")}
              aria-label="清除搜索"
            >
              <X size={14} />
            </button>
          ) : (
            <kbd>⌘ K</kbd>
          )}
        </div>
      </div>
      <div className="list-caption">
        <span>名称 / 分配情况</span>
        <span>传输方式</span>
      </div>
      <div className="service-list">
        {visible.map((s) => (
          <button
            className={`service ${s.id === selectedId ? "selected" : ""}`}
            key={s.id}
            aria-pressed={s.id === selectedId}
            onClick={() => {
              onSelect(s.id);
            }}
          >
            <span className="service-icon">
              <Plug size={20} />
            </span>
            <div className="service-content">
              <div className="service-name">
                {s.name}
                <span>{s.config.transport}</span>
              </div>
              <p>{s.description || s.key}</p>
              <div className="service-meta">
                {s.targets.slice(0, 5).map((t) => {
                  const target = targets.find((x) => x.id === t);
                  return target ? (
                    <ToolIcon key={t} id={target.adapterId} small />
                  ) : null;
                })}
                {s.targets.length > 5 && <small>+{s.targets.length - 5}</small>}
                <span className={pendingIds.has(s.id) ? "warning" : ""}>
                  · {status(s)}
                </span>
              </div>
            </div>
          </button>
        ))}
        {!visible.length && (
          <div className="list-empty">
            <Plug size={28} />
            <p>
              {query
                ? "没有匹配的服务"
                : filter === "all"
                  ? "服务库还是空的"
                  : filter === "pending"
                    ? "当前没有待应用服务"
                    : "还没有分配服务"}
            </p>
            {filter !== "all" && (
              <button className="text-button" onClick={() => onFilter("all")}>
                查看全部服务
              </button>
            )}
          </div>
        )}
      </div>
      <div className="list-bottom">
        <span>{adapterCount} 种适配器</span>
        <span>仅在本机保存</span>
      </div>
    </section>
  );
}

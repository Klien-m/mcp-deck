import { useCallback, useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import {
  Search,
  Plus,
  Download,
  Upload,
  RefreshCw,
  History as HistoryIcon,
  Settings2,
  SlidersHorizontal,
  Grid2X2,
  Plug,
  ArrowRight,
  Trash2,
  Check,
  AlertTriangle,
  FolderSearch,
  Layers,
  X,
  ArrowLeft,
  ShieldCheck,
} from "lucide-react";
import { request, native } from "./api";
import { Code, ErrorBox, Modal, ToolIcon } from "./components";
import { Editor, TargetEditor } from "./Editor";
import { Select } from "./Select";
import type {
  Snapshot,
  Service,
  Target,
  Preview,
  Discovery,
  Checks,
} from "./types";

type View =
  "discover" | "import" | "export" | "preview" | "history" | "tools" | null;
const actionNames: Record<string, string> = {
  add: "新增配置",
  update: "更新配置",
  remove: "移除配置",
};
const historyNames: Record<string, string> = {
  applied: "已写入",
  recovered: "已恢复",
  "recovery-needed": "需要处理",
  "rolled-back": "已撤回",
  "recovery-kept": "已保留磁盘版本",
};
export default function App() {
  const [data, setData] = useState<Snapshot | null>(null);
  const [fatal, setFatal] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [selected, setSelected] = useState("");
  const [filter, setFilter] = useState("all");
  const [query, setQuery] = useState("");
  const [tab, setTab] = useState("overview");
  const [view, setView] = useState<View>(null);
  const [editing, setEditing] = useState<Service | null | undefined>();
  const [targetEditing, setTargetEditing] = useState<
    Target | null | undefined
  >();
  const [revealPreview, setRevealPreview] = useState(false);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [checks, setChecks] = useState<Checks | null>(null);
  const [toast, setToast] = useState("");
  const [undo, setUndo] = useState("");
  const [discoveryTarget, setDiscoveryTarget] = useState("codex");
  const [discovered, setDiscovered] = useState<Discovery[]>([]);
  const [chosen, setChosen] = useState<string[]>([]);
  const [adapterId, setAdapterId] = useState("claude");
  const [transfer, setTransfer] = useState("");
  const [exportIds, setExportIds] = useState<string[]>([]);
  const [secrets, setSecrets] = useState(false);
  const search = useRef<HTMLInputElement>(null);
  const refresh = useCallback(async () => {
    const s = await request<Snapshot>("snapshot");
    const p = await request<Preview>("preview");
    setData(s);
    setFilter((current) =>
      current === "all" ||
      current === "pending" ||
      (s.targets.some((t) => t.id === current) &&
        s.workspace.services.some(
          (service) => !service.deleted && service.targets.includes(current),
        ))
        ? current
        : "all",
    );
    setPreview(p);
    setRevealPreview(false);
    return s;
  }, []);
  useEffect(() => {
    refresh().catch((e) => setFatal(String(e)));
  }, [refresh]);
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
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(""), 7000);
    return () => clearTimeout(t);
  }, [toast]);
  const notice = (text: string) => {
    setToast(text);
    setError("");
    setUndo("");
  };
  async function perform(
    op: string,
    args: Record<string, unknown> = {},
    message = "已保存",
  ) {
    setBusy(true);
    setError("");
    try {
      await request(op, args);
      await refresh();
      notice(message);
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function discover(targetId: string) {
    setDiscoveryTarget(targetId);
    setBusy(true);
    setError("");
    setView("discover");
    setChosen([]);
    setDiscovered([]);
    try {
      setDiscovered(await request<Discovery[]>("discover", { targetId }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function showPreview() {
    setBusy(true);
    setError("");
    setRevealPreview(false);
    try {
      setPreview(await request<Preview>("preview"));
      setView("preview");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function generateExport(
    nextAdapter: string,
    ids: string[],
    include: boolean,
  ) {
    setBusy(true);
    setError("");
    setTransfer("");
    try {
      setTransfer(
        await request<string>("export", {
          adapterId: nextAdapter,
          serviceIds: ids,
          includeSecrets: include,
        }),
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function showExport(ids: string[]) {
    setExportIds(ids);
    setAdapterId("claude");
    setSecrets(false);
    setView("export");
    await generateExport("claude", ids, false);
  }
  async function download() {
    try {
      const a = data?.adapters.find((a) => a.id === adapterId);
      const path = await save({
        defaultPath:
          adapterId === "codex"
            ? "mcp-deck-export.toml"
            : "mcp-deck-export.json",
        filters: [
          {
            name: a?.format || "配置",
            extensions: [adapterId === "codex" ? "toml" : "json"],
          },
        ],
      });
      if (path) {
        await request("saveExport", {
          adapterId,
          serviceIds: exportIds,
          includeSecrets: secrets,
          path,
        });
        notice("配置已导出");
        setView(null);
      }
    } catch (e) {
      setError(String(e));
    }
  }
  async function checkService(service: Service) {
    setTab("checks");
    setChecks(null);
    try {
      setChecks(await request<Checks>("checks", { serviceId: service.id }));
    } catch (e) {
      setError(String(e));
    }
  }
  const close = () => {
    if (!busy) {
      setView(null);
      setError("");
    }
  };

  if (!data)
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
            <button
              onClick={() => {
                setFatal("");
                refresh().catch((e) => setFatal(String(e)));
              }}
            >
              重新加载
            </button>
          </>
        )}
      </main>
    );
  const services = data.workspace.services.filter((s) => !s.deleted);
  const toolTargets = data.targets
    .map((t) => ({
      ...t,
      serviceCount: services.filter((s) => s.targets.includes(t.id)).length,
    }))
    .filter((t) => t.serviceCount > 0);
  const pendingIds = new Set(preview?.changes.map((c) => c.serviceId));
  const visible = services.filter(
    (s) =>
      (filter === "all" ||
        (filter === "pending" && pendingIds.has(s.id)) ||
        s.targets.includes(filter)) &&
      `${s.name} ${s.key} ${s.description}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  const service = visible.find((s) => s.id === selected) || visible[0];
  const detected = data.targets.filter((t) => t.exists).length;
  const firstFound = data.targets.find((t) => t.exists)?.id || "codex";
  const importableKeys = discovered
    .filter((d) => d.config && !d.error && !d.managed)
    .map((d) => d.key);
  const chosenKeys = importableKeys.filter((key) => chosen.includes(key));
  const allChosen = importableKeys.length > 0 && chosenKeys.length === importableKeys.length;
  const adapter = (id: string) => data.adapters.find((a) => a.id === id)!;
  const serviceChanges =
    preview?.changes.filter((c) => c.serviceId === service?.id) || [];
  const status = (s: Service) =>
    preview?.changes.some((c) => c.serviceId === s.id && c.conflict)
      ? "配置冲突"
      : pendingIds.has(s.id)
        ? "待应用"
        : s.targets.length
          ? "配置已对齐"
          : "尚未分配";

  return (
    <div className="app-shell">
      <header className="topbar">
        <span className="topbar-title">
          MCP Deck <span className="version-chip">0.1 · 内部试用</span>
        </span>
        <div className="toolbar">
          <span className="muted">
            {data.isolated ? "隔离测试工作区" : "本机工作区"}
          </span>
          <button
            className="icon-button"
            title="刷新磁盘状态"
            aria-label="刷新磁盘状态"
            disabled={busy}
            onClick={() => perform("snapshot", {}, "已重新读取配置")}
          >
            <RefreshCw size={16} className={busy ? "spin" : ""} />
          </button>
          <button className="primary" disabled={busy} onClick={showPreview}>
            <RefreshCw size={14} />
            同步预览{" "}
            <span className="count-badge">{preview?.changes.length || 0}</span>
          </button>
        </div>
      </header>
      <main className="columns">
        <aside className="sidebar">
          <div className="brand">
            <span className="brand-mark">
              <img src="/app-icon.png" alt="" />
            </span>
            MCP Deck
          </div>
          <div className="nav-label">工作空间</div>
          <button
            className={`nav ${filter === "all" ? "active" : ""}`}
            onClick={() => setFilter("all")}
          >
            <Grid2X2 size={17} />
            <span>全部服务</span>
            <small>{services.length}</small>
          </button>
          <button
            className={`nav ${filter === "pending" ? "active" : ""}`}
            onClick={() => setFilter("pending")}
          >
            <RefreshCw size={17} />
            <span>待应用</span>
            <small>{preview?.changes.length || 0} 项</small>
          </button>
          <div className="nav-label target-label">
            <span>目标工具</span>
            <button
              className="icon-button"
              aria-label="管理目标工具"
              onClick={() => setView("tools")}
            >
              <SlidersHorizontal size={14} />
            </button>
          </div>
          <div className="tool-nav">
            {toolTargets.map((t) => (
              <button
                key={t.id}
                className={`nav ${filter === t.id ? "active" : ""}`}
                onClick={() => setFilter(t.id)}
                title={`${t.name} · ${t.error ? "配置需检查" : t.exists ? "已发现配置" : "未发现配置"}`}
              >
                <ToolIcon id={t.adapterId} />
                <span>{t.name}</span>
                <small className={t.error ? "warning" : ""}>
                  {t.error ? "!" : t.serviceCount}
                </small>
              </button>
            ))}
          </div>
          <div className="sidebar-bottom">
            <button className="nav" onClick={() => discover(firstFound)}>
              <FolderSearch size={17} />
              <span>发现本机配置</span>
            </button>
            <button
              className="nav"
              onClick={() => {
                setTransfer("");
                setAdapterId("claude");
                setView("import");
              }}
            >
              <Download size={17} />
              <span>导入配置</span>
            </button>
            <button className="nav" onClick={() => setView("history")}>
              <HistoryIcon size={17} />
              <span>同步记录</span>
            </button>
            <button className="nav" onClick={() => setView("tools")}>
              <Settings2 size={17} />
              <span>工具与路径</span>
            </button>
            <div className="workspace-note">
              <span className="dot" />
              {detected} 个目标已发现配置
            </div>
          </div>
        </aside>
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
                  onClick={() => showExport([])}
                  disabled={!services.length}
                >
                  <Upload size={17} />
                </button>
                <button
                  className="icon-button"
                  aria-label="添加服务"
                  onClick={() => setEditing(null)}
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
                onChange={(e) => setQuery(e.target.value)}
                placeholder="搜索 MCP 服务…"
                aria-label="搜索 MCP 服务"
              />
              {query ? (
                <button
                  className="icon-button"
                  onClick={() => setQuery("")}
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
                className={`service ${s.id === service?.id ? "selected" : ""}`}
                key={s.id}
                aria-pressed={s.id === service?.id}
                onClick={() => {
                  setSelected(s.id);
                  setTab("overview");
                  setChecks(null);
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
                      const target = data.targets.find((x) => x.id === t);
                      return target ? (
                        <ToolIcon key={t} id={target.adapterId} small />
                      ) : null;
                    })}
                    {s.targets.length > 5 && (
                      <small>+{s.targets.length - 5}</small>
                    )}
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
                  <button
                    className="text-button"
                    onClick={() => setFilter("all")}
                  >
                    查看全部服务
                  </button>
                )}
              </div>
            )}
          </div>
          <div className="list-bottom">
            <span>{data.adapters.length} 种适配器</span>
            <span>仅在本机保存</span>
          </div>
        </section>
        <section className="detail" aria-label="服务详情">
          {service ? (
            <>
              <div className="detail-top">
                <div className="identity">
                  <span className="large-icon">
                    <Plug size={25} />
                  </span>
                  <div>
                    <h2>{service.name}</h2>
                    <p>{service.description || service.key}</p>
                  </div>
                </div>
                <div className="inline">
                  <button onClick={() => showExport([service.id])}>导出</button>
                  <button
                    className="icon-button danger"
                    aria-label={`移除 ${service.name}`}
                    disabled={busy}
                    onClick={async () => {
                      if (
                        await perform(
                          "remove",
                          { serviceId: service.id, undo: false },
                          "已移除服务；目标文件变更将在应用后生效",
                        )
                      )
                        setUndo(service.id);
                    }}
                  >
                    <Trash2 size={17} />
                  </button>
                </div>
              </div>
              <div className="tabs" role="tablist">
                <button
                  role="tab"
                  aria-selected={tab === "overview"}
                  className={tab === "overview" ? "active" : ""}
                  onClick={() => setTab("overview")}
                >
                  概览
                </button>
                <button
                  role="tab"
                  aria-selected={tab === "json"}
                  className={tab === "json" ? "active" : ""}
                  onClick={() => setTab("json")}
                >
                  配置 JSON
                </button>
                <button
                  role="tab"
                  aria-selected={tab === "checks"}
                  className={tab === "checks" ? "active" : ""}
                  onClick={() => checkService(service)}
                >
                  配置检查
                </button>
              </div>
              {tab === "overview" ? (
                <>
                  <div className="section-heading">
                    <h3>服务配置</h3>
                    <button
                      className="text-button"
                      onClick={() => setEditing(service)}
                    >
                      编辑配置
                    </button>
                  </div>
                  <div className="settings">
                    <div>
                      <span>配置键</span>
                      <strong>{service.key}</strong>
                    </div>
                    <div>
                      <span>传输方式</span>
                      <strong>
                        <span className="tag">
                          {service.config.transport === "http"
                            ? "Streamable HTTP"
                            : service.config.transport}
                        </span>
                      </strong>
                    </div>
                    <div>
                      <span>
                        {service.config.transport === "stdio"
                          ? "启动命令"
                          : "服务地址"}
                      </span>
                      <code>
                        {service.config.command || service.config.url}
                      </code>
                    </div>
                    <div>
                      <span>分配状态</span>
                      <strong>{status(service)}</strong>
                    </div>
                  </div>
                  <div className="section-heading">
                    <h3>分配到工具</h3>
                    <span className="muted">
                      {service.targets.length} / {data.targets.length} 个目标
                    </span>
                  </div>
                  <div className="assignment">
                    {data.targets.map((t) => {
                      const a = adapter(t.adapterId);
                      const checked = service.targets.includes(t.id);
                      const compatible =
                        a.transports.includes(service.config.transport) &&
                        (a.supportsCwd || !service.config.cwd);
                      const change = serviceChanges.find(
                        (c) => c.targetId === t.id,
                      );
                      return (
                        <div className="target-row" key={t.id}>
                          <ToolIcon id={t.adapterId} />
                          <div>
                            <strong>{t.name}</strong>
                            <small className={change ? "warning" : ""}>
                              {!compatible
                                ? "不支持此配置中的传输方式或 cwd"
                                : change?.conflict
                                  ? "外部配置变更 · 需要处理"
                                  : change
                                    ? `${actionNames[change.action]} · 待应用`
                                    : checked
                                      ? "配置已对齐 · 加载状态由客户端确认"
                                      : t.exists
                                        ? "未分配"
                                        : "未发现配置 · 应用时创建文件"}
                            </small>
                          </div>
                          <button
                            role="switch"
                            aria-checked={checked}
                            aria-label={`${service.name} 分配到 ${t.name}`}
                            className="switch"
                            disabled={busy || (!compatible && !checked)}
                            onClick={() =>
                              perform(
                                "assign",
                                {
                                  serviceId: service.id,
                                  targetId: t.id,
                                  enabled: !checked,
                                },
                                "分配已暂存，预览后写入",
                              )
                            }
                          />
                        </div>
                      );
                    })}
                  </div>
                  <p className="hint">
                    <ShieldCheck size={15} />
                    分配控制配置文件中的服务条目。客户端可能仍需刷新、授权或受项目配置覆盖。
                  </p>
                </>
              ) : tab === "json" ? (
                <>
                  <div className="section-heading">
                    <h3>服务公共配置</h3>
                    <button
                      className="text-button"
                      onClick={() => setEditing(service)}
                    >
                      编辑完整配置
                    </button>
                  </div>
                  <p className="hint">
                    环境变量、请求头已隐藏。导出时选择目标工具，会转换为对应格式。
                  </p>
                  <Code
                    value={{
                      ...service.config,
                      env: Object.fromEntries(
                        Object.keys(service.config.env).map((k) => [
                          k,
                          "<已隐藏>",
                        ]),
                      ),
                      headers: Object.fromEntries(
                        Object.keys(service.config.headers).map((k) => [
                          k,
                          "<已隐藏>",
                        ]),
                      ),
                    }}
                  />
                </>
              ) : (
                <>
                  <div className="section-heading">
                    <h3>配置检查</h3>
                    <button
                      className="text-button"
                      onClick={() => checkService(service)}
                    >
                      重新检查
                    </button>
                  </div>
                  {checks ? (
                    <>
                      <div
                        className={`callout ${checks.issues.length ? "warn" : ""}`}
                      >
                        <strong>
                          {checks.issues.length
                            ? "有需要确认的配置"
                            : "字段检查通过"}
                        </strong>
                        {checks.issues.map((i, n) => (
                          <p key={n}>{i}</p>
                        ))}
                        {checks.executable && (
                          <p>
                            发现命令：<code>{checks.executable}</code>
                          </p>
                        )}
                      </div>
                      <p className="hint">{checks.note}</p>
                    </>
                  ) : (
                    <p className="muted">正在检查…</p>
                  )}
                </>
              )}
            </>
          ) : (
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
                <button
                  className="primary"
                  onClick={() => discover(firstFound)}
                >
                  <FolderSearch size={17} />
                  发现本机配置
                  <ArrowRight size={16} />
                </button>
                <button onClick={() => setEditing(null)}>
                  <Plus size={17} />
                  添加服务
                </button>
              </div>
              <div className="welcome-tools">
                {data.adapters.map((a) => (
                  <ToolIcon key={a.id} id={a.id} />
                ))}
              </div>
              <span className="muted">
                已内置 {data.adapters.length} 种适配器 · 本地配置管理
              </span>
              {filter !== "all" && (
                <button
                  className="text-button"
                  onClick={() => setFilter("all")}
                >
                  <ArrowLeft size={14} />
                  返回全部服务
                </button>
              )}
            </div>
          )}
        </section>
      </main>
      <footer className="statusbar">
        <span>
          <span className="dot" />
          {data.isolated ? "隔离测试目录" : "本地工作区"} · 配置变更需手动应用
        </span>
        <span>
          {preview?.errors.length
            ? `${preview.errors.length} 个配置问题`
            : busy
              ? "正在处理…"
              : `已保存 · 修订 ${data.workspace.revision}`}
        </span>
      </footer>
      {error && !view && (
        <div className="floating-error">
          <ErrorBox text={error} />
          <button
            className="icon-button"
            onClick={() => setError("")}
            aria-label="关闭错误"
          >
            <X size={16} />
          </button>
        </div>
      )}
      {toast && (
        <div className="toast" role="status">
          <Check size={16} />
          {toast}
          {undo && (
            <button
              onClick={async () => {
                await perform(
                  "remove",
                  { serviceId: undo, undo: true },
                  "已恢复服务",
                );
                setUndo("");
              }}
            >
              撤销移除
            </button>
          )}
        </div>
      )}
      {editing !== undefined && (
        <Editor
          service={editing}
          onClose={() => setEditing(undefined)}
          onSaved={async (id) => {
            setSelected(id);
            setFilter("all");
            setQuery("");
            await refresh();
            notice("服务已保存");
          }}
        />
      )}
      {targetEditing !== undefined && (
        <TargetEditor
          snapshot={data}
          target={targetEditing}
          onClose={() => setTargetEditing(undefined)}
          onSaved={async () => {
            await refresh();
            notice("目标路径已保存");
          }}
        />
      )}

      {view === "tools" && (
        <Modal
          title="工具与配置路径"
          onClose={close}
          wide
          footer={
            <>
              <span className="muted grow">
                {data.targets.length} 个配置目标 · 路径可自定义
              </span>
              <button
                onClick={() => {
                  setView(null);
                  setTargetEditing(null);
                }}
              >
                <Plus size={16} />
                添加配置目标
              </button>
              <button className="primary" onClick={close}>
                完成
              </button>
            </>
          }
        >
          <ErrorBox text={error} />
          <p className="intro">
            每个目标对应一个配置文件。可以为同一工具添加其他
            Profile、扩展宿主或项目位置。
          </p>
          <div className="tools-list">
            {data.targets.map((t) => (
              <div className="tool-setting" key={t.id}>
                <ToolIcon id={t.adapterId} />
                <div>
                  <strong>
                    {t.name}
                    <span className={`tag ${t.error ? "warning" : ""}`}>
                      {t.error
                        ? "需要检查"
                        : t.exists
                          ? `${t.count} 个配置`
                          : "未发现配置"}
                    </span>
                  </strong>
                  <code>{t.path}</code>
                  <small>{t.error || adapter(t.adapterId).note}</small>
                </div>
                <button
                  onClick={() => {
                    setView(null);
                    setTargetEditing(t);
                  }}
                >
                  设置
                </button>
                <button
                  disabled={!t.exists || !!t.error}
                  onClick={() => discover(t.id)}
                >
                  发现
                </button>
              </div>
            ))}
          </div>
          <p className="hint">应用数据目录：{data.dataDir}</p>
        </Modal>
      )}
      {view === "discover" && (
        <Modal
          title="发现本机 MCP 配置"
          onClose={close}
          wide
          footer={
            <>
              <span className="muted grow">纳入管理不会修改目标文件</span>
              <button onClick={close}>取消</button>
              <button
                className="primary"
                disabled={busy || !chosenKeys.length}
                onClick={async () => {
                  if (
                    await perform(
                      "adopt",
                      { targetId: discoveryTarget, keys: chosenKeys },
                      "已纳入服务库，原配置保持不变",
                    )
                  ) {
                    setFilter("all");
                    setView(null);
                  }
                }}
              >
                纳入管理 {chosenKeys.length ? `(${chosenKeys.length})` : ""}
              </button>
            </>
          }
        >
          <label>
            目标工具
            <Select
              label="目标工具"
              value={discoveryTarget}
              onChange={discover}
              disabled={busy}
              options={data.targets.map((t) => ({
                value: t.id,
                label: t.name,
                detail: t.exists ? `${t.count} 个配置` : "未发现配置",
                icon: <ToolIcon id={t.adapterId} small />,
              }))}
            />
          </label>
          <p className="path-label">
            {data.targets.find((t) => t.id === discoveryTarget)?.path}
          </p>
          <ErrorBox text={error} />
          {busy ? (
            <p className="muted">正在读取…</p>
          ) : discovered.length ? (
            <div className="discover-list">
              <div className="discover-controls">
                <label className={`discover-select-all ${!importableKeys.length ? "is-disabled" : ""}`}>
                  <input
                    type="checkbox"
                    checked={allChosen}
                    ref={(input) => {
                      if (input) input.indeterminate = chosenKeys.length > 0 && !allChosen;
                    }}
                    disabled={!importableKeys.length}
                    onChange={(e) => setChosen(e.target.checked ? importableKeys : [])}
                  />
                  全选可导入项
                </label>
                <span className="discover-selection" role="status" aria-live="polite">
                  {importableKeys.length
                    ? `已选 ${chosenKeys.length} / ${importableKeys.length} 项`
                    : "没有可导入项"}
                </span>
              </div>
              {discovered.map((d) => {
                const unsupported = !!d.error || !d.config;
                const unavailable = unsupported || d.managed;
                const checked = chosenKeys.includes(d.key);
                return (
                  <label
                    className={`discover-item ${unsupported ? "is-unsupported" : d.managed ? "is-managed" : checked ? "is-selected" : ""}`}
                    key={d.key}
                  >
                    <input
                      type="checkbox"
                      checked={checked}
                      disabled={unavailable}
                      onChange={(e) => {
                        const checked = e.target.checked;
                        setChosen((c) => checked ? [...c, d.key] : c.filter((k) => k !== d.key));
                      }}
                    />
                    {unsupported ? <AlertTriangle size={20} /> : <Plug size={20} />}
                    <div>
                      <strong>{d.key}</strong>
                      <small>
                        {unsupported
                          ? d.error || "无法识别此配置，原配置保持不变"
                          : d.managed ? "已纳入服务库，无需重复导入" : `${d.config?.transport} · 可导入`}
                      </small>
                    </div>
                    <span className="tag">
                      {unsupported ? "不支持" : d.managed ? "已管理" : checked ? "已选择" : "可导入"}
                    </span>
                  </label>
                );
              })}
            </div>
          ) : (
            <div className="empty-card">
              <FolderSearch size={28} />
              <h3>这里还没有 MCP 配置</h3>
              <p>选择其他工具，或通过「工具与路径」指定文件。</p>
            </div>
          )}
          <p className="hint">
            同名服务会保留独立来源；未勾选及不支持的条目会保留在原文件中。
          </p>
        </Modal>
      )}
      {view === "import" && (
        <Modal
          title="导入配置"
          onClose={close}
          wide
          footer={
            <>
              <button onClick={close}>取消</button>
              <button
                className="primary"
                disabled={busy || !transfer.trim()}
                onClick={async () => {
                  if (
                    await perform(
                      "importText",
                      { adapterId, text: transfer },
                      "已导入服务库，请按需分配",
                    )
                  ) {
                    setView(null);
                    setFilter("all");
                  }
                }}
              >
                导入到服务库
              </button>
            </>
          }
        >
          <ErrorBox text={error} />
          <label>
            来源配置格式
            <Select
              label="来源配置格式"
              value={adapterId}
              onChange={setAdapterId}
              options={data.adapters.map((a) => ({
                value: a.id,
                label: a.name,
                detail: a.format,
                icon: <ToolIcon id={a.id} small />,
              }))}
            />
          </label>
          <p className="intro">
            粘贴对应工具的配置文件。导入后不自动分配或执行命令。
          </p>
          <label className="file-picker">
            <Download size={15} />
            选择配置文件
            <input
              type="file"
              accept=".json,.jsonc,.toml"
              onChange={async (e) => {
                const file = e.target.files?.[0];
                if (!file) return;
                if (file.size > 1024 * 1024) {
                  setError("文件不能超过 1 MB");
                  return;
                }
                setTransfer(await file.text());
              }}
            />
          </label>
          <textarea
            className="transfer-code"
            value={transfer}
            onChange={(e) => setTransfer(e.target.value)}
            rows={13}
            aria-label="导入配置内容"
            spellCheck={false}
          />
        </Modal>
      )}
      {view === "export" && (
        <Modal
          title="导出目标工具配置"
          onClose={close}
          wide
          footer={
            <>
              <span className="muted grow">
                {secrets
                  ? "将包含完整参数与凭据，请妥善保存"
                  : "默认导出脱敏模板，补齐隐藏值后使用"}
              </span>
              <button onClick={close}>关闭</button>
              <button
                className="primary"
                disabled={busy || !transfer}
                onClick={download}
              >
                <Upload size={15} />
                保存文件
              </button>
            </>
          }
        >
          <label>
            目标配置格式
            <Select
              label="目标配置格式"
              value={adapterId}
              onChange={(value) => {
                setAdapterId(value);
                generateExport(value, exportIds, secrets);
              }}
              options={data.adapters.map((a) => ({
                value: a.id,
                label: a.name,
                icon: <ToolIcon id={a.id} small />,
              }))}
            />
          </label>
          <label className="checkbox-line">
            <input
              type="checkbox"
              checked={secrets}
              onChange={(e) => {
                setSecrets(e.target.checked);
                generateExport(adapterId, exportIds, e.target.checked);
              }}
            />
            显示并导出完整配置（包含参数、环境变量与凭据）
          </label>
          <ErrorBox text={error} />
          <Code value={transfer || "正在生成…"} />
        </Modal>
      )}
      {view === "preview" && preview && (
        <Modal
          title="同步变更预览"
          onClose={close}
          wide
          footer={
            <>
              <span className="muted grow">先备份，再逐文件写入并校验</span>
              <button onClick={close}>返回</button>
              <button
                className="primary"
                disabled={
                  busy ||
                  !!preview.errors.length ||
                  !preview.changes.length ||
                  preview.changes.some((c) => c.conflict)
                }
                onClick={async () => {
                  if (
                    await perform(
                      "apply",
                      { id: preview.id },
                      "配置已写入，请在目标工具中刷新或授权",
                    )
                  )
                    setView(null);
                }}
              >
                应用 {preview.changes.length} 项变更
              </button>
            </>
          }
        >
          <ErrorBox text={error} />
          {preview.errors.map((e, i) => (
            <ErrorBox key={i} text={e} />
          ))}
          <label className="checkbox-line">
            <input
              type="checkbox"
              checked={revealPreview}
              disabled={busy}
              onChange={async (e) => {
                const value = e.target.checked;
                setBusy(true);
                try {
                  setPreview(
                    await request<Preview>("preview", { reveal: value }),
                  );
                  setRevealPreview(value);
                } catch (e) {
                  setError(String(e));
                } finally {
                  setBusy(false);
                }
              }}
            />
            显示完整差异（包含命令参数、环境变量与凭据）
          </label>
          <div className="preview-summary">
            <strong>{preview.changes.length}</strong>
            <span>项变更，涉及 {preview.fileCount} 个配置文件</span>
          </div>
          {!preview.changes.length && !preview.errors.length && (
            <div className="empty-card">
              <Check size={30} />
              <h3>配置已对齐</h3>
              <p>没有需要写入的服务变更。</p>
            </div>
          )}
          {preview.changes.map((c, i) => (
            <div
              className={`change ${c.conflict ? "conflict" : ""}`}
              key={`${c.targetId}-${c.serviceId}-${i}`}
            >
              <div className="change-header">
                <strong>{c.key}</strong>
                <span>{c.targetName}</span>
                <span className="tag">{actionNames[c.action]}</span>
              </div>
              <div className="diff-grid">
                <div>
                  <small>当前磁盘配置</small>
                  <Code value={c.before || "无此配置"} />
                </div>
                <div>
                  <small>服务库期望配置</small>
                  <Code value={c.after || "移除此服务条目"} />
                </div>
              </div>
              {c.conflict && (
                <div className="conflict-actions">
                  <p>
                    <AlertTriangle size={15} />
                    {c.message}
                  </p>
                  <button
                    disabled={busy}
                    onClick={() =>
                      perform(
                        "resolve",
                        {
                          serviceId: c.serviceId,
                          targetId: c.targetId,
                          useDisk: true,
                        },
                        "已采用磁盘版本；其他目标变更请重新检查",
                      )
                    }
                  >
                    采用磁盘版本
                  </button>
                  <button
                    disabled={busy}
                    onClick={() =>
                      perform(
                        "resolve",
                        {
                          serviceId: c.serviceId,
                          targetId: c.targetId,
                          useDisk: false,
                        },
                        "已确认保留服务库版本，请检查新的预览",
                      )
                    }
                  >
                    保留服务库版本
                  </button>
                </div>
              )}
            </div>
          ))}
          <p className="hint">
            敏感字段与命令参数默认隐藏，请在服务编辑器确认具体内容。未管理服务与其他配置项保留；客户端加载与认证需分别确认。
          </p>
        </Modal>
      )}
      {view === "history" && (
        <Modal
          title="同步记录与恢复"
          onClose={close}
          wide
          footer={
            <button className="primary" onClick={close}>
              完成
            </button>
          }
        >
          <ErrorBox text={error} />
          <p className="intro">
            每次写入均保留原始文件备份。恢复只在文件仍匹配本次写入结果时执行，避免覆盖后来修改。
          </p>
          {!data.workspace.history.length ? (
            <div className="empty-card">
              <HistoryIcon size={30} />
              <h3>还没有同步记录</h3>
              <p>应用配置后，会在这里留下记录与备份。</p>
            </div>
          ) : (
            <div className="history-list">
              {[...data.workspace.history].reverse().map((h) => (
                <div className="history-item" key={h.id}>
                  <div className="history-heading">
                    <strong>{historyNames[h.status] || h.status}</strong>
                    <time>{new Date(h.at * 1000).toLocaleString("zh-CN")}</time>
                    {["applied", "recovery-needed"].includes(h.status) && (
                      <button
                        disabled={busy}
                        onClick={() =>
                          perform(
                            "rollback",
                            { id: h.id },
                            "已恢复文件，服务库期望值保留",
                          )
                        }
                      >
                        恢复原文件
                      </button>
                    )}
                    {h.status === "recovery-needed" && (
                      <button
                        disabled={busy}
                        onClick={() =>
                          perform(
                            "keepRecovery",
                            { id: h.id },
                            "已保留磁盘现状，并重建同步基线",
                          )
                        }
                      >
                        保留当前文件
                      </button>
                    )}
                  </div>
                  <p>{h.summary}</p>
                  {h.paths.map((p) => (
                    <code key={p}>{p}</code>
                  ))}
                </div>
              ))}
            </div>
          )}
        </Modal>
      )}
    </div>
  );
}

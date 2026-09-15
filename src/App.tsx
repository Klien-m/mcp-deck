/** 工作区装配入口：组合数据操作、服务筛选与视图路由，不直接调用写入 IPC。 */
import { useEffect, useState } from "react";
import { Check, X } from "lucide-react";
import { ErrorBox } from "./components";
import { ServiceDetail } from "./features/services/ServiceDetail";
import { ServiceList } from "./features/services/ServiceList";
import { useServiceLibrary } from "./features/services/useServiceLibrary";
import { useWorkspace } from "./hooks/useWorkspace";
import { Sidebar } from "./layout/Sidebar";
import { WorkspaceLayout } from "./layout/WorkspaceLayout";
import { EmptyWorkspace, Startup } from "./layout/WorkspaceStates";
import { WorkspaceDialogs } from "./layout/WorkspaceDialogs";
import type { WorkspaceView } from "./layout/WorkspaceDialogs";
import type { Change, Service, TargetStatus } from "./types";

// 稳定的空数组让初始化阶段也能无条件调用 Hook，避免每次渲染产生新的依赖引用。
const emptyServices: Service[] = [];
const emptyTargets: TargetStatus[] = [];
const emptyChanges: Change[] = [];

/** 保存当前弹窗、通知和撤销入口；服务选择归 useServiceLibrary，持久化归 useWorkspace。 */
export default function App() {
  const [view, setView] = useState<WorkspaceView | null>(null);
  const [toast, setToast] = useState("");
  const [undo, setUndo] = useState("");
  const { data, preview, fatal, error, busy, clearError, actions } =
    useWorkspace({
      includeDetails: view?.type === "preview",
      onNotice: (message) => {
        setToast(message);
        setUndo("");
      },
    });
  const library = useServiceLibrary(
    data?.workspace.services ?? emptyServices,
    data?.targets ?? emptyTargets,
    preview?.changes ?? emptyChanges,
  );

  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(""), 7000);
    return () => clearTimeout(timer);
  }, [toast]);

  /** 切换视图时清除上一操作错误，避免把旧弹窗的失败提示带到新表单。 */
  function show(next: WorkspaceView | null) {
    clearError();
    setView(next);
  }
  /** 先成功获取含完整差异的计划，再打开弹窗；后续明文切换只读本地数据。 */
  async function showPreview() {
    if (await actions.loadPreview()) show({ type: "preview" });
  }
  // 空数组表示导出整个服务库；按服务导出时传入具体 ID，保持弹窗期间数组引用稳定。
  const showExport = (serviceIds: string[]) =>
    show({ type: "export", serviceIds });
  const addService = () => show({ type: "service", service: null });

  if (!data) return <Startup fatal={fatal || error} onRetry={actions.reload} />;
  const { service } = library;
  const discover = () =>
    show({
      type: "discover",
      targetId: data.targets.find((t) => t.exists)?.id || "codex",
    });
  return (
    <WorkspaceLayout
      isolated={data.isolated}
      busy={busy}
      pendingCount={preview?.changes.length || 0}
      issueCount={preview?.errors.length || 0}
      revision={data.workspace.revision}
      onRefresh={actions.reload}
      onPreview={showPreview}
      overlays={
        <>
          {error && !view && (
            <div className="floating-error">
              <ErrorBox text={error} />
              <button
                className="icon-button"
                onClick={clearError}
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
                  disabled={busy}
                  onClick={async () => {
                    if (await actions.remove(undo, true)) setUndo("");
                  }}
                >
                  撤销移除
                </button>
              )}
            </div>
          )}
          <WorkspaceDialogs
            view={view}
            adapters={data.adapters}
            targets={data.targets}
            history={data.workspace.history}
            dataDir={data.dataDir}
            preview={preview}
            error={error}
            busy={busy}
            actions={actions}
            onView={show}
            onClearError={clearError}
            onServiceSaved={library.revealSaved}
            onImported={() => library.setFilter("all")}
          />
        </>
      }
    >
      <Sidebar
        filter={library.filter}
        onFilter={library.setFilter}
        serviceCount={library.services.length}
        pendingCount={preview?.changes.length || 0}
        toolTargets={library.toolTargets}
        detected={data.targets.filter((t) => t.exists).length}
        onDiscover={discover}
        onImport={() => show({ type: "import" })}
        onHistory={() => show({ type: "history" })}
        onTools={() => show({ type: "tools" })}
      />
      <ServiceList
        visible={library.visible}
        selectedId={service?.id}
        targets={data.targets}
        pendingIds={library.pendingIds}
        status={library.status}
        filter={library.filter}
        query={library.query}
        serviceCount={library.services.length}
        adapterCount={data.adapters.length}
        onSelect={library.select}
        onFilter={library.setFilter}
        onQuery={library.setQuery}
        onExport={() => showExport([])}
        onAdd={addService}
      />
      <section className="detail" aria-label="服务详情">
        {service ? (
          <ServiceDetail
            key={`${service.id}-${library.selected.version}`}
            service={service}
            targets={data.targets}
            adapters={data.adapters}
            changes={
              preview?.changes.filter((c) => c.serviceId === service.id) || []
            }
            busy={busy}
            status={library.status(service)}
            onAssign={(targetId, enabled) =>
              actions.assign(service.id, targetId, enabled)
            }
            onRemove={() => actions.remove(service.id)}
            onRemoved={setUndo}
            onEdit={(service) => show({ type: "service", service })}
            onExport={showExport}
          />
        ) : (
          <EmptyWorkspace
            adapters={data.adapters}
            filter={library.filter}
            onDiscover={discover}
            onAdd={addService}
            onShowAll={() => library.setFilter("all")}
          />
        )}
      </section>
    </WorkspaceLayout>
  );
}

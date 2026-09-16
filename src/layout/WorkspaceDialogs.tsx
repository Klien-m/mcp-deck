import { ServiceEditor } from "../features/services/ServiceEditor";
import { TargetEditor } from "../features/targets/TargetEditor";
import { ToolsDialog } from "../features/targets/ToolsDialog";
import { DiscoveryDialog } from "../features/targets/DiscoveryDialog";
import { OnboardingDialog } from "../features/targets/OnboardingDialog";
import { ImportDialog } from "../features/transfer/ImportDialog";
import { ExportDialog } from "../features/transfer/ExportDialog";
import { PreviewDialog } from "../features/sync/PreviewDialog";
import { HistoryDialog } from "../features/sync/HistoryDialog";
import type { WorkspaceActions } from "../hooks/useWorkspace";
import type {
  Adapter,
  History,
  Preview,
  Service,
  Target,
  TargetStatus,
} from "../types";

/** 互斥弹窗路由及其初始化数据；null 路由代表无弹窗，service/target 为 null 则表示新建。 */
export type WorkspaceView =
  | { type: "service"; service: Service | null }
  | { type: "target"; target: Target | null }
  | { type: "discover"; targetId: string }
  | { type: "onboarding" }
  | { type: "export"; serviceIds: string[] }
  | { type: "import" | "preview" | "history" | "tools" };

type DialogActions = Pick<
  WorkspaceActions,
  | "saveService"
  | "saveTarget"
  | "adopt"
  | "completeOnboarding"
  | "importText"
  | "saveExport"
  | "apply"
  | "resolve"
  | "rollback"
  | "keepRecovery"
>;

/**
 * 弹窗装配边界：把工作区动作裁剪为组件所需回调，并统一处理保存成功后的路由变化。
 * 组件自行管理表单与读取状态，此处不复制快照，也不直接执行 IPC。
 */
export function WorkspaceDialogs({
  view,
  adapters,
  targets,
  history,
  dataDir,
  preview,
  error,
  busy,
  stale,
  actions,
  onView,
  onClearError,
  onServiceSaved,
  onImported,
}: {
  view: WorkspaceView | null;
  adapters: Adapter[];
  targets: TargetStatus[];
  history: History[];
  dataDir: string;
  preview: Preview | null;
  error: string;
  busy: boolean;
  stale: boolean;
  actions: DialogActions;
  onView: (view: WorkspaceView | null) => void;
  onClearError: () => void;
  onServiceSaved: (id: string) => void;
  onImported: () => void;
}) {
  // 写操作未完成时保留弹窗；导出、发现等组件还会叠加自身的本地忙碌条件。
  const close = () => {
    if (!busy) onView(null);
  };
  const imported = () => {
    onImported();
    onView(null);
  };
  const common = { error, busy, onClose: close };
  switch (view?.type) {
    case "onboarding":
      return (
        <OnboardingDialog
          error={error}
          busy={busy}
          onComplete={actions.completeOnboarding}
          onCompleted={imported}
          onClearError={onClearError}
        />
      );
    case "service":
      return (
        <ServiceEditor
          {...common}
          service={view.service}
          onSave={async (input) => {
            const id = await actions.saveService(input);
            if (id === null) return false;
            onServiceSaved(id);
            return true;
          }}
        />
      );
    case "target":
      return (
        <TargetEditor
          {...common}
          adapters={adapters}
          target={view.target}
          onSave={actions.saveTarget}
        />
      );
    case "tools":
      return (
        <ToolsDialog
          adapters={adapters}
          targets={targets}
          dataDir={dataDir}
          error={error}
          onClose={close}
          onEdit={(target) => onView({ type: "target", target })}
          onDiscover={(targetId) => onView({ type: "discover", targetId })}
        />
      );
    case "discover":
      return (
        <DiscoveryDialog
          {...common}
          targets={targets}
          initialTarget={view.targetId}
          onClearError={onClearError}
          onAdopt={actions.adopt}
          onAdopted={imported}
        />
      );
    case "import":
      return (
        <ImportDialog
          {...common}
          adapters={adapters}
          onImport={actions.importText}
          onImported={imported}
        />
      );
    case "export":
      return (
        <ExportDialog
          {...common}
          adapters={adapters}
          serviceIds={view.serviceIds}
          onSave={actions.saveExport}
          onExported={close}
        />
      );
    // 不用预览 ID 作为组件 key；计划刷新时沿用 Modal 实例，避免失焦或视觉闪烁。
    case "preview":
      return (
        preview && (
          <PreviewDialog
            {...common}
            stale={stale}
            preview={preview}
            onApply={actions.apply}
            onResolve={actions.resolve}
            onApplied={close}
          />
        )
      );
    case "history":
      return (
        <HistoryDialog
          {...common}
          history={history}
          onRollback={actions.rollback}
          onKeepRecovery={actions.keepRecovery}
        />
      );
    default:
      return null;
  }
}

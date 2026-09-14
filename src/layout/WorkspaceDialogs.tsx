import { ServiceEditor } from "../features/services/ServiceEditor";
import { TargetEditor } from "../features/targets/TargetEditor";
import { ToolsDialog } from "../features/targets/ToolsDialog";
import { DiscoveryDialog } from "../features/targets/DiscoveryDialog";
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

export type WorkspaceView =
  | { type: "service"; service: Service | null }
  | { type: "target"; target: Target | null }
  | { type: "discover"; targetId: string }
  | { type: "export"; serviceIds: string[] }
  | { type: "import" | "preview" | "history" | "tools" };

type DialogActions = Pick<
  WorkspaceActions,
  | "saveService"
  | "saveTarget"
  | "adopt"
  | "importText"
  | "saveExport"
  | "apply"
  | "resolve"
  | "rollback"
  | "keepRecovery"
>;

// This is the composition boundary. Feature components receive only their own inputs/actions.
export function WorkspaceDialogs({
  view,
  adapters,
  targets,
  history,
  dataDir,
  preview,
  error,
  busy,
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
  actions: DialogActions;
  onView: (view: WorkspaceView | null) => void;
  onClearError: () => void;
  onServiceSaved: (id: string) => void;
  onImported: () => void;
}) {
  const close = () => {
    if (!busy) onView(null);
  };
  const imported = () => {
    onImported();
    onView(null);
  };
  const common = { error, busy, onClose: close };
  switch (view?.type) {
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
    case "preview":
      return (
        preview && (
          <PreviewDialog
            {...common}
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

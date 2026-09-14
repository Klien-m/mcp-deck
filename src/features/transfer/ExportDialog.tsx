import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { Upload } from "lucide-react";
import { request } from "../../api";
import type { CommandArgs } from "../../api";
import { Code, ErrorBox, Modal, ToolIcon } from "../../components";
import { Select } from "../../Select";
import type { Adapter } from "../../types";

export function ExportDialog({
  adapters,
  error: workspaceError,
  busy: workspaceBusy,
  onSave,
  serviceIds,
  onClose,
  onExported,
}: {
  adapters: Adapter[];
  error: string;
  busy: boolean;
  onSave: (options: CommandArgs<"saveExport">) => Promise<boolean>;
  serviceIds: string[];
  onClose: () => void;
  onExported: () => void;
}) {
  const [adapterId, setAdapterId] = useState("claude");
  const [secrets, setSecrets] = useState(false);
  const [result, setResult] = useState<{
    adapterId: string;
    serviceIds: string[];
    secrets: boolean;
    text: string;
  } | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const requestId = useRef(0);
  const saveInProgress = useRef(false);
  const busy = loading || saving || workspaceBusy;
  const transfer =
    result?.adapterId === adapterId &&
    result.serviceIds === serviceIds &&
    result.secrets === secrets
      ? result.text
      : "";

  useEffect(() => {
    const id = ++requestId.current;
    setLoading(true);
    setError("");
    setResult(null);
    request("export", { adapterId, serviceIds, includeSecrets: secrets })
      .then((text) => {
        if (id === requestId.current)
          setResult({ adapterId, serviceIds, secrets, text });
      })
      .catch((e) => {
        if (id === requestId.current) setError(String(e));
      })
      .finally(() => {
        if (id === requestId.current) setLoading(false);
      });
    return () => {
      requestId.current++;
    };
  }, [adapterId, serviceIds, secrets]);

  const close = () => {
    if (!busy && !saveInProgress.current) onClose();
  };
  async function download() {
    if (busy || !transfer || saveInProgress.current) return;
    saveInProgress.current = true;
    setSaving(true);
    setError("");
    try {
      const adapter = adapters.find((a) => a.id === adapterId);
      const path = await save({
        defaultPath:
          adapterId === "codex"
            ? "mcp-deck-export.toml"
            : "mcp-deck-export.json",
        filters: [
          {
            name: adapter?.format || "配置",
            extensions: [adapterId === "codex" ? "toml" : "json"],
          },
        ],
      });
      if (path) {
        const saved = await onSave({
          adapterId,
          serviceIds,
          includeSecrets: secrets,
          path,
        });
        if (saved) onExported();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      saveInProgress.current = false;
      setSaving(false);
    }
  }
  return (
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
          onChange={setAdapterId}
          disabled={saving || workspaceBusy}
          options={adapters.map((a) => ({
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
          disabled={saving || workspaceBusy}
          onChange={(e) => setSecrets(e.target.checked)}
        />
        显示并导出完整配置（包含参数、环境变量与凭据）
      </label>
      <ErrorBox text={error || workspaceError} />
      <Code value={transfer || "正在生成…"} />
    </Modal>
  );
}

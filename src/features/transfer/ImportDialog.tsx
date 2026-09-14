import { useState } from "react";
import { Download } from "lucide-react";
import { ErrorBox, Modal, ToolIcon } from "../../components";
import { Select } from "../../Select";
import type { Adapter } from "../../types";

export function ImportDialog({
  adapters,
  error,
  busy,
  onImport,
  onClose: close,
  onImported,
}: {
  adapters: Adapter[];
  error: string;
  busy: boolean;
  onImport: (adapterId: string, text: string) => Promise<boolean>;
  onClose: () => void;
  onImported: () => void;
}) {
  const [adapterId, setAdapterId] = useState("claude");
  const [transfer, setTransfer] = useState("");
  const [localError, setLocalError] = useState("");
  return (
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
              setLocalError("");
              if (await onImport(adapterId, transfer)) {
                onImported();
              }
            }}
          >
            导入到服务库
          </button>
        </>
      }
    >
      <ErrorBox text={localError || error} />
      <label>
        来源配置格式
        <Select
          label="来源配置格式"
          value={adapterId}
          onChange={setAdapterId}
          disabled={busy}
          options={adapters.map((a) => ({
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
          disabled={busy}
          accept=".json,.jsonc,.toml"
          onChange={async (e) => {
            const file = e.target.files?.[0];
            if (!file) return;
            if (file.size > 1024 * 1024) {
              setLocalError("文件不能超过 1 MB");
              return;
            }
            setLocalError("");
            try {
              setTransfer(await file.text());
            } catch (e) {
              setLocalError(String(e));
            }
          }}
        />
      </label>
      <textarea
        className="transfer-code"
        disabled={busy}
        value={transfer}
        onChange={(e) => {
          setLocalError("");
          setTransfer(e.target.value);
        }}
        rows={13}
        aria-label="导入配置内容"
        spellCheck={false}
      />
    </Modal>
  );
}

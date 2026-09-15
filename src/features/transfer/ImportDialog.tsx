import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { useState } from "react";
import { Download } from "lucide-react";
import { ErrorBox, Modal, ToolIcon } from "../../components";
import { Select } from "../../Select";
import type { Adapter } from "../../types";

/** 接收粘贴或文件文本，保留来源适配器选择；批量解析与原子导入由后端执行。 */
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
  // 本地文件读取错误与后端导入错误分开；输入修正或重试时清除旧本地错误。
  const [localError, setLocalError] = useState("");
  return (
    <Modal
      title="导入配置"
      onClose={close}
      wide
      footer={
        <>
          <Button variant="outline" onClick={close}>取消</Button>
          <Button variant="default"
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
          </Button>
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
        <Input
          type="file"
          disabled={busy}
          accept=".json,.jsonc,.toml"
          onChange={async (e) => {
            const file = e.target.files?.[0];
            if (!file) return;
            // 读取前限制 1 MB，后端也会按实际文本字节数再次校验，不能仅依赖文件扩展名。
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
      <Textarea
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

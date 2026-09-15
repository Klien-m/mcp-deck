import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Modal, ErrorBox, ToolIcon } from "../../components";
import { Select } from "../../Select";
import type { Adapter, Target } from "../../types";

/** 编辑单个目标草稿；target=null 时新建，真实路径约束与关联检查交给后端。 */
export function TargetEditor({
  adapters,
  target,
  onClose,
  onSave,
  busy,
  error: workspaceError,
}: {
  adapters: Adapter[];
  target: Target | null;
  onClose: () => void;
  onSave: (target: Target) => Promise<boolean>;
  busy: boolean;
  error: string;
}) {
  const [value, setValue] = useState<Target>(
    target || {
      id: "",
      adapterId: "codex",
      name: "Codex · 自定义位置",
      path: "",
    },
  );
  const [error, setError] = useState("");
  const adapter = adapters.find((a) => a.id === value.adapterId)!;
  /** 原生选择框只填入路径，不立即保存；用户取消时保留原草稿。 */
  async function choose() {
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "MCP 配置", extensions: ["json", "jsonc", "toml"] }],
      });
      if (typeof path === "string") setValue({ ...value, path });
    } catch (e) {
      setError(String(e));
    }
  }
  /** 通过统一写入口保存；拒绝结果留在表单显示，成功后才关闭。 */
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (busy) return;
    setError("");
    try {
      if (await onSave(value)) onClose();
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <Modal
      title={target ? "配置目标工具" : "添加配置目标"}
      onClose={() => !busy && onClose()}
      footer={
        <>
          <Button variant="outline" onClick={onClose} disabled={busy}>
            取消
          </Button>
          <Button variant="default" form="target-editor" className="primary" disabled={busy}>
            保存目标
          </Button>
        </>
      }
    >
      <form id="target-editor" onSubmit={submit}>
        <ErrorBox text={error || workspaceError} />
        <label>
          适配器
          <Select
            label="适配器"
            disabled={!!target}
            value={value.adapterId}
            onChange={(adapterId) =>
              setValue({
                ...value,
                adapterId,
                name:
                  adapters.find((a) => a.id === adapterId)!.name +
                  " · 自定义位置",
              })
            }
            options={adapters.map((a) => ({
              value: a.id,
              label: a.name,
              icon: <ToolIcon id={a.id} small />,
            }))}
          />
        </label>
        <label>
          目标名称
          <Input
            required
            value={value.name}
            onChange={(e) => setValue({ ...value, name: e.target.value })}
          />
        </label>
        <label>
          配置文件路径
          <div className="input-row">
            <Input
              required
              value={value.path}
              onChange={(e) => setValue({ ...value, path: e.target.value })}
              placeholder="/完整路径/mcp.json"
            />
            <Button variant="outline" type="button" onClick={choose} aria-label="选择配置文件">
              <FolderOpen size={17} />
            </Button>
          </div>
        </label>
        <div className="callout">
          <strong>
            {adapter.format} · {adapter.transports.join(" / ")}
          </strong>
          <p>{adapter.note}</p>
          <p>
            可指定其他 Profile
            或项目配置文件。这里只管理选定文件，不计算项目继承、组织策略或运行时启用状态。
          </p>
        </div>
      </form>
    </Modal>
  );
}

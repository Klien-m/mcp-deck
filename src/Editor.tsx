import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Modal, ErrorBox, ToolIcon } from "./components";
import { Select } from "./Select";
import { request } from "./api";
import type { Service, Config, Target, Snapshot } from "./types";

const empty: Config = {
  transport: "stdio",
  command: "",
  args: [],
  cwd: "",
  env: {},
  url: "",
  headers: {},
};
export function Editor({
  service,
  onClose,
  onSaved,
}: {
  service: Service | null;
  onClose: () => void;
  onSaved: (id: string) => Promise<void>;
}) {
  const [name, setName] = useState(service?.name || "");
  const [key, setKey] = useState(service?.key || "");
  const [description, setDescription] = useState(service?.description || "");
  const [config, setConfig] = useState<Config>(service?.config || empty);
  const [args, setArgs] = useState(JSON.stringify(config.args, null, 2));
  const [env, setEnv] = useState(JSON.stringify(config.env, null, 2));
  const [headers, setHeaders] = useState(
    JSON.stringify(config.headers, null, 2),
  );
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const change = (patch: Partial<Config>) =>
    setConfig((c) => ({ ...c, ...patch }));
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setBusy(true);
    try {
      const values = {
        ...config,
        args: config.transport === "stdio" ? JSON.parse(args) : [],
        env: config.transport === "stdio" ? JSON.parse(env) : {},
        headers: config.transport === "stdio" ? {} : JSON.parse(headers),
        command: config.transport === "stdio" ? config.command : "",
        cwd: config.transport === "stdio" ? config.cwd : "",
        url: config.transport === "stdio" ? "" : config.url,
      };
      const id = await request<string>("saveService", {
        input: {
          id: service?.id || null,
          key: key.trim(),
          name: name.trim(),
          description,
          config: values,
        },
      });
      await onSaved(id);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal
      title={service ? `编辑 ${service.name}` : "添加 MCP 服务"}
      onClose={() => !busy && onClose()}
      wide
      footer={
        <>
          <span className="muted grow">
            保存到服务库后，预览并应用到目标工具
          </span>
          <button onClick={onClose} disabled={busy}>
            取消
          </button>
          <button className="primary" form="editor" disabled={busy}>
            {busy ? "正在保存…" : "保存服务"}
          </button>
        </>
      }
    >
      <form id="editor" onSubmit={submit}>
        <ErrorBox text={error} />
        <div className="form-grid">
          <label>
            显示名称
            <input
              autoFocus
              value={name}
              required
              maxLength={80}
              onChange={(e) => {
                setName(e.target.value);
                if (!service && (!key || key === name)) setKey(e.target.value);
              }}
              placeholder="例如 Filesystem"
            />
          </label>
          <label>
            配置键
            <input
              value={key}
              required
              disabled={!!service}
              maxLength={80}
              onChange={(e) => setKey(e.target.value)}
              placeholder="filesystem"
            />
            <small>工具配置中的唯一名称，创建后保持稳定。</small>
          </label>
          <label className="full">
            功能描述
            <input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="这个服务能帮助你完成什么？"
            />
          </label>
          <label className="full">
            传输方式
            <Select
              label="传输方式"
              value={config.transport}
              onChange={(value) => change({ transport: value as Config["transport"] })}
              options={[
                { value: "stdio", label: "stdio", detail: "本地进程" },
                { value: "http", label: "Streamable HTTP", detail: "远程服务" },
                { value: "sse", label: "SSE", detail: "兼容旧服务" },
              ]}
            />
          </label>
          {config.transport === "stdio" ? (
            <>
              <label className="full">
                启动命令
                <input
                  required
                  value={config.command}
                  onChange={(e) => change({ command: e.target.value })}
                  placeholder="npx / uvx / 可执行文件的完整路径"
                  autoComplete="off"
                />
              </label>
              <label className="full">
                启动参数
                <textarea
                  rows={4}
                  value={args}
                  onChange={(e) => setArgs(e.target.value)}
                  spellCheck={false}
                />
                <small>JSON 字符串数组，每个元素是一个独立参数。</small>
              </label>
              <label className="full">
                工作目录 <span className="optional">可选</span>
                <input
                  value={config.cwd}
                  onChange={(e) => change({ cwd: e.target.value })}
                  placeholder="只用于声明支持 cwd 的工具"
                />
              </label>
              <label className="full">
                环境变量
                <textarea
                  rows={3}
                  value={env}
                  onChange={(e) => setEnv(e.target.value)}
                  spellCheck={false}
                />
                <small>JSON 字符串键值对象。变量引用由目标工具解释。</small>
              </label>
            </>
          ) : (
            <>
              <label className="full">
                服务地址
                <input
                  required
                  value={config.url}
                  onChange={(e) => change({ url: e.target.value })}
                  placeholder="https://example.com/mcp"
                  autoComplete="off"
                />
              </label>
              <label className="full">
                请求头
                <textarea
                  rows={5}
                  value={headers}
                  onChange={(e) => setHeaders(e.target.value)}
                  spellCheck={false}
                />
                <small>JSON 字符串键值对象。OAuth 登录在目标工具中完成。</small>
              </label>
            </>
          )}
          <p className="form-note full">
            配置在本机保存。内部版凭据与备份使用本地文件权限保护，尚未接入系统钥匙串；建议沿用目标工具的环境变量与认证机制。
          </p>
        </div>
      </form>
    </Modal>
  );
}

export function TargetEditor({
  snapshot,
  target,
  onClose,
  onSaved,
}: {
  snapshot: Snapshot;
  target: Target | null;
  onClose: () => void;
  onSaved: () => Promise<void>;
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
  const [busy, setBusy] = useState(false);
  const adapter = snapshot.adapters.find((a) => a.id === value.adapterId)!;
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
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      await request("saveTarget", { target: value });
      await onSaved();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal
      title={target ? "配置目标工具" : "添加配置目标"}
      onClose={() => !busy && onClose()}
      footer={
        <>
          <button onClick={onClose}>取消</button>
          <button form="target-editor" className="primary" disabled={busy}>
            保存目标
          </button>
        </>
      }
    >
      <form id="target-editor" onSubmit={submit}>
        <ErrorBox text={error} />
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
                  snapshot.adapters.find((a) => a.id === adapterId)!.name +
                  " · 自定义位置",
              })
            }
            options={snapshot.adapters.map((a) => ({
              value: a.id,
              label: a.name,
              icon: <ToolIcon id={a.id} small />,
            }))}
          />
        </label>
        <label>
          目标名称
          <input
            required
            value={value.name}
            onChange={(e) => setValue({ ...value, name: e.target.value })}
          />
        </label>
        <label>
          配置文件路径
          <div className="input-row">
            <input
              required
              value={value.path}
              onChange={(e) => setValue({ ...value, path: e.target.value })}
              placeholder="/完整路径/mcp.json"
            />
            <button type="button" onClick={choose} aria-label="选择配置文件">
              <FolderOpen size={17} />
            </button>
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

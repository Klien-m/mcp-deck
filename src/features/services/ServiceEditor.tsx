import { useState } from "react";
import { Modal, ErrorBox } from "../../components";
import { Select } from "../../Select";
import type { Service, ServiceInput, Config } from "../../types";

const empty: Config = {
  transport: "stdio",
  command: "",
  args: [],
  cwd: "",
  env: {},
  url: "",
  headers: {},
};
export function ServiceEditor({
  service,
  onClose,
  onSave,
  busy,
  error: workspaceError,
}: {
  service: Service | null;
  onClose: () => void;
  onSave: (input: ServiceInput) => Promise<boolean>;
  busy: boolean;
  error: string;
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
  const change = (patch: Partial<Config>) =>
    setConfig((c) => ({ ...c, ...patch }));
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    if (busy) return;
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
      const saved = await onSave({
        id: service?.id || null,
        key: key.trim(),
        name: name.trim(),
        description,
        config: values,
      });
      if (saved) onClose();
    } catch (e) {
      setError(String(e));
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
        <ErrorBox text={error || workspaceError} />
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
              onChange={(value) =>
                change({ transport: value as Config["transport"] })
              }
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

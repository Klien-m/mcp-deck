import { useEffect, useRef, useState } from "react";
import { Plug, Trash2, ShieldCheck } from "lucide-react";
import { request } from "../../api";
import { Code, ErrorBox, ToolIcon } from "../../components";
import type {
  Adapter,
  Change,
  Checks,
  Service,
  TargetStatus,
} from "../../types";

import { actionNames } from "../sync/labels";

/**
 * 展示一个服务的概览、公共配置与静态检查；changes 必须已由上层过滤到本服务。
 * 分配和移除通过业务回调执行，局部状态只属于当前详情实例。
 */
export function ServiceDetail({
  service,
  adapters,
  targets,
  changes,
  busy,
  status,
  onAssign,
  onRemove,
  onEdit,
  onExport,
  onRemoved,
}: {
  service: Service;
  adapters: Adapter[];
  targets: TargetStatus[];
  changes: Change[];
  busy: boolean;
  status: string;
  onAssign: (targetId: string, enabled: boolean) => Promise<boolean>;
  onRemove: () => Promise<boolean>;
  onEdit: (service: Service) => void;
  onExport: (ids: string[]) => void;
  onRemoved: (id: string) => void;
}) {
  const [tab, setTab] = useState("overview");
  const [checkState, setCheckState] = useState<{
    serviceId: string;
    result: Checks;
  } | null>(null);
  const [checkError, setCheckError] = useState("");
  const requestId = useRef(0);
  // 同时绑定服务 ID 与请求代次，避免切换服务后短暂显示上一项的检查结果。
  const checks =
    checkState?.serviceId === service.id ? checkState.result : null;
  const adapter = (id: string) => adapters.find((a) => a.id === id)!;

  useEffect(() => {
    setTab("overview");
    setCheckState(null);
    setCheckError("");
    return () => {
      requestId.current++;
    };
  }, [service.id]);

  /** 重新检查时清除旧结果，只接受最后一次请求；检查本身不启动或连接服务。 */
  async function checkService() {
    const id = ++requestId.current;
    const serviceId = service.id;
    setTab("checks");
    setCheckState(null);
    setCheckError("");
    try {
      const result = await request("checks", { serviceId });
      if (id === requestId.current) setCheckState({ serviceId, result });
    } catch (e) {
      if (id === requestId.current) setCheckError(String(e));
    }
  }

  return (
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
          <button onClick={() => onExport([service.id])}>导出</button>
          <button
            className="icon-button danger"
            aria-label={`移除 ${service.name}`}
            disabled={busy}
            onClick={async () => {
              if (await onRemove()) onRemoved(service.id);
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
          onClick={() => checkService()}
        >
          配置检查
        </button>
      </div>
      {tab === "overview" ? (
        <>
          <div className="section-heading">
            <h3>服务配置</h3>
            <button className="text-button" onClick={() => onEdit(service)}>
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
                {service.config.transport === "stdio" ? "启动命令" : "服务地址"}
              </span>
              <code>{service.config.command || service.config.url}</code>
            </div>
            <div>
              <span>分配状态</span>
              <strong>{status}</strong>
            </div>
          </div>
          <div className="section-heading">
            <h3>分配到工具</h3>
            <span className="muted">
              {service.targets.length} / {targets.length} 个目标
            </span>
          </div>
          <div className="assignment">
            {targets.map((t) => {
              const a = adapter(t.adapterId);
              const checked = service.targets.includes(t.id);
              // 界面兼容性只决定能否新增分配；已有不兼容分配仍允许取消，后端会再次校验。
              const compatible =
                a.transports.includes(service.config.transport) &&
                (a.supportsCwd || !service.config.cwd);
              const change = changes.find((c) => c.targetId === t.id);
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
                    onClick={() => onAssign(t.id, !checked)}
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
            <button className="text-button" onClick={() => onEdit(service)}>
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
                Object.keys(service.config.env).map((k) => [k, "<已隐藏>"]),
              ),
              headers: Object.fromEntries(
                Object.keys(service.config.headers).map((k) => [k, "<已隐藏>"]),
              ),
            }}
          />
        </>
      ) : (
        <>
          <div className="section-heading">
            <h3>配置检查</h3>
            <button className="text-button" onClick={() => checkService()}>
              重新检查
            </button>
          </div>
          {checkError ? (
            <ErrorBox text={checkError} />
          ) : checks ? (
            <>
              <div className={`callout ${checks.issues.length ? "warn" : ""}`}>
                <strong>
                  {checks.issues.length ? "有需要确认的配置" : "字段检查通过"}
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
  );
}

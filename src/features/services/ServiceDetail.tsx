import { Switch } from "@/components/ui/switch";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useEffect, useRef, useState } from "react";
import { Plug, Trash2, ShieldCheck, Undo2 } from "lucide-react";
import { request } from "../../api";
import { Code, ErrorBox, ToolIcon } from "../../components";
import type {
  Adapter,
  Change,
  Checks,
  Service,
  TargetStatus,
} from "../../types";

import { describeChange } from "../sync/labels";
import { DiagnosticReport } from "./DiagnosticReport";

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
  stale = false,
  status,
  onAssign,
  onRemove,
  onUndoRemove,
  onEdit,
  onExport,
  onRemoved,
}: {
  service: Service;
  adapters: Adapter[];
  targets: TargetStatus[];
  changes: Change[];
  busy: boolean;
  stale?: boolean;
  status: string;
  onAssign: (targetId: string, enabled: boolean) => Promise<boolean>;
  onRemove: () => Promise<boolean>;
  onUndoRemove: () => Promise<boolean>;
  onEdit: (service: Service) => void;
  onExport: (ids: string[]) => void;
  onRemoved: (id: string) => void;
}) {
  const [tab, setTab] = useState("overview");
  const [checkState, setCheckState] = useState<{
    serviceId: string;
    configVersion: string;
    result: Checks;
  } | null>(null);
  const [checkError, setCheckError] = useState("");
  const requestId = useRef(0);
  // 只在内存比较配置内容；同 ID 的磁盘版本替换也会使旧检查失效，不输出配置快照。
  const configVersion = JSON.stringify(service.config);
  // 同时绑定服务、配置与请求代次，避免下一次 effect 执行前闪现过期检查结果。
  const checks =
    checkState?.serviceId === service.id && checkState.configVersion === configVersion
      ? checkState.result
      : null;
  const adapter = (id: string) => adapters.find((a) => a.id === id)!;

  useEffect(() => {
    setTab("overview");
    setCheckState(null);
    setCheckError("");
    return () => {
      requestId.current++;
    };
  }, [service.id, configVersion]);

  /** 重新检查时清除旧结果，只接受最后一次请求；检查本身不启动或连接服务。 */
  async function checkService() {
    const id = ++requestId.current;
    const serviceId = service.id;
    setTab("checks");
    setCheckState(null);
    setCheckError("");
    try {
      const result = await request("checks", { serviceId });
      if (id === requestId.current) setCheckState({ serviceId, configVersion, result });
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
          <Button variant="outline" disabled={service.deleted} onClick={() => onExport([service.id])}>导出</Button>
          <Button variant="ghost" size="icon-sm"
            className="icon-button danger"
            aria-label={`移除 ${service.name}`}
            disabled={busy || stale || service.deleted}
            onClick={async () => {
              if (await onRemove()) onRemoved(service.id);
            }}
          >
            <Trash2 size={17} />
          </Button>
        </div>
      </div>
      {service.deleted && (
        <div className="callout warn removal-notice" role="status">
          <div>
            <strong>{stale ? "移除状态待刷新" : "待移除"}</strong>
            <p>{stale ? "请重新读取配置后确认移除结果，无需重复操作。" : "确认同步后才会从目标配置文件移除此服务。应用前可以撤销。"}</p>
          </div>
          <Button variant="outline" disabled={busy || stale} onClick={() => void onUndoRemove()}>
            <Undo2 size={15} />撤销移除
          </Button>
        </div>
      )}
      <Tabs value={tab} onValueChange={(next) => {
        if (next === "checks") void checkService();
        else setTab(next);
      }}>
        <TabsList className="detail-tabs" aria-label="服务信息">
          <TabsTrigger value="overview">概览</TabsTrigger>
          <TabsTrigger value="json">配置 JSON</TabsTrigger>
          <TabsTrigger value="checks">配置检查</TabsTrigger>
        </TabsList>
        <TabsContent value="overview">
          <div className="section-heading">
            <h3>服务配置</h3>
            <Button variant="ghost" className="text-button" disabled={busy || stale || service.deleted} onClick={() => onEdit(service)}>
              编辑配置
            </Button>
          </div>
          <div className="settings">
            <div>
              <span>配置键</span>
              <strong>{service.key}</strong>
            </div>
            <div>
              <span>传输方式</span>
              <strong>
                <Badge variant="secondary" className="tag">
                  {service.config.transport === "http"
                    ? "Streamable HTTP"
                    : service.config.transport}
                </Badge>
              </strong>
            </div>
            <div>
              <span>
                {service.config.transport === "stdio" ? "启动命令" : "服务地址"}
              </span>
              <code>{service.config.command || service.config.url}</code>
            </div>
            <div>
              <span>配置文件状态</span>
              <strong>{stale ? "状态待刷新" : status}</strong>
            </div>
            <div>
              <span>连接与加载</span>
              <strong>未检测 · 需在目标工具确认</strong>
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
              const checked = !service.deleted && service.targets.includes(t.id);
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
                    <small className={change || t.error ? "warning" : ""}>
                      {stale
                        ? "状态待刷新 · 请重新读取配置"
                        : t.error
                          ? "配置读取失败 · 无法确认同步状态"
                          : change?.conflict
                            ? "外部配置变更 · 需要处理"
                            : change
                              ? `${describeChange(change)} · 待应用`
                              : !compatible
                                ? "不支持此配置中的传输方式或 cwd"
                                : checked
                                  ? "配置已对齐"
                                  : "尚未分配"}
                    </small>
                    {!stale && change?.conflict && <small className="warning">{describeChange(change)}</small>}
                    {!stale && t.error && <small className="warning">{t.error}</small>}
                    {!stale && !t.exists && !t.error && checked && <small>应用时创建配置文件</small>}
                    <code className="target-path">{t.path}</code>
                  </div>
                  <Switch
                    checked={checked}
                    aria-label={`${service.name} 分配到 ${t.name}`}
                    disabled={busy || stale || service.deleted || (!compatible && !checked)}
                    onCheckedChange={(enabled) => void onAssign(t.id, enabled)}
                  />
                </div>
              );
            })}
          </div>
          <p className="hint">
            <ShieldCheck size={15} />
            分配控制配置文件中的服务条目。客户端可能仍需刷新、授权或受项目配置覆盖。
          </p>
        </TabsContent>
        <TabsContent value="json">
          <div className="section-heading">
            <h3>服务公共配置</h3>
            <Button variant="ghost" className="text-button" disabled={busy || stale || service.deleted} onClick={() => onEdit(service)}>
              编辑完整配置
            </Button>
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
        </TabsContent>
        <TabsContent value="checks">
          <div className="section-heading">
            <h3>配置检查</h3>
            <Button variant="ghost" className="text-button" onClick={() => checkService()}>
              重新检查
            </Button>
          </div>
          {checkError ? (
            <ErrorBox text={checkError} />
          ) : checks ? (
            <DiagnosticReport result={checks} />
          ) : (
            <p className="muted">正在检查…</p>
          )}
        </TabsContent>
      </Tabs>
    </>
  );
}

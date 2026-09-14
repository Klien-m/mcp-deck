import { useEffect, useRef, useState } from "react";
import { AlertTriangle, FolderSearch, Plug } from "lucide-react";
import { request } from "../../api";
import { ErrorBox, Modal, ToolIcon } from "../../components";
import { Select } from "../../Select";
import type { Discovery, TargetStatus } from "../../types";

export function DiscoveryDialog({
  targets,
  initialTarget,
  error,
  busy: workspaceBusy,
  onAdopt,
  onClose,
  onAdopted,
  onClearError,
}: {
  targets: TargetStatus[];
  initialTarget: string;
  error: string;
  busy: boolean;
  onAdopt: (targetId: string, keys: string[]) => Promise<boolean>;
  onClose: () => void;
  onAdopted: () => void;
  onClearError: () => void;
}) {
  const [discoveryTarget, setDiscoveryTarget] = useState(initialTarget);
  const [discoveryResult, setDiscoveryResult] = useState<{
    targetId: string;
    items: Discovery[];
  } | null>(null);
  const [chosen, setChosen] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [localError, setLocalError] = useState("");
  const requestId = useRef(0);
  const discovered =
    discoveryResult?.targetId === discoveryTarget ? discoveryResult.items : [];
  const busy = workspaceBusy || loading;
  const importableKeys = discovered
    .filter((d) => d.config && !d.error && !d.managed)
    .map((d) => d.key);
  const chosenKeys = importableKeys.filter((key) => chosen.includes(key));
  const allChosen =
    importableKeys.length > 0 && chosenKeys.length === importableKeys.length;
  const close = () => {
    if (!busy) onClose();
  };

  useEffect(() => {
    const id = ++requestId.current;
    setLoading(true);
    setLocalError("");
    setDiscoveryResult(null);
    setChosen([]);
    request("discover", { targetId: discoveryTarget })
      .then((items) => {
        if (id === requestId.current)
          setDiscoveryResult({ targetId: discoveryTarget, items });
      })
      .catch((e) => {
        if (id === requestId.current) setLocalError(String(e));
      })
      .finally(() => {
        if (id === requestId.current) setLoading(false);
      });
    return () => {
      requestId.current++;
    };
  }, [discoveryTarget]);

  return (
    <Modal
      title="发现本机 MCP 配置"
      onClose={close}
      wide
      footer={
        <>
          <span className="muted grow">纳入管理不会修改目标文件</span>
          <button onClick={close}>取消</button>
          <button
            className="primary"
            disabled={busy || !chosenKeys.length}
            onClick={async () => {
              if (await onAdopt(discoveryTarget, chosenKeys)) {
                onAdopted();
              }
            }}
          >
            纳入管理 {chosenKeys.length ? `(${chosenKeys.length})` : ""}
          </button>
        </>
      }
    >
      <label>
        目标工具
        <Select
          label="目标工具"
          value={discoveryTarget}
          onChange={(targetId) => {
            onClearError();
            setDiscoveryTarget(targetId);
          }}
          disabled={busy}
          options={targets.map((t) => ({
            value: t.id,
            label: t.name,
            detail: t.exists ? `${t.count} 个配置` : "未发现配置",
            icon: <ToolIcon id={t.adapterId} small />,
          }))}
        />
      </label>
      <p className="path-label">
        {targets.find((t) => t.id === discoveryTarget)?.path}
      </p>
      <ErrorBox text={localError || error} />
      {busy ? (
        <p className="muted">正在读取…</p>
      ) : discovered.length ? (
        <div className="discover-list">
          <div className="discover-controls">
            <label
              className={`discover-select-all ${!importableKeys.length ? "is-disabled" : ""}`}
            >
              <input
                type="checkbox"
                checked={allChosen}
                ref={(input) => {
                  if (input)
                    input.indeterminate = chosenKeys.length > 0 && !allChosen;
                }}
                disabled={!importableKeys.length}
                onChange={(e) =>
                  setChosen(e.target.checked ? importableKeys : [])
                }
              />
              全选可导入项
            </label>
            <span
              className="discover-selection"
              role="status"
              aria-live="polite"
            >
              {importableKeys.length
                ? `已选 ${chosenKeys.length} / ${importableKeys.length} 项`
                : "没有可导入项"}
            </span>
          </div>
          {discovered.map((d) => {
            const unsupported = !!d.error || !d.config;
            const unavailable = unsupported || d.managed;
            const checked = chosenKeys.includes(d.key);
            return (
              <label
                className={`discover-item ${unsupported ? "is-unsupported" : d.managed ? "is-managed" : checked ? "is-selected" : ""}`}
                key={d.key}
              >
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={unavailable}
                  onChange={(e) => {
                    const checked = e.target.checked;
                    setChosen((c) =>
                      checked ? [...c, d.key] : c.filter((k) => k !== d.key),
                    );
                  }}
                />
                {unsupported ? <AlertTriangle size={20} /> : <Plug size={20} />}
                <div>
                  <strong>{d.key}</strong>
                  <small>
                    {unsupported
                      ? d.error || "无法识别此配置，原配置保持不变"
                      : d.managed
                        ? "已纳入服务库，无需重复导入"
                        : `${d.config?.transport} · 可导入`}
                  </small>
                </div>
                <span className="tag">
                  {unsupported
                    ? "不支持"
                    : d.managed
                      ? "已管理"
                      : checked
                        ? "已选择"
                        : "可导入"}
                </span>
              </label>
            );
          })}
        </div>
      ) : (
        <div className="empty-card">
          <FolderSearch size={28} />
          <h3>这里还没有 MCP 配置</h3>
          <p>选择其他工具，或通过「工具与路径」指定文件。</p>
        </div>
      )}
      <p className="hint">
        同名服务会保留独立来源；未勾选及不支持的条目会保留在原文件中。
      </p>
    </Modal>
  );
}

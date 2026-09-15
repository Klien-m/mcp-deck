import { Checkbox } from "@/components/ui/checkbox";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useEffect, useState } from "react";
import { AlertTriangle, FolderSearch, Plug, RefreshCw } from "lucide-react";
import { request } from "../../api";
import { ErrorBox, Modal, ToolIcon } from "../../components";
import type { Adoption, Discovery, TargetDiscovery } from "../../types";

const canAdopt = (item: Discovery) =>
  !!item.config && !item.error && !item.managed;

/** 首次启动聚合发现；选择只保存在弹窗，明确提交后才纳入服务库。 */
export function OnboardingDialog({
  error,
  busy: workspaceBusy,
  onComplete,
  onCompleted,
  onClearError,
}: {
  error: string;
  busy: boolean;
  onComplete: (selections: Adoption[]) => Promise<boolean>;
  onCompleted: () => void;
  onClearError: () => void;
}) {
  const [results, setResults] = useState<TargetDiscovery[]>([]);
  const [chosen, setChosen] = useState<Record<string, string[]>>({});
  const [loading, setLoading] = useState(true);
  const [localError, setLocalError] = useState("");
  const [scan, setScan] = useState(0);
  const busy = loading || workspaceBusy;

  useEffect(() => {
    let active = true;
    setLoading(true);
    setLocalError("");
    setResults([]);
    setChosen({});
    request("discoverAll", {})
      .then((next) => active && setResults(next))
      .catch((e) => active && setLocalError(String(e)))
      .finally(() => active && setLoading(false));
    return () => {
      active = false;
    };
  }, [scan]);

  const selections = results
    .map(({ target, items }) => ({
      targetId: target.id,
      keys: items
        .filter(
          (item) => canAdopt(item) && chosen[target.id]?.includes(item.key),
        )
        .map((item) => item.key),
    }))
    .filter(({ keys }) => keys.length > 0);
  const selectedCount = selections.reduce(
    (sum, entry) => sum + entry.keys.length,
    0,
  );
  const toolCount = results.filter(({ items }) => items.length > 0).length;
  const serviceCount = results.reduce(
    (sum, entry) => sum + entry.items.length,
    0,
  );

  async function complete(selection: Adoption[]) {
    if (!workspaceBusy && (await onComplete(selection))) onCompleted();
  }

  return (
    <Modal
      title="将已有 MCP 纳入管理"
      onClose={() => void complete([])}
      wide
      footer={
        <>
          <span className="muted grow" role="status" aria-live="polite">
            已选 {selections.length} 个工具 · {selectedCount} 个 MCP
          </span>
          <Button variant="outline" disabled={workspaceBusy} onClick={() => void complete([])}>
            {!loading && !results.length && !localError
              ? "进入工作区"
              : "暂时跳过"}
          </Button>
          <Button variant="default"
            className="primary"
            disabled={busy || !selectedCount}
            onClick={() => void complete(selections)}
          >
            纳入管理{selectedCount ? ` (${selectedCount})` : ""}
          </Button>
        </>
      }
    >
      <div className="onboarding-intro">
        <FolderSearch size={28} />
        <div>
          <h3>欢迎使用 MCP Deck</h3>
          <p>自动查找本机 Agent 工具中的 MCP 配置，选择后即可集中管理。</p>
        </div>
      </div>
      <p className="hint">
        扫描已支持工具的配置路径；纳入管理只保存到服务库，原配置保持不变。
      </p>
      <ErrorBox text={localError || error} />
      <div className="onboarding-summary">
        <span role="status" aria-live="polite">
          {loading
            ? "正在查找本机 MCP 配置…"
            : localError
              ? "扫描未完成，请重试"
              : `发现 ${toolCount} 个工具，共 ${serviceCount} 个 MCP`}
        </span>
        <Button variant="outline"
          disabled={busy}
          onClick={() => {
            onClearError();
            setLoading(true);
            setScan((value) => value + 1);
          }}
        >
          <RefreshCw size={13} />
          重新扫描
        </Button>
      </div>
      {!loading && !localError && !results.length && (
        <div className="empty-card">
          <FolderSearch size={28} />
          <h3>暂未发现已有 MCP 配置</h3>
          <p>进入工作区后，可在「工具与路径」指定文件，或手动添加服务。</p>
        </div>
      )}
      {!loading &&
        results.map(({ target, items, error: scanError }) => {
          const keys = items.filter(canAdopt).map((item) => item.key);
          const selected = keys.filter((key) =>
            chosen[target.id]?.includes(key),
          );
          const allSelected =
            keys.length > 0 && selected.length === keys.length;
          return (
            <section
              className="onboarding-tool discover-list"
              key={target.id}
              aria-label={target.name}
            >
              <div className="discover-controls">
                <label
                  className={`discover-select-all ${!keys.length ? "is-disabled" : ""}`}
                >
                  <Checkbox
                    aria-label={`选择 ${target.name} 的全部可导入 MCP`}
                    checked={allSelected ? true : selected.length ? "indeterminate" : false}
                    disabled={busy || !keys.length}
                    onCheckedChange={(checked) => {
                      const next = checked === true ? keys : [];
                      setChosen((current) => ({
                        ...current,
                        [target.id]: next,
                      }));
                    }}
                  />
                  <ToolIcon id={target.adapterId} small />
                  {target.name}
                </label>
                <span className="discover-selection">
                  {items.length} 个 MCP · {keys.length} 个可导入
                </span>
              </div>
              <p className="onboarding-path">{target.path}</p>
              {scanError && (
                <div className="onboarding-read-error" role="alert">
                  <AlertTriangle size={16} />
                  <span>无法读取配置：{scanError}</span>
                </div>
              )}
              {items.map((item) => {
                const unsupported = !!item.error || !item.config;
                const checked = selected.includes(item.key);
                return (
                  <label
                    className={`discover-item ${unsupported ? "is-unsupported" : item.managed ? "is-managed" : checked ? "is-selected" : ""}`}
                    key={item.key}
                  >
                    <Checkbox
                      checked={checked}
                      disabled={busy || !canAdopt(item)}
                      onCheckedChange={(checked) => {
                        const next = checked === true;
                        setChosen((current) => ({
                          ...current,
                          [target.id]: next
                            ? [...(current[target.id] || []), item.key]
                            : (current[target.id] || []).filter(
                                (key) => key !== item.key,
                              ),
                        }));
                      }}
                    />
                    {unsupported ? (
                      <AlertTriangle size={18} />
                    ) : (
                      <Plug size={18} />
                    )}
                    <div>
                      <strong>{item.key}</strong>
                      <small>
                        {unsupported
                          ? item.error || "无法识别此配置"
                          : item.managed
                            ? "已纳入服务库，无需重复导入"
                            : `${item.config?.transport} · 可导入`}
                      </small>
                    </div>
                    <Badge variant="secondary" className="tag">
                      {unsupported
                        ? "不支持"
                        : item.managed
                          ? "已管理"
                          : checked
                            ? "已选择"
                            : "可导入"}
                    </Badge>
                  </label>
                );
              })}
            </section>
          );
        })}
      <p className="hint">
        可按工具全选或逐项选择；同名 MCP
        保留各自来源。跳过后可随时通过「发现本机配置」继续。
      </p>
    </Modal>
  );
}

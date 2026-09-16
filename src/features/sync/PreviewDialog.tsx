import { Checkbox } from "@/components/ui/checkbox";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useState } from "react";
import { AlertTriangle, Check } from "lucide-react";
import { Code, ErrorBox, Modal } from "../../components";
import type { Preview } from "../../types";
import { actionNames, describeChange } from "./labels";

/**
 * 展示同一计划的两份差异并提交应用或冲突选择；明文开关不触发 IPC。
 * 错误、零变更或未解决冲突会阻止应用，后台仍会校验修订和磁盘原文。
 */
export function PreviewDialog({
  preview,
  error,
  busy,
  stale = false,
  onApply,
  onResolve,
  onClose: close,
  onApplied,
}: {
  error: string;
  busy: boolean;
  stale?: boolean;
  onClose: () => void;
  onApply: (id: string) => Promise<boolean>;
  onResolve: (
    serviceId: string,
    targetId: string,
    useDisk: boolean,
  ) => Promise<boolean>;
  preview: Preview;
  onApplied: () => void;
}) {
  // 将明文选择绑定当前计划 ID：计划变化自动回到脱敏，无需重挂载弹窗或重置其他状态。
  const [revealedPlanId, setRevealedPlanId] = useState<string | null>(null);
  const revealPreview = revealedPlanId === preview.id;
  return (
    <Modal
      title="同步变更预览"
      onClose={close}
      wide
      footer={
        <>
          <span className="muted grow">先备份，再逐文件写入并校验</span>
          <Button variant="outline" onClick={close}>返回</Button>
          <Button variant="default"
            className="primary"
            disabled={
              busy ||
              stale ||
              !!preview.errors.length ||
              !preview.changes.length ||
              preview.changes.some((c) => c.conflict)
            }
            onClick={async () => {
              if (await onApply(preview.id)) onApplied();
            }}
          >
            应用 {preview.changes.length} 项变更
          </Button>
        </>
      }
    >
      <ErrorBox text={error} />
      <ErrorBox text={stale ? "此预览已过期，请重新读取配置" : ""} />
      {preview.errors.map((e, i) => (
        <ErrorBox key={i} text={e} />
      ))}
      <label className="checkbox-line">
        <Checkbox
          aria-label="显示完整差异"
          checked={revealPreview}
          disabled={busy}
          onCheckedChange={(checked) =>
            setRevealedPlanId(checked === true ? preview.id : null)
          }
        />
        显示完整差异（包含命令参数、环境变量与凭据）
      </label>
      <div className="preview-summary">
        <strong>{preview.changes.length}</strong>
        <span>项变更，涉及 {preview.fileCount} 个配置文件</span>
      </div>
      {!stale && !preview.changes.length && !preview.errors.length && (
        <div className="empty-card">
          <Check size={30} />
          <h3>配置已对齐</h3>
          <p>没有需要写入的服务变更。</p>
        </div>
      )}
      {(revealPreview
        ? (preview.fullChanges ?? preview.changes)
        : preview.changes
      ).map((c, i) => (
        <div
          className={`change ${c.conflict ? "conflict" : ""}`}
          key={`${c.targetId}-${c.serviceId}-${i}`}
        >
          <div className="change-header">
            <strong>{describeChange(c)}</strong>
            <Badge variant="secondary" className="tag">{actionNames[c.action]}</Badge>
          </div>
          <div className="diff-grid">
            <div>
              <small>当前磁盘配置</small>
              <Code value={c.before || "无此配置"} />
            </div>
            <div>
              <small>服务库期望配置</small>
              <Code value={c.after || "移除此服务条目"} />
            </div>
          </div>
          {c.conflict && (
            <div className="conflict-actions">
              <p>
                <AlertTriangle size={15} />
                {c.message}
              </p>
              <Button variant="outline"
                disabled={busy || stale}
                onClick={() => onResolve(c.serviceId, c.targetId, true)}
              >
                采用磁盘版本
              </Button>
              <Button variant="outline"
                disabled={busy || stale}
                onClick={() => onResolve(c.serviceId, c.targetId, false)}
              >
                保留服务库版本
              </Button>
            </div>
          )}
        </div>
      ))}
      <p className="hint">
        敏感字段与命令参数默认隐藏，请在服务编辑器确认具体内容。未管理服务与其他配置项保留；客户端加载与认证需分别确认。
      </p>
    </Modal>
  );
}

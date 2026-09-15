import { useState } from "react";
import { AlertTriangle, Check } from "lucide-react";
import { Code, ErrorBox, Modal } from "../../components";
import type { Preview } from "../../types";
import { actionNames } from "./labels";

/**
 * 展示同一计划的两份差异并提交应用或冲突选择；明文开关不触发 IPC。
 * 错误、零变更或未解决冲突会阻止应用，后台仍会校验修订和磁盘原文。
 */
export function PreviewDialog({
  preview,
  error,
  busy,
  onApply,
  onResolve,
  onClose: close,
  onApplied,
}: {
  error: string;
  busy: boolean;
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
          <button onClick={close}>返回</button>
          <button
            className="primary"
            disabled={
              busy ||
              !!preview.errors.length ||
              !preview.changes.length ||
              preview.changes.some((c) => c.conflict)
            }
            onClick={async () => {
              if (await onApply(preview.id)) onApplied();
            }}
          >
            应用 {preview.changes.length} 项变更
          </button>
        </>
      }
    >
      <ErrorBox text={error} />
      {preview.errors.map((e, i) => (
        <ErrorBox key={i} text={e} />
      ))}
      <label className="checkbox-line">
        <input
          type="checkbox"
          checked={revealPreview}
          disabled={busy}
          onChange={(e) =>
            setRevealedPlanId(e.target.checked ? preview.id : null)
          }
        />
        显示完整差异（包含命令参数、环境变量与凭据）
      </label>
      <div className="preview-summary">
        <strong>{preview.changes.length}</strong>
        <span>项变更，涉及 {preview.fileCount} 个配置文件</span>
      </div>
      {!preview.changes.length && !preview.errors.length && (
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
            <strong>{c.key}</strong>
            <span>{c.targetName}</span>
            <span className="tag">{actionNames[c.action]}</span>
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
              <button
                disabled={busy}
                onClick={() => onResolve(c.serviceId, c.targetId, true)}
              >
                采用磁盘版本
              </button>
              <button
                disabled={busy}
                onClick={() => onResolve(c.serviceId, c.targetId, false)}
              >
                保留服务库版本
              </button>
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

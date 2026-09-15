import { History as HistoryIcon } from "lucide-react";
import { ErrorBox, Modal } from "../../components";
import type { History } from "../../types";
import { historyNames } from "./labels";

/**
 * 倒序展示同步记录；仅已应用或待恢复记录提供恢复，保留当前文件只适用于待恢复。
 * 这里只表达可用操作，真实路径、状态和外部修改检查仍由事务层执行。
 */
export function HistoryDialog({
  history,
  error,
  busy,
  onRollback,
  onKeepRecovery,
  onClose: close,
}: {
  error: string;
  busy: boolean;
  onClose: () => void;
  onRollback: (id: string) => Promise<boolean>;
  onKeepRecovery: (id: string) => Promise<boolean>;
  history: History[];
}) {
  return (
    <Modal
      title="同步记录与恢复"
      onClose={close}
      wide
      footer={
        <button className="primary" onClick={close}>
          完成
        </button>
      }
    >
      <ErrorBox text={error} />
      <p className="intro">
        每次写入均保留原始文件备份。恢复只在文件仍匹配本次写入结果时执行，避免覆盖后来修改。
      </p>
      {!history.length ? (
        <div className="empty-card">
          <HistoryIcon size={30} />
          <h3>还没有同步记录</h3>
          <p>应用配置后，会在这里留下记录与备份。</p>
        </div>
      ) : (
        <div className="history-list">
          {[...history].reverse().map((h) => (
            <div className="history-item" key={h.id}>
              <div className="history-heading">
                <strong>{historyNames[h.status] || h.status}</strong>
                <time>{new Date(h.at * 1000).toLocaleString("zh-CN")}</time>
                {["applied", "recovery-needed"].includes(h.status) && (
                  <button disabled={busy} onClick={() => onRollback(h.id)}>
                    恢复原文件
                  </button>
                )}
                {h.status === "recovery-needed" && (
                  <button disabled={busy} onClick={() => onKeepRecovery(h.id)}>
                    保留当前文件
                  </button>
                )}
              </div>
              <p>{h.summary}</p>
              {h.paths.map((p) => (
                <code key={p}>{p}</code>
              ))}
            </div>
          ))}
        </div>
      )}
    </Modal>
  );
}

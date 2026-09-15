import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import { ErrorBox, Modal, ToolIcon } from "../../components";
import type { Adapter, Target, TargetStatus } from "../../types";

/** 展示已登记目标与读取概况；路径编辑和发现由上层切换到对应业务弹窗。 */
export function ToolsDialog({
  adapters,
  targets,
  dataDir,
  error,
  onClose: close,
  onEdit,
  onDiscover,
}: {
  adapters: Adapter[];
  targets: TargetStatus[];
  dataDir: string;
  error: string;
  onClose: () => void;
  onEdit: (target: Target | null) => void;
  onDiscover: (targetId: string) => void;
}) {
  const adapter = (id: string) => adapters.find((a) => a.id === id)!;
  return (
    <Modal
      title="工具与配置路径"
      onClose={close}
      wide
      footer={
        <>
          <span className="muted grow">
            {targets.length} 个配置目标 · 路径可自定义
          </span>
          <Button variant="outline"
            onClick={() => {
              onEdit(null);
            }}
          >
            <Plus size={16} />
            添加配置目标
          </Button>
          <Button variant="default" className="primary" onClick={close}>
            完成
          </Button>
        </>
      }
    >
      <ErrorBox text={error} />
      <p className="intro">
        每个目标对应一个配置文件。可以为同一工具添加其他
        Profile、扩展宿主或项目位置。
      </p>
      <div className="tools-list">
        {targets.map((t) => (
          <div className="tool-setting" key={t.id}>
            <ToolIcon id={t.adapterId} />
            <div>
              <strong>
                {t.name}
                <span className={`tag ${t.error ? "warning" : ""}`}>
                  {t.error
                    ? "需要检查"
                    : t.exists
                      ? `${t.count} 个配置`
                      : "未发现配置"}
                </span>
              </strong>
              <code>{t.path}</code>
              <small>{t.error || adapter(t.adapterId).note}</small>
            </div>
            <Button variant="outline"
              onClick={() => {
                onEdit(t);
              }}
            >
              设置
            </Button>
            <Button variant="outline"
              disabled={!t.exists || !!t.error}
              onClick={() => onDiscover(t.id)}
            >
              发现
            </Button>
          </div>
        ))}
      </div>
      <p className="hint">应用数据目录：{dataDir}</p>
    </Modal>
  );
}

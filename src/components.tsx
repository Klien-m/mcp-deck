import { Button } from "@/components/ui/button";
/** 无业务状态的公共展示组件；Modal 只负责弹窗与焦点，操作忙碌规则由调用方控制。 */
import { useRef, type ReactNode } from "react";
import { X, Layers } from "lucide-react";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { Alert, AlertDescription } from "@/components/ui/alert";
type ToolIconAsset = {
  file: string;
  crop?: { canvas: number; inset: number; size: number };
};
const claudeIcon: ToolIconAsset = {
  file: "claude.svg",
};
// 保留官方资源原文，只通过视口裁切补偿画布透明留白；资源图案本身不重绘。
const icons: Record<string, ToolIconAsset> = {
  codex: { file: "codex.svg", crop: { canvas: 716, inset: 178, size: 360 } },
  claude: claudeIcon,
  cursor: { file: "cursor.svg" },
  gemini: { file: "gemini.svg" },
  opencode: { file: "opencode.svg" },
  copilot: { file: "copilot.svg" },
  vscode: { file: "vscode.svg" },
  windsurf: { file: "windsurf.svg" },
  kiro: { file: "kiro.svg" },
  cline: { file: "cline.svg" },
  roo: { file: "roo.svg" },
  "claude-desktop": claudeIcon,
};
/** 按适配器 ID 展示本地工具图标；未知 ID 回退通用图标，small 仅改变展示尺寸。 */
export function ToolIcon({
  id,
  small = false,
}: {
  id: string;
  small?: boolean;
}) {
  const icon = icons[id];
  const src = icon && `/tool-icons/${icon.file}`;
  return (
    <span className={`tool-icon ${small ? "small" : ""}`} aria-hidden="true">
      {icon?.crop ? (
        <svg
          className="tool-icon-art"
          viewBox={`${icon.crop.inset} ${icon.crop.inset} ${icon.crop.size} ${icon.crop.size}`}
          focusable="false"
        >
          <image href={src} width={icon.crop.canvas} height={icon.crop.canvas} />
        </svg>
      ) : icon ? (
        <img className="tool-icon-art" src={src} alt="" draggable={false} />
      ) : (
        <Layers size={small ? 13 : 19} strokeWidth={1.7} />
      )}
    </span>
  );
}
/**
 * Radix 管理模态层与焦点，卸载后尝试把焦点还给仍存在的触发元素。
 * 关闭请求交给 onClose 判断是否允许，避免绕过保存期间保护。
 */
export function Modal({
  title,
  children,
  footer,
  onClose,
  wide = false,
}: {
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  const returnFocus = useRef(document.activeElement as HTMLElement | null);
  return (
    <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent
        className={`deck-dialog ${wide ? "wide" : ""}`}
        showCloseButton={false}
        aria-describedby={undefined}
        onInteractOutside={(event) => event.preventDefault()}
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          if (returnFocus.current?.isConnected) returnFocus.current.focus();
        }}
      >
        <div className="modal-head">
          <DialogTitle>{title}</DialogTitle>
          <Button variant="ghost" size="icon-sm" className="icon-button" aria-label="关闭弹窗" onClick={onClose}><X size={18} /></Button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-footer">{footer}</div>}
      </DialogContent>
    </Dialog>
  );
}
/** 空错误不占位，非空内容使用 alert 语义通知辅助技术。 */
export function ErrorBox({ text }: { text: string }) {
  return text ? (
    <Alert variant="destructive" className="error-box"><AlertDescription>{text}</AlertDescription></Alert>
  ) : null;
}
/** 以文本展示字符串或格式化 JSON；React 负责转义，不把配置内容当 HTML 执行。 */
export function Code({ value }: { value: unknown }) {
  return (
    <pre className="code">
      {typeof value === "string" ? value : JSON.stringify(value, null, 2)}
    </pre>
  );
}

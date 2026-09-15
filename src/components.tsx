/** 无业务状态的公共展示组件；Modal 只负责原生弹窗与焦点，操作忙碌规则由调用方控制。 */
import { useEffect, useRef, type ReactNode } from "react";
import { X, Layers } from "lucide-react";
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
 * 挂载时进入原生 dialog 顶层，卸载后尝试把焦点还给仍存在的触发元素。
 * Escape 先阻止浏览器直接关闭，再交给 onClose 判断是否允许，避免绕过保存期间保护。
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
  const ref = useRef<HTMLDialogElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  useEffect(() => {
    returnFocus.current = document.activeElement as HTMLElement;
    ref.current?.showModal();
    return () => {
      returnFocus.current?.isConnected && returnFocus.current.focus();
    };
  }, []);
  return (
    <dialog
      ref={ref}
      className={wide ? "wide" : ""}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
    >
      <div className="modal-head">
        <h2>{title}</h2>
        <button className="icon-button" aria-label="关闭弹窗" onClick={onClose}>
          <X size={18} />
        </button>
      </div>
      <div className="modal-body">{children}</div>
      {footer && <div className="modal-footer">{footer}</div>}
    </dialog>
  );
}
/** 空错误不占位，非空内容使用 alert 语义通知辅助技术。 */
export function ErrorBox({ text }: { text: string }) {
  return text ? (
    <div className="error-box" role="alert">
      {text}
    </div>
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

import { useEffect, useRef, type ReactNode } from "react";
import { X, Layers } from "lucide-react";
const icons: Record<string, string> = {
  codex: "codex.png",
  claude: "claude.png",
  cursor: "cursor.png",
  gemini: "gemini.png",
  opencode: "opencode.png",
  copilot: "copilot.svg",
  vscode: "vscode.png",
  windsurf: "windsurf.svg",
  kiro: "kiro.svg",
  cline: "cline.png",
  roo: "roo.png",
  "claude-desktop": "claude-desktop.png",
};
export function ToolIcon({
  id,
  small = false,
}: {
  id: string;
  small?: boolean;
}) {
  const icon = icons[id];
  return (
    <span className={`tool-icon ${small ? "small" : ""}`} aria-hidden="true">
      {icon ? (
        <img src={`/tool-icons/${icon}`} alt="" draggable={false} />
      ) : (
        <Layers size={small ? 13 : 19} strokeWidth={1.7} />
      )}
    </span>
  );
}
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
export function ErrorBox({ text }: { text: string }) {
  return text ? (
    <div className="error-box" role="alert">
      {text}
    </div>
  ) : null;
}
export function Code({ value }: { value: unknown }) {
  return (
    <pre className="code">
      {typeof value === "string" ? value : JSON.stringify(value, null, 2)}
    </pre>
  );
}

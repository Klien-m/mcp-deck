import { useEffect, useRef, type ReactNode } from "react";
import {
  X,
  Plug,
  Terminal,
  Sparkles,
  Code2,
  Wind,
  Bot,
  Boxes,
  Github,
  Monitor,
  Layers,
} from "lucide-react";
const icons: Record<string, typeof Plug> = {
  codex: Terminal,
  claude: Sparkles,
  cursor: Code2,
  gemini: Sparkles,
  opencode: Terminal,
  copilot: Github,
  vscode: Code2,
  windsurf: Wind,
  kiro: Bot,
  cline: Bot,
  roo: Boxes,
  "claude-desktop": Monitor,
};
export function ToolIcon({
  id,
  small = false,
}: {
  id: string;
  small?: boolean;
}) {
  const Icon = icons[id] || Layers;
  return (
    <span className={`tool-icon ${small ? "small" : ""} tool-${id}`}>
      <Icon size={small ? 13 : 19} strokeWidth={1.7} />
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

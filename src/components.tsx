import { useEffect, useRef, type ReactNode } from "react";
import { X, Layers } from "lucide-react";
type ToolIconAsset = {
  file: string;
  crop?: { canvas: number; inset: number; size: number };
};
const claudeIcon: ToolIconAsset = {
  file: "claude.svg",
};
// Keep official artwork intact; compensate only for transparent canvas padding.
const icons: Record<string, ToolIconAsset> = {
  codex: { file: "codex.png", crop: { canvas: 250, inset: 18, size: 216 } },
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

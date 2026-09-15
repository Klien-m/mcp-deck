import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown } from "lucide-react";

/** value 是稳定选项身份；label 用于显示与键盘前缀搜索，detail/icon 仅辅助展示。 */
type Option = {
  value: string;
  label: string;
  detail?: string;
  icon?: ReactNode;
};

/**
 * 受控单选框：选中值归调用方，组件管理展开、键盘高亮、定位与焦点。
 * 焦点保留在触发按钮，aria-activedescendant 指向高亮项，提交选择后才调用 onChange。
 */
export function Select({
  value,
  options,
  onChange,
  label,
  disabled = false,
}: {
  value: string;
  options: Option[];
  onChange: (value: string) => void;
  label: string;
  disabled?: boolean;
}) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  // 只有键盘导航才自动滚到高亮项，防止用户手动滚动后被强制拉回选中位置。
  const activeSource = useRef<"keyboard" | "pointer">("keyboard");
  const typeahead = useRef({ text: "", at: 0 });
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [position, setPosition] = useState<{
    left: number;
    top: number;
    width: number;
    maxHeight: number;
  } | null>(null);
  const selected = options.findIndex((option) => option.value === value);
  const current = options[selected];

  /** 只调整菜单列表的滚动位置，不调用会连带滚动外层表单的 scrollIntoView。 */
  function revealOption(index: number) {
    const list = menu.current;
    const item = list?.children[index] as HTMLElement | undefined;
    if (!list || !item) return;
    if (item.offsetTop < list.scrollTop) list.scrollTop = item.offsetTop;
    else if (item.offsetTop + item.offsetHeight > list.scrollTop + list.clientHeight) {
      list.scrollTop = item.offsetTop + item.offsetHeight - list.clientHeight;
    }
  }

  /** 键盘高亮立即可见；即使索引未变，也需要处理用户刚刚手动滚走的情况。 */
  function highlight(index: number) {
    activeSource.current = "keyboard";
    setActive(index);
    // 不能只依赖 active 变化触发的 effect，否则相同选项在手动滚动后不会被定位。
    revealOption(index);
  }

  /** 从当前选项或指定索引展开，清除旧定位，等待布局测量后显示菜单。 */
  function show(index = Math.max(0, selected)) {
    if (disabled || !options.length) return;
    trigger.current?.focus();
    highlight(index);
    setPosition(null);
    setOpen(true);
  }

  /** 确认一个有效选项后关闭并恢复按钮焦点；值未变时不重复通知调用方。 */
  function choose(index: number) {
    const option = options[index];
    if (!option || disabled) return;
    setOpen(false);
    trigger.current?.focus();
    if (option.value !== value) onChange(option.value);
  }

  useEffect(() => {
    if (disabled) setOpen(false);
  }, [disabled]);

  // 在绘制前定位；窗口或外层滚动需要重测，菜单自身滚动不能引起重定位和跳回。
  useLayoutEffect(() => {
    if (!open) return;
    /** 按视口剩余空间选择上下展开，限制宽高并保留边缘间距。 */
    function place() {
      const rect = trigger.current?.getBoundingClientRect();
      if (!rect) return;
      const desired = Math.min(312, options.length * 42 + 12);
      const below = window.innerHeight - rect.bottom - 14;
      const above = rect.top - 14;
      const down = below >= Math.min(desired, 180) || below >= above;
      const maxHeight = Math.max(0, Math.min(desired, down ? below : above));
      const width = Math.min(rect.width, window.innerWidth - 24);
      setPosition({
        left: Math.max(12, Math.min(rect.left, window.innerWidth - width - 12)),
        top: down ? rect.bottom + 6 : rect.top - maxHeight - 6,
        width,
        maxHeight,
      });
    }
    /** 点击或焦点移到触发器／菜单以外时关闭；Portal 内节点也算菜单内部。 */
    function outside(event: Event) {
      const target = event.target as Node;
      if (
        !trigger.current?.contains(target) &&
        !popup.current?.contains(target)
      ) {
        setOpen(false);
      }
    }
    function scroll(event: Event) {
      if (!popup.current?.contains(event.target as Node)) place();
    }
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", scroll, true);
    document.addEventListener("pointerdown", outside, true);
    document.addEventListener("focusin", outside, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", scroll, true);
      document.removeEventListener("pointerdown", outside, true);
      document.removeEventListener("focusin", outside, true);
    };
  }, [open, options.length]);

  useLayoutEffect(() => {
    if (open && activeSource.current === "keyboard") revealOption(active);
  }, [open, active, position?.maxHeight]);

  /**
   * 方向键循环高亮，Home/End 定位端点，Enter/空格确认，Escape 仅关闭当前菜单。
   * 连续输入使用 600 ms 前缀缓冲；重复同一字符循环查找相同首字母。
   */
  function keyboard(event: KeyboardEvent<HTMLButtonElement>) {
    if (disabled || !options.length) return;
    if (event.key === "Escape" && open) {
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
    } else if (event.key === "Tab") {
      setOpen(false);
    } else if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
      event.preventDefault();
      const index =
        event.key === "Home"
          ? 0
          : event.key === "End"
            ? options.length - 1
            : open
              ? (active +
                  (event.key === "ArrowDown" ? 1 : -1) +
                  options.length) %
                options.length
              : Math.max(0, selected);
      if (open) highlight(index);
      else show(index);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (open) choose(active);
      else show();
    } else if (
      event.key.length === 1 &&
      !event.metaKey &&
      !event.ctrlKey &&
      !event.altKey
    ) {
      const time = Date.now();
      const key = event.key.toLocaleLowerCase();
      const previous =
        time - typeahead.current.at < 600 ? typeahead.current.text : "";
      const text = previous + key;
      typeahead.current = { text, at: time };
      const query = [...text].every((char) => char === key) ? key : text;
      const start = open ? active : selected;
      for (let offset = 1; offset <= options.length; offset++) {
        const index = (Math.max(0, start) + offset) % options.length;
        if (options[index].label.toLocaleLowerCase().startsWith(query)) {
          event.preventDefault();
          if (open) highlight(index);
          else show(index);
          break;
        }
      }
    }
  }

  return (
    <div className="deck-select">
      <button
        ref={trigger}
        type="button"
        role="combobox"
        aria-label={label}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-controls={open ? id : undefined}
        aria-activedescendant={open ? `${id}-${active}` : undefined}
        className={`select-trigger ${open ? "is-open" : ""}`}
        disabled={disabled || !options.length}
        onClick={() => (open ? setOpen(false) : show())}
        onKeyDown={keyboard}
      >
        {current?.icon && (
          <span className="select-icon" aria-hidden="true">
            {current.icon}
          </span>
        )}
        <span className="select-text">{current?.label || "请选择"}</span>
        {current?.detail && (
          <span className="select-detail">{current.detail}</span>
        )}
        <ChevronDown className="select-chevron" size={16} aria-hidden="true" />
      </button>
      {open &&
        trigger.current &&
        createPortal(
          <div
            ref={popup}
            className="select-menu"
            style={position || { visibility: "hidden" }}
            onMouseDown={(event) => event.preventDefault()}
            onWheel={() => {
              activeSource.current = "pointer";
            }}
          >
            <div
              ref={menu}
              id={id}
              role="listbox"
              aria-label={label}
              className="select-menu-list"
            >
              {options.map((option, index) => (
                <div
                  key={option.value}
                  id={`${id}-${index}`}
                  role="option"
                  aria-selected={option.value === value}
                  className={`select-option ${index === active ? "is-active" : ""}`}
                  onClick={() => choose(index)}
                >
                  {option.icon && (
                    <span className="select-icon" aria-hidden="true">
                      {option.icon}
                    </span>
                  )}
                  <span className="select-text">{option.label}</span>
                  {option.detail && (
                    <span className="select-detail">{option.detail}</span>
                  )}
                  <span className="select-check" aria-hidden="true">
                    {option.value === value && (
                      <Check size={15} strokeWidth={2} />
                    )}
                  </span>
                </div>
              ))}
            </div>
          </div>,
          // 挂入最近 dialog，既脱离表单滚动裁切，也保持在原生弹窗顶层内；主界面则挂到 body。
          trigger.current.closest("dialog") || document.body,
        )}
    </div>
  );
}

import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown } from "lucide-react";

type Option = {
  value: string;
  label: string;
  detail?: string;
  icon?: ReactNode;
};

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
  const menu = useRef<HTMLDivElement>(null);
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

  function revealOption(index: number) {
    const list = menu.current;
    const item = list?.children[index] as HTMLElement | undefined;
    if (!list || !item) return;
    if (item.offsetTop < list.scrollTop) list.scrollTop = item.offsetTop;
    else if (item.offsetTop + item.offsetHeight > list.scrollTop + list.clientHeight) {
      list.scrollTop = item.offsetTop + item.offsetHeight - list.clientHeight;
    }
  }

  function highlight(index: number) {
    activeSource.current = "keyboard";
    setActive(index);
    // Keyboard navigation must also reveal an unchanged option after manual scrolling.
    revealOption(index);
  }

  function show(index = Math.max(0, selected)) {
    if (disabled || !options.length) return;
    trigger.current?.focus();
    highlight(index);
    setPosition(null);
    setOpen(true);
  }

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

  useLayoutEffect(() => {
    if (!open) return;
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
    function outside(event: Event) {
      const target = event.target as Node;
      if (
        !trigger.current?.contains(target) &&
        !menu.current?.contains(target)
      ) {
        setOpen(false);
      }
    }
    function scroll(event: Event) {
      if (!menu.current?.contains(event.target as Node)) place();
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
            ref={menu}
            id={id}
            role="listbox"
            aria-label={label}
            className="select-menu"
            style={position || { visibility: "hidden" }}
            onMouseDown={(event) => event.preventDefault()}
            onWheel={() => {
              activeSource.current = "pointer";
            }}
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
          </div>,
          // Stay in the modal's top layer while escaping the scrolling form body.
          trigger.current.closest("dialog") || document.body,
        )}
    </div>
  );
}

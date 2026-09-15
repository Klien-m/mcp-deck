import type { ReactNode } from "react";
import { Select as SelectRoot, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

type Option = { value: string; label: string; detail?: string; icon?: ReactNode };

/** 保留业务选择器接口，键盘导航、焦点和弹层定位统一交给 Radix。 */
export function Select({ value, options, onChange, label, disabled = false }: {
  value: string;
  options: Option[];
  onChange: (value: string) => void;
  label: string;
  disabled?: boolean;
}) {
  const current = options.find((option) => option.value === value);
  return (
    <div className="deck-select">
      <SelectRoot value={value} onValueChange={onChange} disabled={disabled || !options.length}>
        <SelectTrigger className="w-full" aria-label={label}>
          <SelectValue placeholder="请选择">{current?.icon}{current?.label}</SelectValue>
        </SelectTrigger>
        <SelectContent position="popper" align="start" className="max-h-80">
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value} textValue={option.label}>
              {option.icon}<span>{option.label}</span>
              {option.detail && <span className="text-xs text-muted-foreground">{option.detail}</span>}
            </SelectItem>
          ))}
        </SelectContent>
      </SelectRoot>
    </div>
  );
}

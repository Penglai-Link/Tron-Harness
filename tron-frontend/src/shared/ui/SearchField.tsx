import { Search, X } from "lucide-react";
import type { InputHTMLAttributes } from "react";
import { useTranslation } from "react-i18next";

import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/Button";
import { TextInput } from "@/shared/ui/Forms";

interface SearchFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "onChange" | "value"> {
  ariaLabel?: string;
  className?: string;
  clearLabel?: string;
  inputClassName?: string;
  onChange: (value: string) => void;
  onClear?: () => void;
  value: string;
}

export function SearchField({
  ariaLabel,
  className,
  clearLabel,
  inputClassName,
  onChange,
  onClear,
  placeholder,
  value,
  ...props
}: SearchFieldProps) {
  const { t } = useTranslation("common");

  return (
    <div className={cn("relative", className)}>
      <Search
        className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]"
        size={15}
        aria-hidden="true"
      />
      <TextInput
        aria-label={ariaLabel ?? t("search.ariaLabel")}
        className={cn("h-9 w-full pl-9", value ? "pr-9" : undefined, inputClassName)}
        placeholder={placeholder}
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        {...props}
      />
      {value ? (
        <Button
          aria-label={clearLabel ?? t("search.clear")}
          className="absolute right-1 top-1/2 size-7 -translate-y-1/2"
          size="icon"
          type="button"
          variant="ghost"
          onClick={() => {
            onChange("");
            onClear?.();
          }}
        >
          <X size={14} aria-hidden="true" />
        </Button>
      ) : null}
    </div>
  );
}

import { X } from "lucide-react";
import type { InputHTMLAttributes } from "react";
import { useTranslation } from "react-i18next";
import { useMemo, useState, useEffect } from "react";

import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/Button";
import { TextInput } from "@/shared/ui/Forms";
import { debounce } from "lodash-es";

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
  const [localValue, setLocalValue] = useState(value);

  const debouncedOnChange = useMemo(
    () => debounce((value: string) => onChange(value), 100),
    [onChange]
  );

  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = event.target.value;
    setLocalValue(newValue);
    debouncedOnChange(newValue);
  };

  const handleClear = () => {
    setLocalValue("");
    onChange("");
    onClear?.();
  };

  return (
    <div className={cn("relative", className)}>
      <TextInput
        aria-label={ariaLabel ?? t("search.ariaLabel")}
        placeholder={placeholder}
        type="search"
        value={localValue}
        onChange={handleChange}
        {...props}
      />
      {localValue ? (
        <Button
          aria-label={clearLabel ?? t("search.clear")}
          className="absolute right-1 top-1/2 size-7 -translate-y-1/2"
          size="icon"
          type="button"
          variant="ghost"
          onClick={handleClear}
        >
          <X size={14} aria-hidden="true" />
        </Button>
      ) : null}
    </div>
  );
}

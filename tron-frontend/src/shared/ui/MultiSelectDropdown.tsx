"use client";

import * as React from "react";
import {
  Combobox,
  ComboboxChip,
  ComboboxChips,
  ComboboxChipsInput,
  ComboboxEmpty,
  ComboboxItem,
  ComboboxList,
  ComboboxPopup,
  ComboboxValue,
} from "@/components/ui/combobox";
import { cn } from "@/shared/lib/utils";

export interface MultiSelectOption {
  description?: string;
  disabled?: boolean;
  label: string;
  value: string;
}

export function MultiSelectDropdown({
  ariaLabel,
  "aria-describedby": ariaDescribedBy,
  className,
  disabled = false,
  emptyMessage,
  id,
  loading = false,
  loadingMessage = "Loading options...",
  onToggle,
  options,
  placeholder,
  searchEmptyMessage = "No matching options.",
  selectedValues,
  triggerClassName,
}: {
  "aria-describedby"?: string;
  ariaLabel: string;
  className?: string;
  disabled?: boolean;
  emptyMessage: string;
  id?: string;
  loading?: boolean;
  loadingMessage?: string;
  onToggle?: (value: string) => void;
  options: MultiSelectOption[];
  placeholder: string;
  searchEmptyMessage?: string;
  searchPlaceholder?: string;
  selectedValues: string[];
  triggerClassName?: string;
}) {
  const uniqueSelectedValues = React.useMemo(
    () => Array.from(new Set(selectedValues.filter(Boolean))),
    [selectedValues]
  );

  const selectedOptions = React.useMemo(() => {
    const valueMap = new Map(options.map((opt) => [opt.value, opt]));
    return uniqueSelectedValues
      .map((val) => valueMap.get(val))
      .filter((opt): opt is MultiSelectOption => opt !== undefined);
  }, [options, uniqueSelectedValues]);

  const handleValueChange = React.useCallback(
    (newValues: MultiSelectOption[]) => {
      if (!onToggle) return;

      const newValueSet = new Set(newValues.map((v) => v.value));
      const oldValueSet = new Set(uniqueSelectedValues);

      const added = newValues.find((v) => !oldValueSet.has(v.value));
      const removed = uniqueSelectedValues.find((v) => !newValueSet.has(v));

      if (added) {
        onToggle(added.value);
      } else if (removed) {
        onToggle(removed);
      }
    },
    [onToggle, uniqueSelectedValues]
  );

  if (loading) {
    return (
      <div className={cn("flex w-full items-center rounded-lg border border-input bg-background px-3 py-2 text-sm text-muted-foreground", className)}>
        {loadingMessage}
      </div>
    );
  }

  if (!options.length) {
    return (
      <div className={cn("flex w-full items-center rounded-lg border border-input bg-background px-3 py-2 text-sm text-muted-foreground", className)}>
        {emptyMessage}
      </div>
    );
  }

  return (
    <div className={cn("w-full", className)}>
      <Combobox
        aria-describedby={ariaDescribedBy}
        aria-label={ariaLabel}
        disabled={disabled || !onToggle}
        id={id}
        items={options}
        multiple
        onValueChange={handleValueChange}
        value={selectedOptions}
      >
        <ComboboxChips className={triggerClassName}>
          <ComboboxValue>
            {(value: MultiSelectOption[]) => (
              <>
                {value?.map((item) => (
                  <ComboboxChip aria-label={item.label} key={item.value}>
                    {item.label}
                  </ComboboxChip>
                ))}
                <ComboboxChipsInput
                  aria-label={ariaLabel}
                  placeholder={value.length > 0 ? undefined : placeholder}
                  size="lg"
                />
              </>
            )}
          </ComboboxValue>
        </ComboboxChips>

        <ComboboxPopup>
          <ComboboxEmpty>{searchEmptyMessage}</ComboboxEmpty>
          <ComboboxList>
            {(item) => (
              <ComboboxItem
                disabled={item.disabled}
                key={item.value}
                value={item}
              >
                <div className="flex flex-col">
                  <span>{item.label}</span>
                  {item.description && (
                    <span className="text-xs text-muted-foreground">
                      {item.description}
                    </span>
                  )}
                </div>
              </ComboboxItem>
            )}
          </ComboboxList>
        </ComboboxPopup>
      </Combobox>
    </div>
  );
}

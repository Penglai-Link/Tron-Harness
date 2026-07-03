import {
  type CSSProperties,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Check,
  ChevronDown,
  Search,
} from "lucide-react";

import { useDismissibleLayer } from "@/shared/hooks/useDismissibleLayer";
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
  triggerClassName,
  searchEmptyMessage = "No matching options.",
  searchPlaceholder = "Search options...",
  selectedValues,
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
  triggerClassName?: string;
  searchEmptyMessage?: string;
  searchPlaceholder?: string;
  selectedValues: string[];
}) {
  const [open, setOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [placement, setPlacement] = useState<{ maxHeight: number; side: "bottom" | "top" }>({
    maxHeight: 288,
    side: "bottom",
  });
  const popupRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionByValue = useMemo(() => new Map(options.map((option) => [option.value, option])), [options]);
  const uniqueSelectedValues = useMemo(() => Array.from(new Set(selectedValues.filter(Boolean))), [selectedValues]);
  const selectedItems = uniqueSelectedValues.map((value) => optionByValue.get(value) ?? { label: value, value });
  const hiddenSelectionCount = Math.max(0, selectedItems.length - 4);
  const triggerDisabled = disabled || !onToggle;
  const normalizedSearchQuery = searchQuery.trim().toLowerCase();
  const filteredOptions = useMemo(() => {
    if (!normalizedSearchQuery) return options;
    return options.filter((option) => [
      option.label,
      option.value,
      option.description ?? "",
    ].some((value) => value.toLowerCase().includes(normalizedSearchQuery)));
  }, [normalizedSearchQuery, options]);

  const updatePlacement = useCallback(() => {
    const triggerElement = triggerRef.current;
    if (!triggerElement) return;

    const viewportHeight = window.innerHeight;
    const triggerRect = triggerElement.getBoundingClientRect();
    const spacing = 8;
    const desiredHeight = 288;
    const spaceBelow = Math.max(0, viewportHeight - triggerRect.bottom - spacing);
    const spaceAbove = Math.max(0, triggerRect.top - spacing);
    const side = spaceBelow >= desiredHeight || spaceBelow >= spaceAbove ? "bottom" : "top";
    const availableSpace = side === "bottom" ? spaceBelow : spaceAbove;

    setPlacement({
      maxHeight: Math.max(96, Math.min(desiredHeight, Math.floor(availableSpace))),
      side,
    });
  }, []);

  useLayoutEffect(() => {
    if (!open) return;

    updatePlacement();
    window.addEventListener("resize", updatePlacement);
    window.addEventListener("scroll", updatePlacement, true);

    return () => {
      window.removeEventListener("resize", updatePlacement);
      window.removeEventListener("scroll", updatePlacement, true);
    };
  }, [open, updatePlacement]);

  useEffect(() => {
    if (!open) {
      setSearchQuery("");
      return;
    }
    window.requestAnimationFrame(() => searchInputRef.current?.focus());
  }, [open]);

  useDismissibleLayer({
    onDismiss: () => setOpen(false),
    open,
    popupRef,
    triggerElement: triggerRef.current,
  });

  return (
    <div className={cn("relative w-full", className)}>
      <button
        ref={triggerRef}
        aria-expanded={open}
        aria-label={ariaLabel}
        aria-describedby={ariaDescribedBy}
        className={cn(
          "flex min-h-12 w-full items-center justify-between gap-3 rounded-lg border border-[var(--color-outline)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-left text-sm text-[var(--color-on-surface)] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
          triggerDisabled
            ? "cursor-not-allowed opacity-70"
            : "hover:border-[var(--color-primary)] hover:bg-[var(--color-hover-surface)]",
          triggerClassName,
        )}
        disabled={triggerDisabled}
        id={id}
        type="button"
        onClick={() => setOpen((current) => !current)}
      >
        <span className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5">
          {selectedItems.length ? (
            <>
              {selectedItems.slice(0, 4).map((item) => (
                <span
                  className="max-w-full truncate rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[12px] text-[var(--color-on-surface)]"
                  key={item.value}
                >
                  {item.label}
                </span>
              ))}
              {hiddenSelectionCount ? (
                <span className="rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[12px] text-[var(--color-on-surface-variant)]">
                  +{hiddenSelectionCount}
                </span>
              ) : null}
            </>
          ) : (
            <span className="text-[var(--color-on-surface-variant)]">{loading ? loadingMessage : placeholder}</span>
          )}
        </span>
        <ChevronDown
          aria-hidden="true"
          className={cn("shrink-0 text-[var(--color-on-surface-variant)] transition-transform", open && "rotate-180")}
          size={16}
        />
      </button>
      {open ? (
        <div
          ref={popupRef}
          className={cn(
            "absolute z-40 w-full overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-1 shadow-xl",
            placement.side === "bottom" ? "top-full mt-2" : "bottom-full mb-2",
          )}
          style={{ maxHeight: placement.maxHeight } satisfies CSSProperties}
        >
          {loading ? (
            <p className="px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">{loadingMessage}</p>
          ) : options.length ? (
            <>
              <div className="relative p-1">
                <Search
                  aria-hidden="true"
                  className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]"
                  size={14}
                />
                <input
                  ref={searchInputRef}
                  aria-label={`Search ${ariaLabel}`}
                  className="h-9 w-full rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] pl-8 pr-3 text-sm text-[var(--color-on-surface)] outline-none transition focus:border-[var(--color-focus-ring)] focus:ring-2 focus:ring-[var(--color-focus-ring)]/20"
                  placeholder={searchPlaceholder}
                  type="search"
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                />
              </div>
              <div
                className="overflow-y-auto"
                style={{ maxHeight: Math.max(80, placement.maxHeight - 48) } satisfies CSSProperties}
                role="listbox"
                aria-label={`${ariaLabel} options`}
                aria-multiselectable="true"
              >
                {filteredOptions.length ? filteredOptions.map((option) => {
                  const active = uniqueSelectedValues.includes(option.value);
                  return (
                    <button
                      aria-selected={active}
                      className={cn(
                        "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
                        active
                          ? "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface)] ring-1 ring-inset ring-[var(--color-outline-variant)] hover:bg-[var(--color-surface-container-highest)]"
                          : "text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]",
                        option.disabled && "cursor-not-allowed opacity-60",
                      )}
                      disabled={option.disabled}
                      key={option.value}
                      role="option"
                      type="button"
                      onClick={() => onToggle?.(option.value)}
                    >
                      <span
                        className={cn(
                          "flex h-4 w-4 shrink-0 items-center justify-center rounded border",
                          active
                            ? "border-[var(--color-on-surface)] bg-[var(--color-on-surface)] text-[var(--color-surface-container-lowest)]"
                            : "border-[var(--color-outline)] bg-[var(--color-surface-container-lowest)]",
                        )}
                      >
                        {active ? <Check size={12} aria-hidden="true" /> : null}
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate font-medium">{option.label}</span>
                        {option.description ? (
                          <span className="mt-0.5 block truncate text-[12px] text-[var(--color-on-surface-variant)]">
                            {option.description}
                          </span>
                        ) : null}
                      </span>
                    </button>
                  );
                }) : (
                  <p className="px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">{searchEmptyMessage}</p>
                )}
              </div>
            </>
          ) : (
            <p className="px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">{emptyMessage}</p>
          )}
        </div>
      ) : null}
    </div>
  );
}

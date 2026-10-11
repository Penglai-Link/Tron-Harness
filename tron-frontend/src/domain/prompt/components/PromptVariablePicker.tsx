import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { LoaderCircle } from "lucide-react";

import { fetchPlatformPromptVariableOptions } from "@/domain/variable/api";
import { listPlatformPromptVariableOptions } from "@/domain/variable/mock/data";
import type { PromptVariableOption } from "@/domain/variable/model";
import type { ServiceDataSource } from "@/shared/api/serviceConfig";

import type {
  PromptRuntimeProvider,
  PromptVariable,
} from "@/domain/prompt/model";

export interface PromptVariablePickerTrigger {
  end: number;
  query: string;
  requestId: number;
  start: number;
}

interface PromptVariablePickerLabels {
  empty: string;
  loading: string;
  managed: string;
  noDescription: string;
  runtime: string;
  runtimeDescription: (provider: PromptRuntimeProvider, fallback: string) => string;
  runtimeSourceLabel: (provider: PromptRuntimeProvider, fallback: string) => string;
  searchPlaceholder: string;
}

interface PickerPosition {
  left: number;
  top: number;
  width: number;
}

const pickerHorizontalMargin = 8;
const pickerPreferredWidth = 416;
const pickerEstimatedHeight = 288;

function clamp(value: number, lower: number, upper: number) {
  return Math.min(Math.max(value, lower), Math.max(lower, upper));
}

function textareaCaretPosition(textarea: HTMLTextAreaElement, selectionStart: number) {
  const computed = window.getComputedStyle(textarea);
  const mirror = document.createElement("div");
  const copiedStyleProperties = [
    "boxSizing",
    "borderTopWidth",
    "borderRightWidth",
    "borderBottomWidth",
    "borderLeftWidth",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "fontFamily",
    "fontSize",
    "fontStyle",
    "fontVariant",
    "fontWeight",
    "letterSpacing",
    "lineHeight",
    "textAlign",
    "textIndent",
    "textTransform",
    "wordSpacing",
    "tabSize",
  ] as const;

  mirror.style.position = "absolute";
  mirror.style.visibility = "hidden";
  mirror.style.whiteSpace = "pre-wrap";
  mirror.style.overflowWrap = "break-word";
  mirror.style.wordBreak = "break-word";
  mirror.style.overflow = "hidden";
  mirror.style.top = "0";
  mirror.style.left = "-9999px";
  mirror.style.width = `${textarea.clientWidth}px`;
  copiedStyleProperties.forEach((property) => {
    mirror.style[property] = computed[property];
  });

  mirror.textContent = textarea.value.slice(0, selectionStart);
  const marker = document.createElement("span");
  marker.textContent = textarea.value.slice(selectionStart) || ".";
  mirror.append(marker);
  document.body.append(mirror);

  const lineHeight = Number.parseFloat(computed.lineHeight)
    || Number.parseFloat(computed.fontSize) * 1.4;
  const position = {
    left: marker.offsetLeft - textarea.scrollLeft,
    top: marker.offsetTop - textarea.scrollTop,
    lineHeight,
  };
  mirror.remove();
  return position;
}

export function findPromptVariablePickerTrigger(
  promptBody: string,
  selectionStart: number,
  requestId: number,
): PromptVariablePickerTrigger | null {
  if (selectionStart < 2) return null;
  const start = promptBody.lastIndexOf("{{", selectionStart - 1);
  if (start < 0) return null;
  const query = promptBody.slice(start + 2, selectionStart);
  if (query.includes("}}") || !/^[A-Za-z0-9_.-]*$/.test(query)) return null;
  return { start, end: selectionStart, query, requestId };
}

export function promptVariableAlias(
  variableKey: string,
  variables: PromptVariable[],
): string {
  const existing = variables.find((variable) => (
    variable.sourceKind === "MANAGED"
      && variable.managedVariableKey === variableKey
  ));
  if (existing) return existing.name;

  const normalized = variableKey
    .trim()
    .replace(/[^A-Za-z0-9_]/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "") || "variable";
  const base = /^[A-Za-z_]/.test(normalized) ? normalized : `var_${normalized}`;
  const boundedBase = base.slice(0, 60);
  const occupied = new Set(variables.map((variable) => variable.name));
  if (!occupied.has(boundedBase)) return boundedBase;
  for (let suffix = 2; ; suffix += 1) {
    const candidate = `${boundedBase.slice(0, 64 - String(suffix).length - 1)}_${suffix}`;
    if (!occupied.has(candidate)) return candidate;
  }
}

export function PromptVariablePicker({
  anchorElement,
  dataSource,
  labels,
  onDismiss,
  onSelect,
  trigger,
}: {
  anchorElement: HTMLTextAreaElement | null;
  dataSource: ServiceDataSource;
  labels: PromptVariablePickerLabels;
  onDismiss: () => void;
  onSelect: (option: PromptVariableOption) => void;
  trigger: PromptVariablePickerTrigger | null;
}) {
  const [loading, setLoading] = useState(false);
  const [options, setOptions] = useState<PromptVariableOption[]>([]);
  const [search, setSearch] = useState({ requestId: 0, value: "" });
  const pickerRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<PickerPosition | null>(null);
  const activeSearch = trigger && search.requestId === trigger.requestId
    ? search.value
    : trigger?.query ?? "";

  useEffect(() => {
    if (!trigger) {
      setLoading(false);
      setOptions([]);
      return;
    }

    const controller = new AbortController();
    setLoading(true);
    setOptions([]);
    const request = dataSource === "api"
      ? fetchPlatformPromptVariableOptions({ search: activeSearch }, controller.signal)
      : Promise.resolve(listPlatformPromptVariableOptions(activeSearch));

    request
      .then((records) => {
        if (!controller.signal.aborted) setOptions(records);
      })
      .catch(() => {
        if (!controller.signal.aborted) setOptions([]);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [activeSearch, dataSource, trigger]);

  useEffect(() => {
    if (!trigger) return;
    const closeWhenClickingAway = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (pickerRef.current?.contains(target) || anchorElement?.contains(target)) return;
      onDismiss();
    };
    document.addEventListener("pointerdown", closeWhenClickingAway);
    return () => document.removeEventListener("pointerdown", closeWhenClickingAway);
  }, [anchorElement, onDismiss, trigger]);

  useLayoutEffect(() => {
    if (!trigger || !anchorElement) {
      setPosition(null);
      return;
    }

    const updatePosition = () => {
      const rect = anchorElement.getBoundingClientRect();
      const caret = textareaCaretPosition(anchorElement, trigger.end);
      const width = Math.min(pickerPreferredWidth, window.innerWidth - pickerHorizontalMargin * 2);
      const height = pickerRef.current?.getBoundingClientRect().height || pickerEstimatedHeight;
      const left = clamp(
        rect.left + caret.left,
        pickerHorizontalMargin,
        window.innerWidth - width - pickerHorizontalMargin,
      );
      const below = rect.top + caret.top + caret.lineHeight + 6;
      const top = below + height <= window.innerHeight - pickerHorizontalMargin
        ? below
        : Math.max(pickerHorizontalMargin, rect.top + caret.top - height - 6);
      setPosition({ left, top, width });
    };

    updatePosition();
    const observer = new ResizeObserver(updatePosition);
    observer.observe(anchorElement);
    anchorElement.addEventListener("scroll", updatePosition);
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      observer.disconnect();
      anchorElement.removeEventListener("scroll", updatePosition);
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [activeSearch, anchorElement, loading, options.length, trigger]);

  if (!trigger || !position) return null;

  return createPortal(
    <div
      ref={pickerRef}
      className="fixed z-[140] flex max-h-72 flex-col overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-xl"
      role="listbox"
      aria-label="Prompt variable options"
      style={{ left: position.left, top: position.top, width: position.width }}
    >
      <div className="border-b border-[var(--color-outline-variant)] p-1">
        <input
          className="h-8 w-full rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface)] px-2 text-sm text-[var(--color-on-surface)] outline-none placeholder:text-[var(--color-on-surface-variant)] focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]"
          placeholder={labels.searchPlaceholder}
          type="search"
          value={activeSearch}
          onChange={(event) => setSearch({
            requestId: trigger.requestId,
            value: event.target.value,
          })}
          onKeyDown={(event) => {
            if (event.key === "Escape") onDismiss();
          }}
        />
      </div>
      <div className="min-h-0 overflow-y-auto p-1">
        {loading ? (
          <div className="flex items-center gap-[12px] px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">
            <LoaderCircle size={15} className="animate-spin" aria-hidden="true" />
            {labels.loading}
          </div>
        ) : options.length ? options.map((option) => {
          const isRuntime = option.sourceKind === "RUNTIME";
          const description = isRuntime && option.runtimeProvider
            ? labels.runtimeDescription(option.runtimeProvider, option.description || labels.noDescription)
            : option.description || labels.noDescription;
          const sourceLabel = isRuntime && option.runtimeProvider
            ? labels.runtimeSourceLabel(option.runtimeProvider, option.sourceLabel)
            : option.sourceLabel;
          return (
            <button
              key={option.id}
              className="flex w-full flex-col rounded-md px-3 py-2 text-left hover:bg-[var(--color-surface-container)] focus-visible:bg-[var(--color-surface-container)] focus-visible:outline-none"
              role="option"
              type="button"
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => onSelect(option)}
            >
              <span className="flex items-center gap-[12px]">
                <span className="font-mono text-sm font-medium text-[var(--color-on-surface)]">{option.key}</span>
                <span className={`rounded px-1.5 py-0.5 text-[11px] font-medium ${isRuntime
                  ? "border border-border bg-muted text-muted-foreground"
                  : "border border-primary/30 bg-primary/10 text-primary"}`}
                >
                  {isRuntime ? labels.runtime : labels.managed}
                </span>
              </span>
              <span className="mt-0.5 text-xs text-[var(--color-on-surface-variant)]">
                {description} · {sourceLabel}
              </span>
            </button>
          );
        }) : (
          <p className="px-3 py-2 text-sm text-[var(--color-on-surface-variant)]">{labels.empty}</p>
        )}
      </div>
    </div>,
    document.body,
  );
}

import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, Sparkles } from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { AutomationModelOption } from "@/domain/automation/pages/automationExecutionOptions";

function normalizedSearchValue(value: string) {
  return value.trim().toLowerCase().replace(/[\s\-_/.:]+/g, "");
}

function fuzzySubsequenceMatch(query: string, target: string) {
  if (!query) return true;
  let queryIndex = 0;
  for (const char of target) {
    if (char === query[queryIndex]) queryIndex += 1;
    if (queryIndex === query.length) return true;
  }
  return false;
}

function fuzzyTextMatch(query: string, target: string) {
  const rawQuery = query.trim().toLowerCase();
  if (!rawQuery) return true;
  const rawTarget = target.toLowerCase();
  if (rawTarget.includes(rawQuery)) return true;

  const normalizedQuery = normalizedSearchValue(query);
  if (!normalizedQuery) return true;
  const normalizedTarget = normalizedSearchValue(target);
  return normalizedTarget.includes(normalizedQuery)
    || fuzzySubsequenceMatch(normalizedQuery, normalizedTarget);
}

function fuzzyModelMatch(query: string, model: AutomationModelOption) {
  return fuzzyTextMatch(query, model.displayName ?? model.label)
    || fuzzyTextMatch(query, model.label)
    || fuzzyTextMatch(query, model.modelName ?? "")
    || fuzzyTextMatch(query, model.accessPlatform ?? "")
    || fuzzyTextMatch(query, model.upstreamVendorName ?? model.vendor ?? "");
}

function formatMultiplier(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return null;
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 6 }).format(value);
}

export function TaskExecutionModelSelect({
  ariaLabel,
  disabled = false,
  emptyLabel,
  loading = false,
  loadingLabel,
  onChange,
  options,
  value,
}: {
  ariaLabel: string;
  disabled?: boolean;
  emptyLabel: string;
  loading?: boolean;
  loadingLabel: string;
  onChange: (value: string) => void;
  options: AutomationModelOption[];
  value: string;
}) {
  const [open, setOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const { t } = useTranslation("pages");
  const selectedModel = options.find((model) => model.optionCode === value) ?? null;
  const filteredModels = useMemo(
    () => options.filter((model) => fuzzyModelMatch(searchQuery, model)),
    [options, searchQuery],
  );
  const autoModels = useMemo(
    () => filteredModels.filter((model) => model.automatic),
    [filteredModels],
  );
  const groupedModels = useMemo(() => {
    const groups = new Map<string, { label: string; models: AutomationModelOption[] }>();
    filteredModels
      .filter((model) => !model.automatic)
      .forEach((model) => {
        const qualityType = model.qualityType ?? "High";
        const vendor = model.upstreamVendorName ?? model.vendor ?? t("modelProviders.values.unknown");
        const personAiGroup = model.providerCode === "PersonAi";
        const key = personAiGroup
          ? `${model.providerCode}:${qualityType}:${vendor}`
          : model.accessPlatform ?? "platform";
        const label = personAiGroup
          ? t("chat.management.modelSelector.qualityGroup", {
              quality: t(`chat.management.modelSelector.quality.${qualityType}`),
              vendor,
            })
          : model.accessPlatform ?? t("chat.management.modelSelector.platformModels");
        const group = groups.get(key) ?? { label, models: [] };
        group.models.push(model);
        groups.set(key, group);
      });
    return Array.from(groups.entries()).map(([key, group]) => ({
      key,
      label: group.label,
      models: group.models.sort((left, right) => (
        (left.displayName ?? left.label).localeCompare(right.displayName ?? right.label)
      )),
    }));
  }, [filteredModels, t]);
  const selectedModelLabel = loading
    ? loadingLabel
    : selectedModel?.displayName ?? selectedModel?.label ?? emptyLabel;

  useEffect(() => {
    if (!open) {
      setSearchQuery("");
      return;
    }
    const frame = window.requestAnimationFrame(() => searchInputRef.current?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, [open]);

  function selectModel(optionCode: string) {
    onChange(optionCode);
    setOpen(false);
  }

  function renderModelOption(model: AutomationModelOption) {
    const displayName = model.displayName ?? model.label;
    const title = [displayName, model.modelName, model.upstreamVendorName].filter(Boolean).join("\n");
    const multiplier = model.providerCode === "PersonAi" && !model.automatic
      ? formatMultiplier(model.outputCoefficient)
      : null;
    return (
      <DropdownMenuRadioItem
        className="w-full min-w-0 items-center px-3 py-2"
        key={model.optionCode}
        title={title}
        value={model.optionCode}
      >
        <span className="flex min-w-0 flex-1 items-center gap-2">
          <span className="min-w-0 flex-1 truncate font-medium">{displayName}</span>
          {multiplier ? (
            <span className="shrink-0 font-mono text-[10px] text-[var(--color-on-surface-variant)]">
              {multiplier}×
            </span>
          ) : null}
        </span>
      </DropdownMenuRadioItem>
    );
  }

  return (
    <div className="relative min-w-0">
      <DropdownMenu open={open} onOpenChange={setOpen}>
        <DropdownMenuTrigger
          aria-label={ariaLabel}
          className="flex h-10 w-full min-w-0 items-center gap-2 rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 text-sm text-[var(--color-on-surface)] shadow-sm outline-none transition hover:bg-[var(--color-hover-surface)] focus-visible:border-[var(--color-focus-ring)] focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]/20 disabled:pointer-events-none disabled:opacity-50"
          disabled={disabled || options.length === 0}
          title={selectedModelLabel}
        >
          <Sparkles aria-hidden="true" className="shrink-0 text-[var(--color-on-surface-variant)]" size={15} />
          <span className="min-w-0 flex-1 truncate text-left">{selectedModelLabel}</span>
          <ChevronDown aria-hidden="true" className="shrink-0 text-[var(--color-on-surface-variant)]" size={15} />
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align="start"
          className="max-h-80 w-[min(36rem,calc(100vw-2rem))] max-w-[calc(100vw-2rem)] overflow-y-auto overscroll-contain p-1.5"
          sideOffset={4}
        >
          <div className="sticky top-0 z-10 bg-popover pb-1.5">
            <input
              ref={searchInputRef}
              aria-label={t("chat.management.modelSelector.searchAria")}
              className="h-9 w-full rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-2.5 text-sm text-[var(--color-on-surface)] outline-none focus:border-[var(--color-focus-ring)]"
              placeholder={t("chat.management.modelSelector.searchPlaceholder")}
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              onKeyDown={(event) => {
                event.stopPropagation();
                if (event.key !== "Enter") return;
                const firstModel = filteredModels[0];
                if (!firstModel) return;
                event.preventDefault();
                selectModel(firstModel.optionCode);
              }}
            />
          </div>
          {filteredModels.length ? (
            <DropdownMenuRadioGroup value={value} onValueChange={selectModel}>
              {autoModels.map(renderModelOption)}
              {groupedModels.map((group) => (
                <div key={group.key} className="mb-1 last:mb-0">
                  <p className="px-3 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wide text-[var(--color-on-surface-variant)]">
                    {group.label}
                  </p>
                  {group.models.map(renderModelOption)}
                </div>
              ))}
            </DropdownMenuRadioGroup>
          ) : (
            <p className="px-3 py-2 text-xs text-[var(--color-on-surface-variant)]">
              {t("chat.management.modelSelector.noMatch", { query: searchQuery.trim() })}
            </p>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

import {
  Check,
  ChevronDown,
  RotateCcw,
  Search,
} from "lucide-react";
import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { useTranslation } from "react-i18next";

import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import type { ModelSelection, ReasoningEffort } from "@/domain/model/model";
import { cn } from "@/shared/lib/utils";

import "./ChatModelEffortSelector.css";

/** The model fields this selector renders; `ChatModelOption` satisfies them. */
export interface ChatModelEffortModel {
  accessPlatform: string | null;
  displayName: string;
  enabled: boolean;
  modelName: string;
  modelSelection: ModelSelection | null;
  optionCode: string;
  outputCoefficient: number | null;
  providerCode: string | null;
  qualityType: "High" | "Middle" | "Low" | null;
  upstreamVendorName: string | null;
  vendor: string | null;
}

export interface ChatModelEffortChoice {
  label: string;
  value: ReasoningEffort;
}

interface ChatModelEffortSelectorProps {
  defaultEffort: ReasoningEffort | null;
  disabled?: boolean;
  effort: ReasoningEffort | null;
  effortOptions: ChatModelEffortChoice[];
  emptyLabel: string;
  loading: boolean;
  models: ChatModelEffortModel[];
  onSelectEffort: (effort: ReasoningEffort | null) => void;
  onSelectModel: (modelCode: string) => void;
  selectedModelCode: string;
}

interface EffortStep {
  label: string;
  value: ReasoningEffort | null;
}

const intelligenceFrom = "var(--chat-intelligence-from, #4f7cff)";
const intelligenceTo = "var(--chat-intelligence-to, #a855f7)";
const intelligenceAccent = "var(--chat-intelligence-accent, #8b5cf6)";

function normalizedFuzzyValue(value: string) {
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

  const normalizedQuery = normalizedFuzzyValue(query);
  if (!normalizedQuery) return true;
  const normalizedTarget = normalizedFuzzyValue(target);
  if (normalizedTarget.includes(normalizedQuery)) return true;
  return fuzzySubsequenceMatch(normalizedQuery, normalizedTarget);
}

function fuzzyModelMatch(query: string, model: ChatModelEffortModel) {
  return fuzzyTextMatch(query, model.displayName)
    || fuzzyTextMatch(query, model.modelName)
    || fuzzyTextMatch(query, model.accessPlatform ?? "")
    || fuzzyTextMatch(query, model.upstreamVendorName ?? model.vendor ?? "");
}

function isPersonAiAutoModel(model: ChatModelEffortModel) {
  return model.modelSelection?.kind === "Auto"
    || (model.modelSelection?.kind === "ProviderRoute" && model.modelSelection.routeCode === "ChatAuto");
}

function formatModelMultiplier(value: number | null) {
  if (value === null || !Number.isFinite(value)) return null;
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 6 }).format(value);
}

export function ChatModelEffortSelector({
  defaultEffort,
  disabled = false,
  effort,
  effortOptions,
  emptyLabel,
  loading,
  models,
  onSelectEffort,
  onSelectModel,
  selectedModelCode,
}: ChatModelEffortSelectorProps) {
  const { t } = useTranslation("pages");
  const [open, setOpen] = useState(false);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const radioGroupName = useId();
  const sliderId = useId();

  const selectedModel = models.find((model) => model.optionCode === selectedModelCode) ?? null;
  const modelDisplayName = (model: ChatModelEffortModel) => (
    isPersonAiAutoModel(model)
      ? t("chat.management.modelSelector.autoName")
      : model.displayName
  );
  const selectedModelLabel = loading
    ? t("chat.management.modelSelector.loading")
    : selectedModel
      ? modelDisplayName(selectedModel)
      : emptyLabel;

  const steps = useMemo<EffortStep[]>(() => {
    if (!effortOptions.length) return [];
    if (defaultEffort) return effortOptions.map((option) => ({ ...option }));
    return [
      { label: t("common:reasoningEffort.chatAutomatic"), value: null },
      ...effortOptions.map((option) => ({ ...option })),
    ];
  }, [defaultEffort, effortOptions, t]);
  const lastStep = steps.length - 1;
  const preferredEffort = effort ?? defaultEffort;
  const matchedIndex = preferredEffort
    ? steps.findIndex((option) => option.value === preferredEffort)
    : -1;
  const stepIndex = matchedIndex >= 0 ? matchedIndex : 0;
  const effortLabel = steps[stepIndex]?.label
    ?? t("common:reasoningEffort.chatAutomatic");
  const hasSlider = steps.length > 0;

  const filteredModels = useMemo(
    () => models.filter((model) => model.enabled && fuzzyModelMatch(searchQuery, model)),
    [models, searchQuery],
  );
  const autoModels = useMemo(
    () => filteredModels.filter(isPersonAiAutoModel),
    [filteredModels],
  );
  const groupedModels = useMemo(() => {
    const groups = new Map<string, { key: string; label: string; models: ChatModelEffortModel[] }>();
    filteredModels
      .filter((model) => !isPersonAiAutoModel(model))
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
          : model.accessPlatform ?? t("chat.management.modelSelector.providerModels");
        const group = groups.get(key) ?? { key, label, models: [] };
        group.models.push(model);
        groups.set(key, group);
      });
    return Array.from(groups.values()).map((group) => ({
      ...group,
      models: group.models.sort((left, right) => left.displayName.localeCompare(right.displayName)),
    }));
  }, [filteredModels, t]);

  const modelTitle = (model: ChatModelEffortModel) => (
    [modelDisplayName(model), model.upstreamVendorName].filter(Boolean).join("\n")
  );
  const outputMultiplier = (model: ChatModelEffortModel) => {
    if (model.providerCode !== "PersonAi" || isPersonAiAutoModel(model)) return null;
    return formatModelMultiplier(model.outputCoefficient);
  };

  useEffect(() => {
    if (!open) {
      setSearchQuery("");
      return;
    }
    setModelsOpen(!hasSlider);
    if (!hasSlider) return;
    const frame = window.requestAnimationFrame(() => {
      document.getElementById(sliderId)?.focus({ preventScroll: true });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [hasSlider, open, sliderId]);

  useEffect(() => {
    if (!modelsOpen) return;
    const frame = window.requestAnimationFrame(() => {
      searchInputRef.current?.focus({ preventScroll: true });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [modelsOpen]);

  function handleResetEffort() {
    if (effort === null) return;
    onSelectEffort(null);
  }

  function handleSelectModel(modelCode: string) {
    if (modelCode === selectedModelCode) {
      setModelsOpen(false);
      return;
    }
    onSelectModel(modelCode);
    setOpen(false);
  }

  const triggerTitle = `${selectedModelLabel} · ${effortLabel}`;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        aria-label={t("chat.management.modelSelector.triggerAria", {
          model: selectedModelLabel,
          effort: effortLabel,
        })}
        className="inline-flex h-8 min-w-0 max-w-[15rem] flex-none items-center gap-1.5 rounded-md border-0 bg-transparent px-1.5 text-xs text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] disabled:pointer-events-none disabled:opacity-50 data-popup-open:bg-[var(--color-hover-surface)] data-popup-open:text-[var(--color-on-surface)]"
        disabled={disabled}
        title={triggerTitle}
      >
        <span
          aria-hidden="true"
          className="relative flex size-3.5 flex-none items-center justify-center rounded-full"
          style={{ background: `linear-gradient(135deg, ${intelligenceFrom}, ${intelligenceTo})` }}
        >
          <span className="size-1 rounded-full bg-white/90" />
        </span>
        <span className="min-w-0 truncate font-medium">{selectedModelLabel}</span>
        {hasSlider ? (
          <>
            <span aria-hidden="true" className="flex-none text-[var(--color-outline)]">·</span>
            <span className="flex-none text-[var(--color-on-surface-variant)]">{effortLabel}</span>
          </>
        ) : null}
        <ChevronDown aria-hidden="true" className="size-3.5 flex-none opacity-60" />
      </PopoverTrigger>
      <PopoverContent
        align="start"
        aria-label={t("chat.management.modelSelector.panelAria")}
        className="w-[min(22rem,calc(100vw-1rem))] p-0"
        side="top"
        sideOffset={8}
      >
        <div className="p-3">
          {hasSlider ? (
            <>
              <div className="flex items-start gap-2">
                <div className="min-w-0 flex-1">
                  <p className="text-[11px] font-medium text-[var(--color-on-surface-variant)]">
                    {t("common:reasoningEffort.chatLabel")}
                  </p>
                  <p
                    className="mt-0.5 truncate text-xl font-semibold leading-tight"
                    style={{ color: intelligenceAccent }}
                    title={effortLabel}
                  >
                    {effortLabel}
                  </p>
                </div>
                {effort === null ? null : (
                  <button
                    aria-label={t("chat.management.modelSelector.resetEffortAria")}
                    className="mt-0.5 inline-flex size-7 flex-none items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] motion-reduce:transition-none"
                    onClick={handleResetEffort}
                    title={t("chat.management.modelSelector.resetEffort")}
                    type="button"
                  >
                    <RotateCcw aria-hidden="true" className="size-3.5" />
                  </button>
                )}
              </div>

              <IntelligenceSlider
                id={sliderId}
                label={t("common:reasoningEffort.chatLabel")}
                onChange={(index) => onSelectEffort(steps[index]?.value ?? null)}
                steps={steps}
                value={stepIndex}
              />
              <div className="mt-1 flex items-center justify-between text-[10px] text-[var(--color-on-surface-variant)]">
                <span>{t("chat.management.modelSelector.sliderLow")}</span>
                <span>{t("chat.management.modelSelector.sliderHigh")}</span>
              </div>
            </>
          ) : (
            <p className="text-[11px] font-medium text-[var(--color-on-surface-variant)]">
              {t("chat.management.modelSelector.currentModel")}
            </p>
          )}

          <button
            aria-expanded={modelsOpen}
            className={cn(
              "flex w-full min-w-0 items-center gap-1.5 rounded-md border border-[var(--color-outline-variant)] px-2 py-1.5 text-left text-xs text-[var(--color-on-surface-variant)] transition hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]",
              hasSlider ? "mt-3" : "mt-1",
            )}
            onClick={() => setModelsOpen((current) => !current)}
            type="button"
          >
            <span className="min-w-0 flex-1 truncate">
              <span className="sr-only">{t("chat.management.modelSelector.currentModel")}</span>
              <span className="font-medium text-[var(--color-on-surface)]">{selectedModelLabel}</span>
            </span>
            <span className="flex-none text-[10px]">
              {modelsOpen
                ? t("chat.management.modelSelector.hideModels")
                : t("chat.management.modelSelector.changeModel")}
            </span>
            <ChevronDown
              aria-hidden="true"
              className={cn("size-3.5 flex-none transition-transform", modelsOpen && "rotate-180")}
            />
          </button>
        </div>

        {modelsOpen ? (
          <div className="border-t border-[var(--color-outline-variant)] p-1.5">
            <div className="relative pb-1.5">
              <Search
                aria-hidden="true"
                className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-[var(--color-on-surface-variant)]"
              />
              <input
                ref={searchInputRef}
                aria-label={t("chat.management.modelSelector.searchAria")}
                className="h-8 w-full rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] pl-7 pr-2 text-xs text-[var(--color-on-surface)] outline-none focus:border-[var(--color-focus-ring)]"
                onChange={(event) => setSearchQuery(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key !== "Enter") return;
                  const firstModel = filteredModels.find((model) => model.enabled);
                  if (!firstModel) return;
                  event.preventDefault();
                  handleSelectModel(firstModel.optionCode);
                }}
                placeholder={t("chat.management.modelSelector.searchPlaceholder")}
                value={searchQuery}
              />
            </div>
            <div className="max-h-60 overflow-y-auto overscroll-contain">
              {filteredModels.length ? (
                <div aria-label={t("chat.management.modelSelector.selectAria")} role="radiogroup">
                  {autoModels.map((model) => (
                    <ModelRadioRow
                      checked={model.optionCode === selectedModelCode}
                      groupName={radioGroupName}
                      key={model.optionCode}
                      label={modelDisplayName(model)}
                      multiplier={null}
                      onSelect={() => handleSelectModel(model.optionCode)}
                      title={modelTitle(model)}
                    />
                  ))}
                  {groupedModels.map((group) => (
                    <div key={group.key} className="mb-1 last:mb-0">
                      <p className="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wide text-[var(--color-on-surface-variant)]">
                        {group.label}
                      </p>
                      {group.models.map((model) => (
                        <ModelRadioRow
                          checked={model.optionCode === selectedModelCode}
                          groupName={radioGroupName}
                          key={model.optionCode}
                          label={modelDisplayName(model)}
                          multiplier={outputMultiplier(model)}
                          onSelect={() => handleSelectModel(model.optionCode)}
                          title={modelTitle(model)}
                        />
                      ))}
                    </div>
                  ))}
                </div>
              ) : (
                <p className="px-2 py-2 text-xs text-[var(--color-on-surface-variant)]">
                  {t("chat.management.modelSelector.noMatch", { query: searchQuery.trim() })}
                </p>
              )}
            </div>
          </div>
        ) : null}
      </PopoverContent>
    </Popover>
  );
}

function ModelRadioRow({
  checked,
  groupName,
  label,
  multiplier,
  onSelect,
  title,
}: {
  checked: boolean;
  groupName: string;
  label: string;
  multiplier: string | null;
  onSelect: () => void;
  title: string;
}) {
  return (
    <label
      className={cn(
        "flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-xs transition hover:bg-[var(--color-hover-surface)] focus-within:bg-[var(--color-hover-surface)] focus-within:ring-2 focus-within:ring-inset focus-within:ring-[var(--color-focus-ring)]",
        checked && "bg-[var(--color-secondary-container)] text-[var(--color-on-secondary-container)]",
      )}
      title={title}
    >
      <input
        checked={checked}
        className="sr-only"
        name={groupName}
        onChange={onSelect}
        type="radio"
      />
      <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
      {multiplier ? (
        <span className="flex-none font-mono text-[10px] text-[var(--color-on-surface-variant)]">
          {multiplier}×
        </span>
      ) : null}
      {checked ? <Check aria-hidden="true" className="size-3.5 flex-none" /> : null}
    </label>
  );
}

function IntelligenceSlider({
  id,
  label,
  onChange,
  steps,
  value,
}: {
  id: string;
  label: string;
  onChange: (index: number) => void;
  steps: EffortStep[];
  value: number;
}) {
  const trackRef = useRef<HTMLDivElement | null>(null);
  const [dragging, setDragging] = useState(false);
  const lastStep = steps.length - 1;
  const safeValue = Math.min(Math.max(value, 0), Math.max(lastStep, 0));
  const ratio = lastStep > 0 ? safeValue / lastStep : 1;

  function commitAt(clientX: number) {
    const track = trackRef.current;
    if (!track) return;
    const bounds = track.getBoundingClientRect();
    if (bounds.width <= 0) return;
    const nextRatio = Math.min(1, Math.max(0, (clientX - bounds.left) / bounds.width));
    const nextIndex = Math.round(nextRatio * lastStep);
    if (nextIndex !== safeValue) onChange(nextIndex);
  }

  function handlePointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.focus({ preventScroll: true });
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragging(true);
    commitAt(event.clientX);
  }

  function handlePointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    if (!dragging) return;
    commitAt(event.clientX);
  }

  function handlePointerUp(event: ReactPointerEvent<HTMLDivElement>) {
    if (!dragging) return;
    setDragging(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    let next = safeValue;
    switch (event.key) {
      case "ArrowRight":
      case "ArrowUp":
        next = Math.min(lastStep, safeValue + 1);
        break;
      case "ArrowLeft":
      case "ArrowDown":
        next = Math.max(0, safeValue - 1);
        break;
      case "Home":
        next = 0;
        break;
      case "End":
        next = lastStep;
        break;
      case "PageUp":
        next = Math.min(lastStep, safeValue + 2);
        break;
      case "PageDown":
        next = Math.max(0, safeValue - 2);
        break;
      default:
        return;
    }
    event.preventDefault();
    if (next !== safeValue) onChange(next);
  }

  const currentStep = steps[safeValue];
  const hue = 224 + ratio * 54;
  const glowColor = `hsl(${hue} 95% 66%)`;
  const fillStart = `hsl(214 96% 62%)`;
  const fillMid = `hsl(${hue - 10} 95% 64%)`;
  const fillEnd = `hsl(${hue} 92% 66%)`;
  const atMax = lastStep > 0 && safeValue === lastStep;
  // Keep the 28px knob fully inside the rounded track at both ends.
  const thumbLeft = `calc(${ratio * 100}% + ${(0.5 - ratio) * 28}px)`;

  return (
    <div className="mt-3">
      <div
        ref={trackRef}
        aria-label={label}
        aria-valuemax={lastStep}
        aria-valuemin={0}
        aria-valuenow={safeValue}
        aria-valuetext={currentStep?.label}
        className={cn(
          "group relative flex h-9 touch-none select-none items-center rounded-full outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-surface-container-lowest)]",
          dragging ? "cursor-grabbing" : "cursor-pointer",
        )}
        id={id}
        onClick={(event) => {
          if (dragging) return;
          commitAt(event.clientX);
        }}
        onKeyDown={handleKeyDown}
        onPointerCancel={handlePointerUp}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        role="slider"
        tabIndex={0}
      >
        <span
          aria-hidden="true"
          className={cn(
            "pointer-events-none absolute top-1/2 size-8 -translate-x-1/2 -translate-y-1/2 rounded-full blur-md transition-[left,background-color,opacity] duration-200 ease-out motion-reduce:transition-none",
            atMax && "chat-intelligence-glow-pulse",
          )}
          style={{
            left: thumbLeft,
            backgroundColor: glowColor,
            opacity: dragging ? 0.38 : 0.18,
          }}
        />
        <span
          aria-hidden="true"
          className="absolute inset-x-0 top-1/2 h-8 -translate-y-1/2 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]"
        >
          <span
            className={cn(
              "absolute inset-0 opacity-95 transition-[filter] duration-200 ease-out motion-reduce:transition-none",
              dragging && "brightness-110 saturate-[1.12]",
            )}
            style={{
              background: `linear-gradient(90deg, ${fillStart} 0%, ${fillStart} 2%, ${fillMid} 52%, ${fillEnd} 100%)`,
              boxShadow: "inset 0 1px 0 rgb(255 255 255 / 0.22)",
            }}
          >
            <span
              aria-hidden="true"
              className="chat-intelligence-sparks absolute inset-0 opacity-40"
              style={{
                backgroundImage:
                  "radial-gradient(circle at 18% 32%, rgb(255 255 255 / 0.35) 0 0.5px, transparent 1.5px), radial-gradient(circle at 42% 72%, rgb(255 255 255 / 0.28) 0 0.5px, transparent 1.5px), radial-gradient(circle at 68% 28%, rgb(255 255 255 / 0.32) 0 0.5px, transparent 1.5px), radial-gradient(circle at 84% 66%, rgb(255 255 255 / 0.25) 0 0.5px, transparent 1.5px)",
              }}
            />
          </span>
          <span
            aria-hidden="true"
            className="absolute inset-y-0 right-0 rounded-r-full bg-[var(--color-surface-container-high)] transition-[left] duration-150 ease-out motion-reduce:transition-none"
            style={{ left: thumbLeft }}
          />
        </span>
        <span
          aria-hidden="true"
          className={cn(
            "absolute top-1/2 size-7 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white/60 bg-white transition-[left,transform,box-shadow] duration-150 ease-out motion-reduce:transition-none",
            dragging && "scale-105",
          )}
          style={{
            left: thumbLeft,
            boxShadow: `0 0 0 1px rgb(255 255 255 / 0.22), 0 2px 10px rgb(0 0 0 / 0.35), 0 0 ${dragging ? 18 : 10}px ${glowColor}`,
          }}
        />
      </div>
    </div>
  );
}

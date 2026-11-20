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
  type CSSProperties,
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
    setModelsOpen(false);
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
  const [trackWidth, setTrackWidth] = useState(0);
  const lastStep = steps.length - 1;
  const safeValue = Math.min(Math.max(value, 0), Math.max(lastStep, 0));
  const ratio = lastStep > 0 ? safeValue / lastStep : 1;

  // The network is laid out in real pixels, so it needs the measured track width.
  useEffect(() => {
    const track = trackRef.current;
    if (!track) return;
    const measure = () => setTrackWidth(track.clientWidth);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(track);
    return () => observer.disconnect();
  }, []);

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
            <IntelligenceNetwork width={trackWidth} />
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

const intelligenceNetHeight = 32;

/** Deterministic pseudo-random so the constellation is identical on every render. */
function netNoise(seed: number) {
  const value = Math.sin(seed * 127.1 + 311.7) * 43758.5453;
  return value - Math.floor(value);
}

interface IntelligenceNetNode {
  hub: boolean;
  index: number;
  t: number;
  x: number;
  y: number;
}

interface IntelligenceNetLink {
  delay: string;
  d: string;
  key: string;
  opacity: number;
}

interface IntelligenceNetGraph {
  chain: { d: string; length: number } | null;
  links: IntelligenceNetLink[];
  nodes: IntelligenceNetNode[];
}

/**
 * Lays out a connected mesh: two bands per column, each node linked to the
 * nearest node of the next column, and a hub short of the deep end. Everything
 * is deterministic for a given width, so it can be cached across renders.
 */
function buildIntelligenceNet(width: number): IntelligenceNetGraph | null {
  if (width < 96) return null;

  const columns = Math.max(5, Math.min(9, Math.round((width - 30) / 38)));
  const firstX = 20;
  const step = (width - 40) / (columns - 1);
  const grid: IntelligenceNetNode[][] = [];
  const nodes: IntelligenceNetNode[] = [];

  for (let column = 0; column < columns; column += 1) {
    const t = column / (columns - 1);
    const x = firstX + column * step + (netNoise(column * 7 + 2) - 0.5) * 8;
    const bands = [{ base: 8.5, row: 0 }, { base: 23.5, row: 1 }];
    if (t > 0.55 && netNoise(column * 4 + 9) > 0.6) bands.push({ base: 16, row: 2 });
    grid.push(bands.map(({ base, row }) => {
      const node: IntelligenceNetNode = {
        hub: false,
        index: nodes.length,
        t,
        x,
        y: Math.max(4.5, Math.min(27.5, base + (netNoise(column * 13 + row * 5) - 0.5) * 7)),
      };
      nodes.push(node);
      return node;
    }));
  }

  const hub = grid[columns - 2][0];
  hub.hub = true;

  const links: IntelligenceNetLink[] = [];
  const edges: Array<[number, number]> = [];
  const seen = new Set<string>();
  const addLink = (from: IntelligenceNetNode, to: IntelligenceNetNode) => {
    const key = from.index < to.index ? `${from.index}-${to.index}` : `${to.index}-${from.index}`;
    if (seen.has(key)) return;
    seen.add(key);
    edges.push([from.index, to.index]);
    const midX = (from.x + to.x) / 2 + (netNoise(from.index * 13 + to.index) - 0.5) * 6;
    const midY = (from.y + to.y) / 2 + (netNoise(from.index * 7 + to.index * 3) - 0.5) * 9;
    links.push({
      d: `M${from.x.toFixed(1)} ${from.y.toFixed(1)} Q${midX.toFixed(1)} ${midY.toFixed(1)} ${to.x.toFixed(1)} ${to.y.toFixed(1)}`,
      delay: `${(-netNoise(from.index * 1.7 + to.index) * 6.8).toFixed(2)}s`,
      key,
      opacity: 0.24 + 0.32 * from.t,
    });
  };

  for (let column = 0; column < columns - 1; column += 1) {
    const t = column / (columns - 1);
    const nextColumn = grid[column + 1];
    grid[column].forEach((from) => {
      const sorted = [...nextColumn].sort(
        (left, right) => Math.abs(left.y - from.y) - Math.abs(right.y - from.y),
      );
      addLink(from, sorted[0]);
      if (t > 0.45) addLink(from, sorted[1] ?? sorted[0]);
    });
    if (t > 0.6) addLink(grid[column][0], grid[column][grid[column].length - 1]);
  }

  // A shortest path through the mesh; the travelling spark rides along it.
  const adjacency = new Map<number, number[]>();
  nodes.forEach((node) => adjacency.set(node.index, []));
  edges.forEach(([from, to]) => {
    adjacency.get(from)?.push(to);
    adjacency.get(to)?.push(from);
  });

  const startNode = grid[Math.min(2, columns - 1)][0];
  const previous = new Map<number, number>([[startNode.index, -1]]);
  const queue = [startNode.index];
  while (queue.length) {
    const current = queue.shift();
    if (current === undefined || current === hub.index) break;
    for (const next of adjacency.get(current) ?? []) {
      if (previous.has(next)) continue;
      previous.set(next, current);
      queue.push(next);
    }
  }

  let chain: IntelligenceNetGraph["chain"] = null;
  if (previous.has(hub.index)) {
    const path: IntelligenceNetNode[] = [];
    let cursor = hub.index;
    while (cursor !== -1) {
      path.push(nodes[cursor]);
      cursor = previous.get(cursor) ?? -1;
    }
    path.reverse();
    if (path.length > 1) {
      const d = path
        .map((node, index) => `${index ? "L" : "M"}${node.x.toFixed(1)} ${node.y.toFixed(1)}`)
        .join(" ");
      const length = path.slice(1).reduce(
        (sum, node, index) => sum + Math.hypot(node.x - path[index].x, node.y - path[index].y),
        0,
      );
      chain = { d, length };
    }
  }

  return { chain, links, nodes };
}

/** Decorative thought-web inside the fill; motion is pure CSS and aria-hidden. */
function IntelligenceNetwork({ width }: { width: number }) {
  const graph = useMemo(() => buildIntelligenceNet(width), [width]);
  if (!graph) return null;

  return (
    <svg
      aria-hidden="true"
      className="chat-intelligence-net"
      height={intelligenceNetHeight}
      viewBox={`0 0 ${Math.round(width)} ${intelligenceNetHeight}`}
      width={width}
    >
      {graph.links.map((link) => (
        <path
          className="chat-intelligence-net-link"
          d={link.d}
          key={link.key}
          strokeOpacity={link.opacity}
          style={{ animationDelay: link.delay }}
        />
      ))}
      {graph.nodes.map((node) => (
        <g key={node.index}>
          {node.hub ? (
            <circle cx={node.x} cy={node.y} fill="rgb(255 255 255 / 0.15)" r={6} />
          ) : null}
          <circle
            className="chat-intelligence-net-node"
            cx={node.x}
            cy={node.y}
            fill="#fff"
            fillOpacity={0.6 + 0.35 * node.t}
            r={node.hub ? 2.6 : 1.1 + 1.05 * node.t}
            style={{ animationDelay: `${(-netNoise(node.index * 2.3) * 5.4).toFixed(2)}s` }}
          />
        </g>
      ))}
      {graph.chain ? (
        <path
          className="chat-intelligence-net-spark"
          d={graph.chain.d}
          style={{ "--chat-net-length": `${graph.chain.length.toFixed(1)}px` } as CSSProperties}
        />
      ) : null}
    </svg>
  );
}

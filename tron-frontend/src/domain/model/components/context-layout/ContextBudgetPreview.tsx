import { useState } from "react";
import { useTranslation } from "react-i18next";
import { TextInput } from "@/components/forms";
import { contextBlockColor } from "./contextBlockColors";
import { InfoTooltip } from "@/components/info-tooltip";

export interface PreviewBudget {
  id: string;
  name: string;
  share: number;
  tokens: number;
}

export function ContextBudgetPreview({ budgets, windowTokens, onWindowChange, highlightedId, onHighlightChange }: {
  budgets: PreviewBudget[];
  windowTokens: number;
  onWindowChange: (tokens: number) => void;
  highlightedId: string | null;
  onHighlightChange: (id: string | null) => void;
}) {
  const { t, i18n } = useTranslation("pages");
  const [windowEditing, setWindowEditing] = useState(false);
  const [windowDraft, setWindowDraft] = useState("");
  const number = new Intl.NumberFormat(i18n.language, { notation: "compact", maximumFractionDigits: 1 });
  const percentage = (tokens: number) => `${Number((tokens / windowTokens * 100).toFixed(2))}%`;
  const segments = budgets.map(budget => ({
    ...budget,
    color: contextBlockColor(budget.id),
  })).filter(budget => budget.tokens > 0);
  const totalShare = budgets.reduce((sum, budget) => sum + budget.share, 0);
  const active = segments.find(segment => segment.id === highlightedId);
  const label = (segment: typeof segments[number]) => `${segment.name} · ${number.format(segment.tokens)} tokens · ${percentage(segment.tokens)}`;

  return (
    <section aria-label={t("contextLayout.previewTitle")} className="mb-5 min-w-0 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-4">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-sm font-medium">
          {t("contextLayout.previewTitle")}
          <InfoTooltip ariaLabel={t("contextLayout.previewTitle")}>{t("contextLayout.previewHelp")}</InfoTooltip>
        </div>
        <label className="flex items-center gap-2 text-xs text-[var(--color-on-surface-variant)]">
          {t("contextLayout.previewWindow")}
          <TextInput type="number" min={1000} max={100_000_000} step={1000} value={windowEditing ? windowDraft : windowTokens}
            className="h-8 w-32 text-right tabular-nums" aria-label={t("contextLayout.previewWindow")}
            onFocus={() => { setWindowDraft(String(windowTokens)); setWindowEditing(true); }}
            onBlur={() => setWindowEditing(false)}
            onChange={event => {
              setWindowDraft(event.currentTarget.value);
              const value = Number(event.currentTarget.value);
              if (Number.isSafeInteger(value) && value >= 1000 && value <= 100_000_000) onWindowChange(value);
            }} />
          <span>tokens</span>
        </label>
      </div>
      <div className="mb-2 flex min-h-5 flex-wrap justify-between gap-2 text-xs text-[var(--color-on-surface-variant)]">
        <span className={totalShare === 10_000 ? "" : "text-[var(--color-error)]"}>{t("contextLayout.totalShare", { value: totalShare / 100 })}</span>
        <span className="tabular-nums" aria-live="polite">{active ? label(active) : t("contextLayout.previewBlockCount", { count: budgets.length })}</span>
      </div>
      <div className="flex h-12 overflow-hidden rounded-md bg-[var(--color-outline-variant)]" aria-label={t("contextLayout.previewDetails")}>
        {segments.map(segment => (
          <button key={segment.id} type="button" className="min-w-0 shrink-0 overflow-hidden text-xs text-[var(--color-on-surface)] outline-none transition-opacity focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--color-focus-ring)]"
            style={{ width: `${segment.share / 100}%`, background: segment.color, opacity: highlightedId && highlightedId !== segment.id ? 0.35 : 1, boxShadow: "inset -1px 0 0 var(--color-elevated-surface)" }}
            title={label(segment)} aria-label={label(segment)}
            onMouseEnter={() => onHighlightChange(segment.id)} onMouseLeave={() => onHighlightChange(null)}
            onFocus={() => onHighlightChange(segment.id)} onBlur={() => onHighlightChange(null)}>
            {segment.share >= 700 ? <span className="mx-auto inline-block max-w-full truncate rounded bg-[var(--color-elevated-surface)]/90 px-2 py-0.5 text-[var(--color-on-surface)]">{segment.name}</span> : null}
          </button>
        ))}
      </div>

    </section>
  );
}

import { useTranslation } from "react-i18next";
import { contextBlockColor } from "./contextBlockColors";
import { InfoTooltip } from "@/components/info-tooltip";
import { ContextWindowInput } from "./ContextWindowInput";

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
  const number = new Intl.NumberFormat(i18n.language, { notation: "compact", maximumFractionDigits: 1 });
  const percentage = (tokens: number) => `${Number((tokens / windowTokens * 100).toFixed(2))}%`;
  const segments = budgets.map(budget => ({
    ...budget,
    color: contextBlockColor(budget.id),
  })).filter(budget => budget.tokens > 0);
  const totalShare = budgets.reduce((sum, budget) => sum + budget.share, 0);
  const validShares = budgets.every(budget => Number.isSafeInteger(budget.share) && budget.share >= 1 && budget.share <= 10_000);
  const validationMessage = totalShare !== 10_000
    ? t(totalShare > 10_000 ? "contextLayout.shareExcess" : "contextLayout.shareMissing", { value: Math.abs(totalShare - 10_000) / 100 })
    : !validShares ? t("contextLayout.positiveShareRequired") : null;
  const active = segments.find(segment => segment.id === highlightedId);
  const label = (segment: typeof segments[number]) => `${segment.name} · ${number.format(segment.tokens)} tokens · ${percentage(segment.tokens)}`;

  return (
    <section aria-label={t("contextLayout.previewTitle")} className="mb-5 min-w-0 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-4">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-sm font-medium">
          {t("contextLayout.previewTitle")}
          <InfoTooltip ariaLabel={t("contextLayout.previewTitle")}>{t("contextLayout.previewHelp")}</InfoTooltip>
        </div>
        <div className="flex items-center gap-2 text-xs text-[var(--color-on-surface-variant)]">
          <span>{t("contextLayout.previewWindow")}</span>
          <ContextWindowInput value={windowTokens} onChange={onWindowChange} />
          <span>tokens</span>
        </div>
      </div>
      <div role="status" className="mb-2 flex min-h-5 flex-wrap items-center gap-x-3 gap-y-1 text-xs tabular-nums text-[var(--color-on-surface-variant)]">
        <span className={validationMessage ? "text-[var(--color-error)]" : ""}>{t("contextLayout.totalShare", { value: totalShare / 100 })}</span>
        <span>{t("contextLayout.previewBlockCount", { count: budgets.length })}</span>
        {validationMessage ? <span className="text-[var(--color-error)]">{validationMessage}</span> : null}
        {active ? <span className="truncate">{label(active)}</span> : null}
      </div>
      <div className="flex h-12 overflow-hidden rounded-md bg-[var(--color-outline-variant)]" aria-label={t("contextLayout.previewDetails")}>
        {segments.map(segment => (
          <button key={segment.id} type="button" className="min-w-0 shrink-0 overflow-hidden text-xs text-[var(--color-on-surface)] outline-none transition-opacity focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--color-focus-ring)]"
            style={{ width: `${segment.share / 100}%`, background: segment.color, opacity: highlightedId && highlightedId !== segment.id ? 0.35 : 1, boxShadow: "inset -1px 0 0 var(--color-elevated-surface)" }}
            title={label(segment)} aria-label={label(segment)}
            onMouseEnter={() => onHighlightChange(segment.id)} onMouseLeave={() => onHighlightChange(null)}
            onFocus={() => onHighlightChange(segment.id)} onBlur={() => onHighlightChange(null)}>
            {segment.share >= 700 ? <span className="block truncate px-3 text-xs font-medium text-[var(--color-inverse-on-surface)]">{segment.name}</span> : null}
          </button>
        ))}
      </div>

    </section>
  );
}

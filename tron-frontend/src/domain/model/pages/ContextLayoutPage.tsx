import { contextBlockName } from "@/shared/context/contextBlockPresentation";
import { useEffect, useMemo, useState } from "react";
import { GripVertical, RotateCcw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { InfoTooltip } from "@/components/info-tooltip";
import { fetchContextLayout, updateContextLayout, type ContextLayoutBudget, type ContextLayoutState } from "../api/contextLayoutApi";

const HISTORY_ANCHOR = "current_session_history";

export function ContextLayoutPage() {
  const { t } = useTranslation("pages");
  const [layout, setLayout] = useState<ContextLayoutState | null>(null);
  const [order, setOrder] = useState<string[]>([]);
  const [pluginOverrides, setPluginOverrides] = useState<Record<string, number>>({});
  const [nativeBudgets, setNativeBudgets] = useState<Record<string, ContextLayoutBudget>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [draggedId, setDraggedId] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError("");
    setNotice("");
    fetchContextLayout(controller.signal)
      .then((result) => {
        if (controller.signal.aborted) return;
        setLayout(result);
        setOrder(result.order);
        setPluginOverrides(result.pluginOverrides);
        setNativeBudgets(result.nativeBudgets ?? {});
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("contextLayout.loadFailed"));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [refreshKey, t]);

  const blocksById = useMemo(() => new Map((layout?.blocks ?? []).map((block) => [block.id, block])), [layout]);
  const changed = layout !== null && (
    JSON.stringify(order) !== JSON.stringify(layout.order)
    || JSON.stringify(pluginOverrides) !== JSON.stringify(layout.pluginOverrides)
    || JSON.stringify(nativeBudgets) !== JSON.stringify(layout.nativeBudgets ?? {})
  );
  const historyIndex = order.indexOf(HISTORY_ANCHOR);

  function moveBlock(sourceId: string, targetId: string) {
    const sourceIndex = order.indexOf(sourceId);
    const targetIndex = order.indexOf(targetId);
    if (historyIndex < 0 || sourceIndex <= historyIndex || targetIndex <= historyIndex || sourceIndex < 0 || targetIndex < 0) return;
    setOrder((current) => {
      const next = [...current];
      const from = next.indexOf(sourceId);
      const to = next.indexOf(targetId);
      if (from <= historyIndex || to <= historyIndex || from < 0 || to < 0) return current;
      next.splice(from, 1);
      next.splice(to, 0, sourceId);
      return next;
    });
  }

  function moveBy(id: string, offset: -1 | 1) {
    const index = order.indexOf(id);
    const target = order[index + offset];
    if (target) moveBlock(id, target);
  }

  function setTokenOverride(id: string, value: string) {
    setPluginOverrides((current) => {
      const next = { ...current };
      const parsed = value.trim() === "" ? null : Number(value);
      if (parsed === null || !Number.isSafeInteger(parsed) || parsed < 1) delete next[id];
      else next[id] = parsed;
      return next;
    });
  }

  function setNativeBudget(id: string, field: keyof ContextLayoutBudget, value: string) {
    const parsed = value.trim() === "" ? null : field === "maxRatioBps" ? Math.round(Number(value) * 100) : Number(value);
    if (parsed !== null && (!Number.isSafeInteger(parsed) || parsed < 1)) return;
    setNativeBudgets(current => ({ ...current, [id]: { ...current[id], [field]: parsed } }));
  }

  function removeUnavailableBlock(id: string) {
    setOrder((current) => current.filter((blockId) => blockId !== id));
    setPluginOverrides((current) => {
      const next = { ...current };
      delete next[id];
      return next;
    });
  }

  async function save() {
    if (!layout || saving || !changed) return;
    setSaving(true);
    setError("");
    setNotice("");
    try {
      const updated = await updateContextLayout({ revision: layout.revision, order, pluginOverrides, nativeBudgets });
      setLayout(updated);
      setOrder(updated.order);
      setPluginOverrides(updated.pluginOverrides);
      setNativeBudgets(updated.nativeBudgets ?? {});
      setNotice(t("contextLayout.saved"));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : t("contextLayout.saveFailed"));
    } finally {
      setSaving(false);
    }
  }

  const columns = "grid grid-cols-[minmax(0,1fr)_6rem_4rem] items-center gap-2 sm:grid-cols-[minmax(0,1fr)_8rem_6rem] sm:gap-5";
  const inputClass = "h-8 rounded-md border-transparent bg-transparent px-2 text-right font-mono text-sm tabular-nums shadow-none transition-colors hover:border-[var(--color-outline-variant)] hover:bg-[var(--color-elevated-surface)] focus:border-[var(--color-focus-ring)] focus:bg-[var(--color-elevated-surface)] [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none";

  return (
    <div className="mx-auto min-w-0 max-w-5xl pb-8">
      <header className="mb-7 flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-1.5">
          <h1 className="text-xl font-semibold tracking-tight">{t("contextLayout.title")}</h1>
          <p className="text-sm text-[var(--color-on-surface-variant)]">{t("contextLayout.subtitle")}</p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button variant="ghost" disabled={loading || saving} onClick={() => setRefreshKey(value => value + 1)}>
            <RotateCcw aria-hidden="true" className="size-3.5" />{t("contextLayout.refresh")}
          </Button>
          <Button disabled={loading || saving || !changed} onClick={() => void save()}>
            {saving ? t("contextLayout.saving") : t("contextLayout.save")}
          </Button>
        </div>
      </header>

      {error ? <p role="alert" className="mb-4 text-sm text-[var(--color-error)]">{error}</p> : null}
      {notice ? <p role="status" className="mb-4 text-sm text-[var(--color-success)]">{notice}</p> : null}
      {loading ? <p role="status" className="py-10 text-sm text-[var(--color-on-surface-variant)]">{t("contextLayout.loading")}</p> : null}
      {!loading && layout && order.length === 0 ? <p className="py-10 text-sm text-[var(--color-on-surface-variant)]">{t("contextLayout.empty")}</p> : null}

      {!loading && order.length > 0 ? (
        <section aria-label={t("contextLayout.title")}>
          <div className={`${columns} border-b border-[var(--color-outline-variant)] px-3 pb-2 text-xs text-[var(--color-on-surface-variant)] sm:px-4`}>
            <span className="pl-6">{t("contextLayout.blockColumn")}</span>
            <span className="flex items-center justify-end gap-1">
              {t("contextLayout.tokensColumn")}
              <InfoTooltip ariaLabel={t("contextLayout.tokenLimit")}>{t("contextLayout.nativeBudgetHelp")}</InfoTooltip>
            </span>
            <span className="flex items-center justify-end gap-1">
              {t("contextLayout.shareColumn")}
              <InfoTooltip ariaLabel={t("contextLayout.ratioLimit")}>{t("contextLayout.ratioLimitHelp")}</InfoTooltip>
            </span>
          </div>
          <ol className="divide-y divide-[var(--color-outline-variant)]" aria-label={t("contextLayout.title")}>
            {order.map(id => {
              const block = blocksById.get(id);
              const movable = historyIndex >= 0 && order.indexOf(id) > historyIndex;
              const name = contextBlockName(id, t, block?.displayName || id);
              const native = nativeBudgets[id];
              const hasRatio = layout?.nativeBudgets?.[id]?.maxRatioBps != null;
              return (
                <li key={id} draggable={movable && !saving} tabIndex={movable ? 0 : undefined}
                  onKeyDown={event => {
                    if (!movable || saving || event.target !== event.currentTarget || !event.altKey) return;
                    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
                      event.preventDefault(); moveBy(id, event.key === "ArrowUp" ? -1 : 1);
                    }
                  }}
                  onDragStart={event => {
                    if (!movable || saving) { event.preventDefault(); return; }
                    event.dataTransfer.effectAllowed = "move";
                    event.dataTransfer.setData("text/plain", id); setDraggedId(id);
                  }}
                  onDragEnd={() => setDraggedId(null)}
                  onDragOver={event => { if (movable && draggedId && draggedId !== id) event.preventDefault(); }}
                  onDrop={event => {
                    event.preventDefault();
                    const sourceId = event.dataTransfer.getData("text/plain") || draggedId;
                    if (sourceId) moveBlock(sourceId, id);
                    setDraggedId(null);
                  }}
                  className={`group ${columns} min-h-14 px-3 py-2 transition-colors hover:bg-[var(--color-hover-surface)] focus-visible:outline-2 focus-visible:outline-[var(--color-focus-ring)] sm:px-4 ${draggedId === id ? "bg-[var(--color-surface-container)] opacity-50" : ""}`}
                >
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="flex w-4 shrink-0 items-center justify-center">
                      {movable ? <GripVertical size={14} className="cursor-grab text-[var(--color-on-surface-variant)] opacity-40 transition-opacity group-hover:opacity-90 active:cursor-grabbing" aria-hidden="true" /> : null}
                    </span>
                    <span className="truncate text-sm font-medium" title={name}>{name}</span>
                    {block?.plugin ? <span className="shrink-0 rounded px-1.5 py-0.5 text-[10px] text-[var(--color-on-surface-variant)] ring-1 ring-inset ring-[var(--color-outline-variant)]" title={block.providerId ?? undefined}>{t("contextLayout.plugin")}</span> : null}
                    {block?.unavailable ? <Button variant="ghost" size="icon-xs" disabled={saving} aria-label={t("contextLayout.removeUnavailable", { name })} onClick={() => removeUnavailableBlock(id)}><Trash2 /></Button> : null}
                  </div>

                  {native ? (
                    <>
                      <TextInput type="number" min={1} step={1} disabled={saving} value={native.maxTokens ?? ""}
                        className={inputClass} placeholder={t("contextLayout.unlimited")}
                        aria-label={t("contextLayout.tokenLimitFor", { name })}
                        onChange={event => setNativeBudget(id, "maxTokens", event.currentTarget.value)} />
                      {hasRatio ? <div className="relative">
                        <TextInput type="number" min={0.01} max={100} step={0.01} disabled={saving}
                          value={native.maxRatioBps == null ? "" : native.maxRatioBps / 100}
                          className={`${inputClass} pr-6`} aria-label={t("contextLayout.ratioLimitFor", { name })}
                          onChange={event => setNativeBudget(id, "maxRatioBps", event.currentTarget.value)} />
                        <span className="pointer-events-none absolute inset-y-0 right-2 flex items-center text-xs text-[var(--color-on-surface-variant)]">%</span>
                      </div> : <span className="pr-3 text-right text-xs text-[var(--color-on-surface-variant)] opacity-40">—</span>}
                    </>
                  ) : block?.plugin && (!block.unavailable || block.overrideMaxTokens != null) ? (
                    <>
                      <TextInput type="number" min={1} step={1} disabled={saving} value={pluginOverrides[id] ?? ""}
                        className={inputClass} placeholder={block.defaultMaxTokens?.toLocaleString() ?? t("contextLayout.inheritBudget")}
                        aria-label={t("contextLayout.tokenOverrideFor", { name })}
                        title={t("contextLayout.tokenOverrideHelp")}
                        onChange={event => setTokenOverride(id, event.currentTarget.value)} />
                      <span className="pr-3 text-right text-xs text-[var(--color-on-surface-variant)] opacity-40">—</span>
                    </>
                  ) : <div className="col-span-2 flex items-center justify-end gap-1 text-xs text-[var(--color-on-surface-variant)]">
                    <span>{t(id === HISTORY_ANCHOR ? "contextLayout.remaining" : block?.unavailable ? "contextLayout.unavailable" : "contextLayout.retained")}</span>
                    <InfoTooltip ariaLabel={name}>{t(id === HISTORY_ANCHOR ? "contextLayout.historyBudgetHelp" : block?.unavailable ? "contextLayout.unavailable" : "contextLayout.instructionBudgetHelp")}</InfoTooltip>
                  </div>}
                </li>
              );
            })}
          </ol>
          <footer className="mt-3 flex items-center gap-1 border-t border-[var(--color-outline-variant)] pt-3 text-xs text-[var(--color-on-surface-variant)]">
            <InfoTooltip ariaLabel={t("contextLayout.orderHelpLabel")}>{t("contextLayout.orderHelp")} {t("contextLayout.dragHelp")}</InfoTooltip>
            <span>{t("contextLayout.compactHelp")}</span>
          </footer>
        </section>
      ) : null}
    </div>
  );
}

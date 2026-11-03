import { useEffect, useMemo, useState } from "react";
import { GripVertical, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Button } from "@/components/ui/button";
import { InfoTooltip } from "@/components/info-tooltip";
import { PageHeader } from "@/shared";
import { contextBlockName } from "@/shared/context/contextBlockPresentation";
import { ContextBlockHelp } from "@/shared/context/ContextBlockHelp";
import { fetchContextLayout, updateContextLayout, type ContextLayoutState } from "../api/contextLayoutApi";
import { contextBlockColor } from "../components/context-layout/contextBlockColors";
import { BudgetShareControl } from "../components/context-layout/BudgetShareControl";
import { allocateContextShares } from "../components/context-layout/contextShares";
import { ContextBudgetPreview } from "../components/context-layout/ContextBudgetPreview";

export function ContextLayoutPage() {
  const { t, i18n } = useTranslation("pages");
  const [layout, setLayout] = useState<ContextLayoutState | null>(null);
  const [order, setOrder] = useState<string[]>([]);
  const [shares, setShares] = useState<Record<string, number>>({});
  const [previewWindow, setPreviewWindow] = useState(1_000_000);
  const [resetOpen, setResetOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [highlightedId, setHighlightedId] = useState<string | null>(null);
  const [draggedId, setDraggedId] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true); setError(""); setNotice("");
    fetchContextLayout(controller.signal).then(result => {
      if (controller.signal.aborted) return;
      setLayout(result); setOrder(result.order); setShares(result.shares ?? {});
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("contextLayout.loadFailed"));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [refreshKey, t]);

  const blocksById = useMemo(() => new Map((layout?.blocks ?? []).map(block => [block.id, block])), [layout]);
  const changed = layout !== null && (JSON.stringify(order) !== JSON.stringify(layout.order) || JSON.stringify(shares) !== JSON.stringify(layout.shares));
  const canReset = layout?.defaults != null && (
    JSON.stringify(order) !== JSON.stringify(layout.defaults.order)
    || JSON.stringify(shares) !== JSON.stringify(layout.defaults.shares)
  );
  const total = Object.values(shares).reduce((sum, value) => sum + value, 0);
  const valid = total <= 10_000 && order.every(id => Number.isSafeInteger(shares[id]) && shares[id] >= 1 && shares[id] <= 10_000) && Object.keys(shares).length === order.length;
  const number = new Intl.NumberFormat(i18n.language);

  function moveBlock(sourceId: string, targetId: string) {
    setOrder(current => {
      const from = current.indexOf(sourceId); const to = current.indexOf(targetId);
      if (from <= 0 || to <= 0) return current;
      const next = [...current]; next.splice(from, 1); next.splice(to, 0, sourceId); return next;
    });
  }
  function removeBlock(id: string) {
    setOrder(current => current.filter(value => value !== id));
    setShares(current => { const next = { ...current }; delete next[id]; return next; });
  }
  function reset() {
    if (!layout?.defaults || saving) return;
    setOrder([...layout.defaults.order]);
    setShares({ ...layout.defaults.shares });
    setDraggedId(null);
    setError("");
    setNotice(t("contextLayout.resetDraft"));
  }
  async function save() {
    if (!layout || saving || !changed) return;
    if (!valid) { setError(t("contextLayout.totalRequired")); return; }
    setSaving(true); setError(""); setNotice("");
    try {
      const updated = await updateContextLayout({ revision: layout.revision, order, shares });
      setLayout(updated); setOrder(updated.order); setShares(updated.shares);
    } catch (reason) { setError(reason instanceof Error ? reason.message : t("contextLayout.saveFailed")); }
    finally { setSaving(false); }
  }
  const columns = "grid grid-cols-[minmax(8rem,1fr)_4rem_10rem_6rem] items-center gap-3 sm:grid-cols-[minmax(0,1fr)_4.5rem_16rem_9rem] sm:gap-5";
  const tokenPreview = allocateContextShares(shares, previewWindow);
  const budgets = order.map(id => ({ id, name: blocksById.get(id)?.displayName || contextBlockName(id, t), share: shares[id] ?? 0, tokens: tokenPreview[id] ?? 0 }));

  return (
    <div className="min-w-0 pb-8">
      <PageHeader title={t("contextLayout.title")} actions={<>
        <Button disabled={loading || saving || !changed || !valid} onClick={() => void save()}>{saving ? t("contextLayout.saving") : t("contextLayout.save")}</Button>
        <Button variant="secondary" disabled={loading || saving || !canReset} onClick={() => setResetOpen(true)}>{t("contextLayout.reset")}</Button>
        <Button variant="secondary" disabled={loading || saving} onClick={() => setRefreshKey(value => value + 1)}>{t("contextLayout.refresh")}</Button>
      </>} />
      {error ? <p role="alert" className="mb-4 text-sm text-[var(--color-error)]">{error}</p> : null}
      {notice ? <p role="status" className="mb-4 text-sm text-[var(--color-success)]">{notice}</p> : null}
      {loading ? <p role="status" className="py-10 text-sm text-[var(--color-on-surface-variant)]">{t("contextLayout.loading")}</p> : null}
      {!loading && order.length > 0 ? <>
        <ContextBudgetPreview budgets={budgets} windowTokens={previewWindow} onWindowChange={setPreviewWindow} highlightedId={highlightedId} onHighlightChange={setHighlightedId} />
        <div className="overflow-x-auto">
          <section aria-label={t("contextLayout.title")} className="min-w-[30rem] overflow-hidden rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)]">
            <div className={`${columns} border-b border-[var(--color-outline-variant)] px-4 py-3 text-xs text-[var(--color-on-surface-variant)]`}>
              <span className="pl-6">{t("contextLayout.blockColumn")}</span>
              <span className="text-center">{t("contextLayout.updateMode")}</span>
              <span className="flex items-center justify-end gap-1">{t("contextLayout.allocationColumn")} <InfoTooltip ariaLabel={t("contextLayout.allocationColumn")}>{t("contextLayout.allocationHelp")}</InfoTooltip></span>
              <span className="text-right">{t("contextLayout.estimatedTokens")}</span>
            </div>
            <ol className="divide-y divide-[var(--color-outline-variant)]" aria-label={t("contextLayout.title")}>
              {order.map((id, index) => {
                const block = blocksById.get(id);
                const movable = id !== "seed_prompt_prefix";
                const name = block?.displayName || contextBlockName(id, t);
                return <li key={id}
                  onMouseEnter={() => setHighlightedId(id)} onMouseLeave={() => setHighlightedId(null)}
                  onFocusCapture={() => setHighlightedId(id)}
                  onBlurCapture={event => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setHighlightedId(null); }}
                  onDragOver={event => {
                    if (!movable || saving || !draggedId || draggedId === id) return;
                    event.preventDefault(); event.dataTransfer.dropEffect = "move";
                  }}
                  onDrop={event => {
                    if (!movable || saving || !draggedId) return;
                    event.preventDefault(); moveBlock(draggedId, id); setDraggedId(null);
                  }}
                  className={`group ${columns} min-h-14 px-4 py-2 hover:bg-[var(--color-hover-surface)] focus-visible:outline-2 focus-visible:outline-[var(--color-focus-ring)] ${draggedId === id ? "opacity-50" : ""}`}>
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="flex w-4 shrink-0 justify-center">{movable ? <Button
                      variant="ghost" size="icon-xs" disabled={saving} draggable={!saving}
                      aria-label={t("contextLayout.reorderBlock", { name })}
                      className="cursor-grab text-[var(--color-on-surface-variant)] opacity-40 group-hover:opacity-90 focus-visible:opacity-100 active:cursor-grabbing"
                      onKeyDown={event => {
                        if (saving || !event.altKey) return;
                        if (event.key === "ArrowUp" || event.key === "ArrowDown") {
                          event.preventDefault(); const target = order[index + (event.key === "ArrowUp" ? -1 : 1)]; if (target) moveBlock(id, target);
                        }
                      }}
                      onDragStart={event => {
                        if (saving) { event.preventDefault(); return; }
                        event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/plain", id); setDraggedId(id);
                      }}
                      onDragEnd={() => setDraggedId(null)}
                    ><GripVertical aria-hidden="true" /></Button> : null}</span>
                    <span aria-hidden="true" className="size-2.5 shrink-0 rounded-[2px]" style={{ background: contextBlockColor(id) }} />
                    <span className="truncate text-sm font-medium" title={name}>{name}</span>
                    <ContextBlockHelp name={name} description={block?.description} updateMode={block?.updateMode} />
                    {block?.plugin ? <span className="shrink-0 rounded px-1.5 py-0.5 text-[10px] text-[var(--color-on-surface-variant)] ring-1 ring-inset ring-[var(--color-outline-variant)]" title={block.providerId ?? undefined}>{t("contextLayout.plugin")}</span> : null}
                    {block?.unavailable ? <Button variant="ghost" size="icon-xs" disabled={saving} aria-label={t("contextLayout.removeUnavailable", { name })} onClick={() => removeBlock(id)}><Trash2 /></Button> : null}
                  </div>
                  <span className="text-center text-xs text-[var(--color-on-surface-variant)]">{t(`contextLayout.updateModes.${block?.updateMode ?? "Unspecified"}`)}</span>
                  <BudgetShareControl value={shares[id] ?? null} disabled={saving} label={t("contextLayout.ratioLimitFor", { name })} placeholder="0"
                    onChange={value => { setShares(current => ({ ...current, [id]: value ?? 0 })); setError(""); setNotice(""); }} />
                  <span className="text-right text-sm tabular-nums text-[var(--color-on-surface-variant)]" title={t("contextLayout.estimatedTokensHelp")}>{number.format(tokenPreview[id] ?? 0)}</span>
                </li>;
              })}
            </ol>
          </section>
        </div>
      </> : null}
      <ConfirmDialog
        open={resetOpen}
        tone="default"
        pending={loading || saving}
        title={t("contextLayout.resetTitle")}
        description={t("contextLayout.resetDescription")}
        cancelText={t("contextLayout.resetCancel")}
        confirmText={t("contextLayout.resetConfirm")}
        onCancel={() => setResetOpen(false)}
        onConfirm={() => { reset(); setResetOpen(false); }}
      />
    </div>
  );
}

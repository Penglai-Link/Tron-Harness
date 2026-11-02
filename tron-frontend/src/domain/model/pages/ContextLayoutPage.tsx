import { contextBlockName } from "@/shared/context/contextBlockPresentation";
import { useEffect, useMemo, useState } from "react";
import { GripVertical, RotateCcw, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Field, TextInput } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { InfoTooltip } from "@/components/info-tooltip";
import { PageHeader, SurfaceCard } from "@/shared";
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

  const pageTitle = t("contextLayout.title");
  return (
    <div className="min-w-0 space-y-5">
      <PageHeader
        title={pageTitle}
        actions={(
          <>
            <Button variant="outline" disabled={loading || saving} onClick={() => setRefreshKey((value) => value + 1)}>
              <RotateCcw aria-hidden="true" />{t("contextLayout.refresh")}
            </Button>
            <Button disabled={loading || saving || !changed} onClick={() => void save()}>
              {saving ? t("contextLayout.saving") : t("contextLayout.save")}
            </Button>
          </>
        )}
      />

      <SurfaceCard className="min-w-0 space-y-4 p-4 sm:p-5">
        <p className="max-w-4xl text-sm text-[var(--color-on-surface-variant)]">{t("contextLayout.description")}</p>
        <p className="flex items-start gap-2 text-xs text-[var(--color-on-surface-variant)]">
          <InfoTooltip ariaLabel={t("contextLayout.orderHelpLabel")}>{t("contextLayout.orderHelp")}</InfoTooltip>
          <span>{t("contextLayout.dragHelp")}</span>
        </p>

        {error ? <p role="alert" className="text-sm text-[var(--color-error)]">{error}</p> : null}
        {notice ? <p role="status" className="text-sm text-[var(--color-success)]">{notice}</p> : null}
        {loading ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{t("contextLayout.loading")}</p> : null}

        {!loading && layout && order.length === 0 ? (
          <p className="text-sm text-[var(--color-on-surface-variant)]">{t("contextLayout.empty")}</p>
        ) : null}

        {!loading && order.length > 0 ? (
          <ol className="min-w-0 divide-y rounded-lg border" aria-label={t("contextLayout.title")}>
            {order.map((id, index) => {
              const block = blocksById.get(id);
              const movable = historyIndex >= 0 && index > historyIndex;
              const name = contextBlockName(id, t, block?.displayName || id);
              const budgetParts = [
                block?.defaultMaxTokens != null
                  ? t("contextLayout.tokenBudget", { count: block.defaultMaxTokens.toLocaleString() })
                  : null,
                block?.defaultMaxRatioBps != null
                  ? t("contextLayout.ratioBudget", { value: (block.defaultMaxRatioBps / 100).toLocaleString(undefined, { maximumFractionDigits: 2 }) })
                  : null,
                block?.defaultMaxMessages != null
                  ? t("contextLayout.messageBudget", { count: block.defaultMaxMessages.toLocaleString() })
                  : null,
              ].filter((part): part is string => part !== null);
              const budget = budgetParts.length > 0 ? budgetParts.join(" · ") : t("contextLayout.noDefaultBudget");

              return (
                <li
                  key={id}
                  draggable={movable && !saving}
                  tabIndex={movable ? 0 : undefined}
                  onKeyDown={(event) => {
                    if (!movable || saving || event.target !== event.currentTarget || !event.altKey) return;
                    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
                      event.preventDefault();
                      moveBy(id, event.key === "ArrowUp" ? -1 : 1);
                    }
                  }}
                  onDragStart={(event) => {
                    if (!movable) { event.preventDefault(); return; }
                    event.dataTransfer.effectAllowed = "move";
                    event.dataTransfer.setData("text/plain", id);
                    setDraggedId(id);
                  }}
                  onDragEnd={() => setDraggedId(null)}
                  onDragOver={(event) => {
                    if (movable && draggedId && draggedId !== id) event.preventDefault();
                  }}
                  onDrop={(event) => {
                    event.preventDefault();
                    const sourceId = event.dataTransfer.getData("text/plain") || draggedId;
                    if (sourceId) moveBlock(sourceId, id);
                    setDraggedId(null);
                  }}
                  className={`min-w-0 p-3 sm:p-4 ${draggedId === id ? "opacity-45" : ""}`}
                >
                  <div className="flex min-w-0 flex-col gap-3 sm:flex-row sm:items-start">
                    <div className="flex min-w-0 flex-1 items-start gap-2">
                      {movable ? <span className="mt-0.5 shrink-0 cursor-grab active:cursor-grabbing"><GripVertical size={16} aria-hidden="true" /></span> : null}
                      <div className="min-w-0 flex-1">
                        <div className="flex min-w-0 flex-wrap items-center gap-2">
                          <span className="break-all font-medium">{name}</span>
                          {block?.plugin ? <Badge variant="outline" title={block.providerId ?? undefined}>{t("contextLayout.plugin")}</Badge> : null}
                          {block?.unavailable ? <Badge variant="warning">{t("contextLayout.unavailable")}</Badge> : null}
                        </div>
                        {block?.plugin ? <p className="mt-1 break-all font-mono text-xs text-[var(--color-on-surface-variant)]">{id}</p> : null}
                        {block?.description ? <p className="mt-2 break-words text-sm text-[var(--color-on-surface-variant)]">{block.description}</p> : null}
                        {block?.plugin ? <p className="mt-2 text-xs text-[var(--color-on-surface-variant)]">{t("contextLayout.defaultBudget", { budget })}</p> : null}
                      </div>
                    </div>

                    {nativeBudgets[id] ? (
                      <div className="grid w-full shrink-0 gap-3 sm:w-72">
                        <Field label={t("contextLayout.tokenLimit")} hint={t("contextLayout.nativeBudgetHelp")}>
                          <TextInput type="number" min={1} step={1} disabled={saving} value={nativeBudgets[id].maxTokens ?? ""} placeholder={t("contextLayout.noTokenCap")}
                            aria-label={t("contextLayout.tokenLimitFor", { name })} onChange={event => setNativeBudget(id, "maxTokens", event.currentTarget.value)} />
                        </Field>
                        {layout?.nativeBudgets?.[id]?.maxRatioBps != null ? (
                          <Field label={t("contextLayout.ratioLimit")} hint={t("contextLayout.ratioLimitHelp")}>
                            <TextInput type="number" min={0.01} max={100} step={0.01} disabled={saving} value={nativeBudgets[id].maxRatioBps == null ? "" : nativeBudgets[id].maxRatioBps! / 100}
                              aria-label={t("contextLayout.ratioLimitFor", { name })} onChange={event => setNativeBudget(id, "maxRatioBps", event.currentTarget.value)} />
                          </Field>
                        ) : null}
                      </div>
                    ) : !block?.plugin && (id === "seed_prompt_prefix" || id === HISTORY_ANCHOR) ? <p className="text-xs text-[var(--color-on-surface-variant)] sm:max-w-72">{t(id === HISTORY_ANCHOR ? "contextLayout.historyBudgetHelp" : "contextLayout.instructionBudgetHelp")}</p> : null}
                    {block?.plugin && (!block.unavailable || block.overrideMaxTokens != null) ? (
                      <Field className="w-full shrink-0 sm:w-64" label={t("contextLayout.tokenOverride")} hint={t("contextLayout.tokenOverrideHelp")}>
                        <TextInput
                          type="number"
                          min={1}
                          step={1}
                          inputMode="numeric"
                          disabled={saving}
                          value={pluginOverrides[id] ?? ""}
                          placeholder={t("contextLayout.inheritBudget")}
                          onChange={(event) => setTokenOverride(id, event.currentTarget.value)}
                          aria-label={t("contextLayout.tokenOverrideFor", { name })}
                        />
                      </Field>
                    ) : null}
                    {block?.unavailable ? (
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t("contextLayout.removeUnavailable", { name })}
                        disabled={saving}
                        onClick={() => removeUnavailableBlock(id)}
                      >
                        <Trash2 aria-hidden="true" />
                      </Button>
                    ) : null}
                  </div>
                </li>
              );
            })}
          </ol>
        ) : null}
      </SurfaceCard>
    </div>
  );
}

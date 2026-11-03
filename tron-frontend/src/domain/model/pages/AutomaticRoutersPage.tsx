import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { Combobox, ComboboxInput, ComboboxPopup, ComboboxList, ComboboxEmpty, ComboboxItem } from "@/components/ui/combobox";
import { useToast } from "@/components/ui/toast";
import { PageHeader } from "@/shared";
import { fetchAutomaticRouters, updateAutomaticRouter, type AutomaticRouter, type RouteSources, type RouteSourceMode } from "../api/automaticRouterApi";

export function AutomaticRoutersPage() {
  const { t } = useTranslation("pages");
  const [routes, setRoutes] = useState<AutomaticRouter[]>([]);
  const [revision, setRevision] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true); setError("");
    fetchAutomaticRouters(controller.signal).then(records => { if (!controller.signal.aborted) setRoutes(records); })
      .catch((reason: unknown) => { if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("automaticRouters.failed")); })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [revision, t]);
  return <div className="space-y-5">
    <PageHeader title={t("automaticRouters.title")} actions={<Button variant="outline" disabled={loading} onClick={() => setRevision(v => v + 1)}>{t("automaticRouters.refresh")}</Button>} />
    {error && <p role="alert" className="text-[var(--color-error)]">{error}</p>}
    {loading ? <p role="status">{t("automaticRouters.loading")}</p> : <div className="max-w-5xl divide-y rounded-lg border bg-[var(--color-elevated-surface)]">
      {routes.map(route => <RouterEditor key={route.kind} route={route} />)}
    </div>}
  </div>;
}

function RouterEditor({ route }: { route: AutomaticRouter }) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [sources, setSources] = useState<RouteSources>(route.sources);
  const [saved, setSaved] = useState(route.sources);
  const [saving, setSaving] = useState(false);
  useEffect(() => { setSources(route.sources); setSaved(route.sources); }, [route.sources]);
  const providers = [...new Map(route.candidates.map(m => [m.providerId, { id: m.providerId, label: m.providerName }])).values()];
  const items = sources.mode === "SelectedProviders" ? providers : [...new Set(route.candidates.map(m => m.modelCode))].map(code => { const group = route.candidates.filter(m => m.modelCode === code); return { id: code, label: `${group[0].modelName} · ${group.map(m => m.providerName).join(" / ")}` }; });
  const selectedIds = sources.mode === "SelectedProviders" ? sources.providerIds : sources.modelCodes;
  // Retain unavailable identities visibly so an edit never silently widens access.
  for (const id of selectedIds) if (!items.some(item => item.id === id)) items.push({ id, label: t("automaticRouters.unavailable", { id }) });
  const selected = items.filter(item => selectedIds.includes(item.id));
  const effective = route.candidates.filter(m => sources.mode === "AllProviders" || (sources.mode === "SelectedProviders" ? sources.providerIds.includes(m.providerId) : sources.modelCodes.includes(m.modelCode)));
  async function save() {
    if (saving) return;
    setSaving(true);
    try { await updateAutomaticRouter(route.kind, sources); setSaved(sources); toast.success(t("automaticRouters.saved")); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("automaticRouters.failed")); }
    finally { setSaving(false); }
  }
  return <section className="space-y-4 p-5" aria-label={t(`automaticRouters.kinds.${route.kind}`)}>
    <div className="flex items-center justify-between gap-4">
      <div className="flex items-center gap-2"><h2 className="font-semibold">{t(`automaticRouters.kinds.${route.kind}`)}</h2><InfoTooltip ariaLabel={`${t(`automaticRouters.kinds.${route.kind}`)} ${t("automaticRouters.source")}`}>{t(`automaticRouters.routeHelp.${route.kind}`)}</InfoTooltip></div>
      <Button disabled={saving || JSON.stringify(saved) === JSON.stringify(sources)} onClick={() => void save()}>{t("automaticRouters.save")}</Button>
    </div>
    <div className="grid gap-4 md:grid-cols-[14rem_1fr]">
      <Field label={t("automaticRouters.source")}>
        <SelectInput className="h-8 min-h-8 sm:min-h-8" value={sources.mode} disabled={saving} onValueChange={mode => { if (mode) setSources({ mode: mode as RouteSourceMode, providerIds: [], modelCodes: [] }); }}>
          {(["AllProviders", "SelectedProviders", "SelectedModels"] as const).map(mode => <SelectItem key={mode} value={mode}>{t(`automaticRouters.modes.${mode}`)}</SelectItem>)}
        </SelectInput>
      </Field>
      {sources.mode !== "AllProviders" && <Field label={t(sources.mode === "SelectedProviders" ? "automaticRouters.providers" : "automaticRouters.models")}>
        <Combobox multiple items={items} value={selected} disabled={saving} itemToStringLabel={item => item.label} itemToStringValue={item => item.id}
          isItemEqualToValue={(item, value) => item.id === value.id}
          onValueChange={(values, details) => { if (details.reason === "escape-key") { details.cancel(); return; } setSources({ ...sources, providerIds: sources.mode === "SelectedProviders" ? values.map(v => v.id) : [], modelCodes: sources.mode === "SelectedModels" ? values.map(v => v.id) : [] }); }}>
          <ComboboxInput className="h-8" aria-label={t(sources.mode === "SelectedProviders" ? "automaticRouters.providers" : "automaticRouters.models")} placeholder={t("automaticRouters.choose")} />
          <ComboboxPopup className="w-(--anchor-width)"><ComboboxEmpty>{t("automaticRouters.empty")}</ComboboxEmpty><ComboboxList>{(item: { id: string; label: string }) => <ComboboxItem key={item.id} value={item}>{item.label}</ComboboxItem>}</ComboboxList></ComboboxPopup>
        </Combobox>
        {sources.mode === "SelectedProviders" && <div className="mt-2 flex flex-wrap gap-2">{selected.map(item => <span key={item.id} className="rounded border px-2 py-1 text-xs">{item.label}</span>)}</div>}
      </Field>}
    </div>
    <details className="text-sm"><summary className="cursor-pointer text-[var(--color-on-surface-variant)]">{t("automaticRouters.candidates", { count: effective.length })}</summary>
      <ul className="mt-2 space-y-1">{effective.map(m => <li key={`${m.providerId}:${m.bindingId}`} className="flex flex-wrap items-baseline gap-x-2 break-words"><span>{m.providerName} / {m.modelName}</span><span className="text-xs text-[var(--color-on-surface-variant)]">{m.modelCode} · {m.inputModalities?.map(input => t(`configuredProviders.inputs.${input}`)).join(" / ")}{m.dimension ? ` · ${m.dimension}D` : ""}</span></li>)}</ul>
      {sources.mode === "SelectedModels" && selected.filter(item => !effective.some(m => m.modelCode === item.id)).map(item => <p key={item.id} className="mt-1 break-words">{item.label}</p>)}
      {!effective.length && (sources.mode !== "SelectedModels" || !selected.length) && <p className="mt-2">{t("automaticRouters.empty")}</p>}
    </details>
  </section>;
}

import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Combobox, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, ComboboxPopup } from "@/components/ui/combobox";
import { fetchProviderPresetModels, type ConfiguredProviderProtocol } from "@/domain/model/api/configuredProviderApi";
import { draftRowId, presetModelToDraft, type ProviderModelDraft } from "./configuredProviderForm";

export function ProviderModelPicker({ presetId, models, protocol, disabled, onChange }: {
  presetId?: string | null;
  models: ProviderModelDraft[];
  protocol: ConfiguredProviderProtocol;
  disabled: boolean;
  onChange: (models: ProviderModelDraft[]) => void;
}) {
  const { t } = useTranslation("pages");
  const [localCatalog] = useState(() => models.filter(model => model.modelId.trim()));
  const [catalog, setCatalog] = useState<ProviderModelDraft[]>(localCatalog);
  const [loading, setLoading] = useState(Boolean(presetId));
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    if (!presetId) { setCatalog(localCatalog); setLoading(false); return; }
    const controller = new AbortController();
    setCatalog([]);
    setLoading(true);
    setError("");
    const request = fetchProviderPresetModels(presetId, controller.signal).then(items => items.map(item => presetModelToDraft(item, protocol)));
    request.then((items) => {
      if (!controller.signal.aborted) setCatalog([...new Map(items.map(item => [item.modelId, item])).values()]);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("configuredProviders.errors.catalog"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [presetId, localCatalog, protocol, revision, t]);
  const selected = useMemo(() => catalog.filter(item => models.some(model => model.modelId === item.modelId)), [catalog, models]);
  return <Field label={t("configuredProviders.catalog.selectModels")}>
    <Combobox multiple items={catalog} value={selected} disabled={disabled || loading}
      itemToStringLabel={item => item.name} itemToStringValue={item => item.modelId}
      isItemEqualToValue={(item, value) => item.modelId === value.modelId}
      filter={(item, query) => `${item.name} ${item.modelId}`.toLowerCase().includes(query.trim().toLowerCase())}
      onValueChange={(values, details) => {
        // Escape closes search; it must not discard the configured model list.
        if (details.reason === "escape-key") { details.cancel(); return; }
        const nextIds = new Set(values.map(item => item.modelId));
        const removed = new Set(selected.filter(item => !nextIds.has(item.modelId)).map(item => item.modelId));
        const next = models.filter(model => !removed.has(model.modelId));
        for (const item of values) {
          if (!next.some(model => model.modelId === item.modelId)) next.push({ ...item, rowId: draftRowId(), modelCode: null });
        }
        onChange(next);
      }}>
      <ComboboxInput aria-label={t("configuredProviders.catalog.selectModels")} autoComplete="off"
        placeholder={t(loading ? "configuredProviders.loading" : "configuredProviders.catalog.searchOrChooseModels")} />
      <ComboboxPopup className="w-(--anchor-width)">
        <ComboboxEmpty>{t("configuredProviders.catalog.noModels")}</ComboboxEmpty>
        <ComboboxList>{(item: ProviderModelDraft) => <ComboboxItem key={item.modelId} value={item}>
          <div className="min-w-0"><div className="truncate">{item.name}</div>
            <div className="truncate font-mono text-xs text-[var(--color-on-surface-variant)]">{item.modelId}</div></div>
        </ComboboxItem>}</ComboboxList>
      </ComboboxPopup>
    </Combobox>
    {error && <div role="alert" className="mt-2 flex items-center gap-3 text-sm text-[var(--color-error)]">
      <span>{error}</span><Button type="button" variant="secondary" disabled={disabled} onClick={() => setRevision(value => value + 1)}>{t("configuredProviders.actions.retry")}</Button>
    </div>}
  </Field>;
}

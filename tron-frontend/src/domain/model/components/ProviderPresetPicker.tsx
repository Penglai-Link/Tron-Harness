import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import {
  fetchProviderPresetModels, fetchProviderPresets,
  type ProviderPreset, type ProviderPresetModel,
} from "@/domain/model/api/configuredProviderApi";

export function ProviderPresetPicker({ selectedModelIds, disabled, onSelect, onAddModel }: {
  selectedModelIds: string[];
  disabled: boolean;
  onSelect: (preset: ProviderPreset) => void;
  onAddModel: (model: ProviderPresetModel) => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [models, setModels] = useState<ProviderPresetModel[]>([]);
  const [providerSearch, setProviderSearch] = useState("");
  const [modelSearch, setModelSearch] = useState("");
  const [visibleLimit, setVisibleLimit] = useState(20);
  const [loading, setLoading] = useState(true);
  const [loadingModels, setLoadingModels] = useState(false);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError("");
    fetchProviderPresets(controller.signal).then((records) => {
      if (!controller.signal.aborted) setPresets(records);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("configuredProviders.errors.catalog"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false);
    });
    return () => controller.abort();
  }, [revision, t]);

  useEffect(() => {
    if (!selectedId) return;
    const controller = new AbortController();
    setLoadingModels(true);
    setModels([]);
    setError("");
    fetchProviderPresetModels(selectedId, controller.signal).then((records) => {
      if (!controller.signal.aborted) setModels(records);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("configuredProviders.errors.catalog"));
    }).finally(() => {
      if (!controller.signal.aborted) setLoadingModels(false);
    });
    return () => controller.abort();
  }, [selectedId, revision, t]);

  const selected = presets.find((preset) => preset.id === selectedId);
  const filteredPresets = useMemo(() => {
    const query = providerSearch.trim().toLowerCase();
    return presets.filter((preset) => preset.id === selectedId || `${preset.id} ${preset.name}`.toLowerCase().includes(query));
  }, [presets, providerSearch, selectedId]);
  const filteredModels = useMemo(() => {
    const query = modelSearch.trim().toLowerCase();
    return models.filter((model) => `${model.modelId} ${model.name}`.toLowerCase().includes(query));
  }, [models, modelSearch]);

  return (
    <section className="space-y-4 border-b border-[var(--color-outline-variant)] pb-6" aria-labelledby="provider-presets-heading">
      <div>
        <h2 id="provider-presets-heading" className="text-base font-semibold">{text("catalog.title")}</h2>
        <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{text("catalog.hint")}</p>
      </div>
      {loading ? <p className="inline-flex items-center gap-2 text-sm" role="status"><Spinner size={16} />{text("loading")}</p> : (
        <div className="grid gap-4 md:grid-cols-2">
          <Field label={text("catalog.searchProviders")}>
            <TextInput aria-label={text("catalog.searchProviders")} value={providerSearch} disabled={disabled}
              onChange={(event) => setProviderSearch(event.currentTarget.value)} />
          </Field>
          <Field label={text("catalog.provider")}>
            <SelectInput aria-label={text("catalog.provider")} value={selectedId || null} disabled={disabled}
              placeholder={text("catalog.chooseProvider")}
              onValueChange={(value: string | null) => {
                const preset = presets.find((entry) => entry.id === value);
                if (!preset?.supported || !preset.protocol) return;
                setSelectedId(preset.id);
                setModelSearch("");
                setVisibleLimit(20);
                onSelect(preset);
              }}>
              {filteredPresets.map((preset) => <SelectItem key={preset.id} value={preset.id} disabled={!preset.supported || !preset.protocol}>
                {preset.name}{!preset.supported ? ` · ${text("catalog.unsupported")}` : ""}
              </SelectItem>)}
            </SelectInput>
          </Field>
        </div>
      )}
      {error && <div role="alert" className="flex items-center justify-between gap-3 text-sm text-[var(--color-error)]">
        <span>{error}</span><Button type="button" variant="secondary" onClick={() => setRevision((value) => value + 1)}>{text("actions.retry")}</Button>
      </div>}
      {selected && <>
        <div className="flex flex-wrap gap-2">
          {selected.guessed && <Badge variant="warning">{text("catalog.guessed")}</Badge>}
          {(selected.needsBaseUrl || !selected.baseUrl) && <Badge variant="warning">{text("catalog.needsBaseUrl")}</Badge>}
          <span className="text-sm text-[var(--color-on-surface-variant)]">{t("configuredProviders.catalog.modelCount", { count: selected.modelCount })}</span>
        </div>
        {selected.guessed && <p className="text-sm text-[var(--color-on-surface-variant)]">{text("catalog.guessedHint")}</p>}
        <Field label={text("catalog.searchModels")}>
          <TextInput aria-label={text("catalog.searchModels")} value={modelSearch} disabled={disabled}
            onChange={(event) => { setModelSearch(event.currentTarget.value); setVisibleLimit(20); }} />
        </Field>
        {loadingModels ? <p role="status" className="text-sm">{text("loading")}</p> : (
          <div className="max-h-80 overflow-y-auto rounded border border-[var(--color-outline-variant)]">
            {filteredModels.slice(0, visibleLimit).map((model) => {
              const added = selectedModelIds.includes(model.modelId);
              return <div key={model.modelId} className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-3 py-2 last:border-0">
                <div className="min-w-0"><p className="truncate text-sm font-medium" title={model.name}>{model.name}</p>
                  <p className="truncate font-mono text-xs text-[var(--color-on-surface-variant)]" title={model.modelId}>{model.modelId}</p></div>
                <Button type="button" size="sm" variant="secondary" disabled={disabled || added}
                  aria-label={t(added ? "configuredProviders.catalog.addedModel" : "configuredProviders.catalog.addModel", { name: model.name })}
                  onClick={() => onAddModel(model)}>{text(added ? "catalog.added" : "catalog.add")}</Button>
              </div>;
            })}
            {!filteredModels.length && <p className="p-3 text-sm text-[var(--color-on-surface-variant)]">{text("catalog.noModels")}</p>}
            {filteredModels.length > visibleLimit && <div className="p-3"><Button type="button" variant="ghost" onClick={() => setVisibleLimit((value) => value + 20)}>{text("catalog.more")}</Button></div>}
          </div>
        )}
      </>}
    </section>
  );
}

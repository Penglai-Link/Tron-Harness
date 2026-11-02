import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Combobox, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, ComboboxPopup } from "@/components/ui/combobox";
import { Spinner } from "@/components/ui/spinner";
import { InfoTooltip } from "@/components/info-tooltip";
import { fetchModelProviderConnections, type ModelProviderConnection } from "@/domain/model/api";
import {
  fetchConfiguredProvider, fetchProviderPresets,
  type ProviderPreset,
} from "@/domain/model/api/configuredProviderApi";
import { duplicateProviderDraft, type ConfiguredProviderDraft } from "./configuredProviderForm";

interface ProviderOption {
  value: string;
  label: string;
  searchText: string;
  disabled: boolean;
}

export function ProviderPresetPicker({ disabled, onSelect, onCreateFromExisting, onSelectionPending }: {
  disabled: boolean;
  onSelect: (preset: ProviderPreset | null) => void;
  onCreateFromExisting: (draft: ConfiguredProviderDraft) => void;
  onSelectionPending: () => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [existing, setExisting] = useState<ModelProviderConnection[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  const [loadingConnection, setLoadingConnection] = useState(false);
  const connectionRequest = useRef<AbortController | null>(null);
  useEffect(() => () => connectionRequest.current?.abort(), []);
  function uniqueName(baseName: string) {
    const names = new Set(existing.map(item => item.displayName));
    let name = baseName;
    let suffix = 2;
    while (names.has(name)) name = `${baseName} (${suffix++})`;
    return name;
  }

  async function createFromExisting(provider: ModelProviderConnection) {
    if (provider.source !== "Custom") return;
    const controller = new AbortController();
    connectionRequest.current = controller;
    onSelectionPending();
    setLoadingConnection(true);
    try {
      const next = duplicateProviderDraft(await fetchConfiguredProvider(provider.providerCode, controller.signal));
      if (!controller.signal.aborted) {
        next.displayName = uniqueName(next.displayName);
        onCreateFromExisting(next);
      }
    } catch (reason) {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : text("errors.load"));
    } finally {
      if (!controller.signal.aborted) setLoadingConnection(false);
    }
  }

  useEffect(() => {
    const controller = new AbortController();
    fetchModelProviderConnections(controller.signal).then((records) => {
      if (!controller.signal.aborted) setExisting(records.filter(provider => provider.source === "Custom"));
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : t("configuredProviders.errors.load"));
    });
    return () => controller.abort();
  }, [revision, t]);

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

  const selected = presets.find((preset) => preset.id === selectedId);
  const providerOptions = useMemo<ProviderOption[]>(() => [
    ...existing.map((provider) => ({
      value: `__existing__:${provider.providerCode}`,
      label: t("configuredProviders.catalog.newFromExisting", { name: provider.displayName }),
      searchText: `${provider.providerCode} ${provider.displayName}`,
      disabled: false,
    })),
    {
      value: "__tron_custom__",
      label: t("configuredProviders.catalog.customProvider"),
      searchText: `custom ${t("configuredProviders.catalog.customProvider")}`,
      disabled: false,
    },
    ...presets.map((preset) => ({
      value: preset.id,
      label: preset.supported && preset.protocol ? preset.name : `${preset.name} · ${t("configuredProviders.catalog.unsupported")}`,
      searchText: `${preset.id} ${preset.name}`,
      disabled: !preset.supported || !preset.protocol,
    })),
  ], [existing, presets, t]);

  return (
    <section className="space-y-4 border-b border-[var(--color-outline-variant)] pb-6" aria-labelledby="provider-presets-heading">
      <div>
        <div className="flex items-center gap-1"><h2 id="provider-presets-heading" className="text-base font-semibold">{text("catalog.title")}</h2>
          <InfoTooltip ariaLabel={text("catalog.title")}>{text("catalog.hint")}</InfoTooltip></div>
      </div>
      {loading && <p className="inline-flex items-center gap-2 text-sm" role="status"><Spinner size={16} />{text("loading")}</p>}
      <Field label={text("catalog.provider")}>
        <Combobox
          items={providerOptions}
          value={providerOptions.find((option) => option.value === selectedId) ?? null}
          disabled={disabled}
          itemToStringLabel={(option) => option.label}
          itemToStringValue={(option) => option.value}
          isItemEqualToValue={(option, value) => option.value === value.value}
          filter={(option, query) => `${option.searchText} ${option.label}`.toLowerCase().includes(query.trim().toLowerCase())}
          onValueChange={(option) => {
            const value = option?.value;
            if (!value) return;
            connectionRequest.current?.abort();
            setLoadingConnection(false);
            setError("");
            if (value === "__tron_custom__") {
              setSelectedId(value);
              onSelect(null);
              return;
            }
            if (value?.startsWith("__existing__:")) {
              const provider = existing.find((item) => item.providerCode === value.slice("__existing__:".length));
              if (provider) { setSelectedId(value); void createFromExisting(provider); }
              return;
            }
            const preset = presets.find((entry) => entry.id === value);
            if (!preset?.supported || !preset.protocol) return;
            setSelectedId(preset.id);
            onSelect({ ...preset, name: uniqueName(preset.name) });
          }}>
          <ComboboxInput
            aria-label={text("catalog.provider")}
            placeholder={text("catalog.searchOrChooseProvider")}
            autoComplete="off"
            spellCheck={false}
            className="h-[var(--tron-input-height,2.25rem)] sm:h-[var(--tron-input-height,2.125rem)]"
          />
          <ComboboxPopup className="w-(--anchor-width)">
            <ComboboxEmpty>{text("catalog.noProviders")}</ComboboxEmpty>
            <ComboboxList>
              {(option: ProviderOption) => <ComboboxItem key={option.value} value={option} disabled={option.disabled}>
                {option.label}
              </ComboboxItem>}
            </ComboboxList>
          </ComboboxPopup>
        </Combobox>
      </Field>
      {loadingConnection && <p role="status" className="inline-flex items-center gap-2 text-sm"><Spinner size={16} />{text("loading")}</p>}
      {error && <div role="alert" className="flex items-center justify-between gap-3 text-sm text-[var(--color-error)]">
        <span>{error}</span><Button type="button" variant="secondary" disabled={disabled} onClick={() => {
          const provider = selectedId.startsWith("__existing__:") ? existing.find(item => item.providerCode === selectedId.slice("__existing__:".length)) : undefined;
          if (provider) { connectionRequest.current?.abort(); setError(""); void createFromExisting(provider); }
          else setRevision((value) => value + 1);
        }}>{text("actions.retry")}</Button>
      </div>}
      {selected && <>
        <div className="flex flex-wrap gap-2">
          {selected.guessed && <Badge variant="warning">{text("catalog.guessed")}</Badge>}
          {(selected.needsBaseUrl || !selected.baseUrl) && <Badge variant="warning">{text("catalog.needsBaseUrl")}</Badge>}
        </div>
        {selected.guessed && <InfoTooltip ariaLabel={text("catalog.guessed")}>{text("catalog.guessedHint")}</InfoTooltip>}

      </>}
    </section>
  );
}

import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Square, SquareCheck } from "lucide-react";
import { Field, SelectInput, TextInput, ToggleSwitch } from "@/components/forms";
import { Combobox, ComboboxInput, ComboboxPopup, ComboboxList, ComboboxItem } from "@/components/ui/combobox";
import { SelectItem } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { InfoTooltip } from "@/components/info-tooltip";
import type { ConfiguredProviderModelType, ConfiguredProviderProtocol, ProviderInputCapability } from "@/domain/model/api/configuredProviderApi";
import { normalizeReasoningEfforts, reasoningEffortOptions } from "@/domain/model/constants/modelPageOptions";
import type { ReasoningEffort } from "@/domain/model/model";
import { configuredProviderModelTypes, emptyProviderModel, isMediaModel, isMultimodalModel, supportsModelType, type ProviderModelDraft } from "./configuredProviderForm";
import { ProviderModelPicker } from "./ProviderModelPicker";

const contextWindowPresets = [
  { label: "256K", tokens: "256000" },
  { label: "475K", tokens: "475000" },
  { label: "512K", tokens: "512000" },
  { label: "1M", tokens: "1000000" },
  { label: "2M", tokens: "2000000" },
];

function ContextWindowInput({ value, onChange, label, disabled }: {
  value: string;
  onChange: (value: string) => void;
  label: string;
  disabled: boolean;
}) {
  const { t, i18n } = useTranslation("pages");
  return (
    <Combobox items={contextWindowPresets} filter={null} disabled={disabled}
      value={contextWindowPresets.find(preset => preset.tokens === value) ?? null}
      inputValue={value} itemToStringLabel={preset => preset.tokens}
      onInputValueChange={(input, details) => {
        if (details.reason === "input-change" && /^\d*$/.test(input)) onChange(input);
      }}
      onValueChange={preset => { if (preset) onChange(preset.tokens); }}>
      <ComboboxInput required aria-label={label} inputMode="numeric" pattern="[0-9]*"
        placeholder={t("configuredProviders.models.contextPlaceholder")}
        triggerProps={{ "aria-label": t("configuredProviders.models.contextPresets") }} />
      <ComboboxPopup className="w-(--anchor-width)">
        <ComboboxList>{(preset: typeof contextWindowPresets[number]) => (
          <ComboboxItem key={preset.tokens} value={preset}>
            <span className="flex items-center justify-between gap-4">
              <span>{preset.label}</span>
              <span className="text-xs text-[var(--color-on-surface-variant)]">{new Intl.NumberFormat(i18n.language).format(Number(preset.tokens))} tokens</span>
            </span>
          </ComboboxItem>
        )}</ComboboxList>
      </ComboboxPopup>
    </Combobox>
  );
}

function ModelInputCapabilities({ model, number, disabled, onChange }: {
  model: ProviderModelDraft;
  number: number;
  disabled: boolean;
  onChange: (inputs: ProviderInputCapability[]) => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const flexibleInputs = isMediaModel(model.modelType) || isMultimodalModel(model.modelType);
  const textOnly = !flexibleInputs && model.modelType !== "Text";
  const inputs: ProviderInputCapability[] = flexibleInputs ? ["Text", "Image", "Audio", "Video"] : textOnly ? ["Text"] : ["Text", "Image"];
  const help = textOnly ? "models.textOnlyInputsHelp" : model.modelType === "Text" ? "models.chatInputsHelp" : isMultimodalModel(model.modelType) ? "models.multimodalInputsHelp" : "models.mediaInputsHelp";

  return <fieldset className="min-w-0 space-y-2">
    <legend className="inline-flex items-center gap-1 text-sm font-medium">
      {text("models.inputTypes")}
      <span className="font-normal text-[var(--color-on-surface-variant)]">{text(textOnly ? "models.fixedInputs" : "models.multipleInputs")}</span>
      <InfoTooltip ariaLabel={text("models.inputTypes")}>
        <div className="space-y-2"><p>{text("models.inputCapabilitiesHelp")}</p><p>{text(help)}</p></div>
      </InfoTooltip>
    </legend>
    <div className="flex flex-wrap gap-2">
      {inputs.map((input) => {
        const checked = model.inputCapabilities.includes(input);
        const required = input === "Text" && !flexibleInputs;
        const Icon = checked ? SquareCheck : Square;
        return <Button key={input} type="button" size="sm" variant={checked ? "secondary" : "outline"}
          aria-pressed={checked}
          aria-label={`${t("configuredProviders.models.inputNumber", { number, input: text(`inputs.${input}`) })}${required ? ` ${text("models.requiredInput")}` : ""}`}
          disabled={disabled || required}
          onClick={() => onChange(checked ? model.inputCapabilities.filter((value) => value !== input) : [...model.inputCapabilities, input])}>
          <Icon aria-hidden="true" className="size-4" />
          {text(`inputs.${input}`)}
          {required && <span className="text-xs">{text("models.requiredInput")}</span>}
        </Button>;
      })}
    </div>
  </fieldset>;
}

export function ConfiguredProviderModels({ presetId, models, protocol, disabled, defaultReasoningEfforts, onDefaultReasoningEffortsChange, onChange }: {
  presetId?: string | null;
  models: ProviderModelDraft[];
  protocol: ConfiguredProviderProtocol;
  disabled: boolean;
  defaultReasoningEfforts: ReasoningEffort[];
  onDefaultReasoningEffortsChange: (efforts: ReasoningEffort[]) => void;
  onChange: (models: ProviderModelDraft[]) => void;
}) {
  const { t } = useTranslation("pages");
  const editorPrefix = useId();
  const [editingRowId, setEditingRowId] = useState<string | null>(() => models.find(model => !model.modelId.trim())?.rowId ?? null);
  const text = (key: string) => t(`configuredProviders.${key}`);
  const effortLabel = (value: ReasoningEffort) => t(`common:reasoningEffort.values.${value}`);
  const update = (rowId: string, patch: Partial<ProviderModelDraft>) => onChange(models.map((model) => model.rowId === rowId ? { ...model, ...patch } : model));
  const effortValues = (values: Iterable<string>) => normalizeReasoningEfforts(Array.from(values) as ReasoningEffort[]);
  return (
    <section className="space-y-4 border-t border-[var(--color-outline-variant)] pt-6" aria-labelledby="provider-models-heading">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-1"><h2 id="provider-models-heading" className="text-base font-semibold">{text("models.title")}</h2>
            <InfoTooltip ariaLabel={text("models.title")}>{text("models.hint")}</InfoTooltip></div>
        </div>
        <Button type="button" variant="secondary" disabled={disabled} onClick={() => { const model = emptyProviderModel(); setEditingRowId(model.rowId); onChange([...models, model]); }}>{text("models.add")}</Button>
      </div>
      <Field label={<span className="inline-flex items-center gap-1">{text("models.providerReasoningEfforts")}<InfoTooltip ariaLabel={text("models.providerReasoningEfforts")}>{text("models.providerReasoningEffortsHint")}</InfoTooltip></span>}>
        <SelectInput multiple aria-label={text("models.providerReasoningEfforts")}
          value={defaultReasoningEfforts} disabled={disabled}
          onValueChange={(values) => onDefaultReasoningEffortsChange(effortValues(values ?? []))}>
          {reasoningEffortOptions.map((option) => <SelectItem key={option.value} value={option.value}>{effortLabel(option.value)}</SelectItem>)}
        </SelectInput>
      </Field>
      <ProviderModelPicker key={presetId ?? "manual"} presetId={presetId} models={models} protocol={protocol} disabled={disabled} onChange={onChange} />
      {models.map((model, index) => (
        <div key={model.rowId} className="min-w-0 border-b border-[var(--color-outline-variant)] py-2 last:border-b-0">
          <div className="flex items-center justify-between gap-3">
            <span className="min-w-0 truncate font-mono text-sm" title={model.modelId}>{model.modelId || text("models.new")}</span>
            <div className="flex shrink-0 items-center gap-1">
              <Button type="button" variant="ghost" size="sm" disabled={disabled}
                aria-label={t("configuredProviders.models.editNumber", { number: index + 1 })}
                aria-expanded={editingRowId === model.rowId} aria-controls={`${editorPrefix}-${model.rowId}`}
                onClick={() => setEditingRowId(current => current === model.rowId ? null : model.rowId)}>
                {text(editingRowId === model.rowId ? "actions.collapse" : "actions.edit")}
              </Button>
              <Button type="button" variant="ghost" size="sm" disabled={disabled}
                aria-label={t("configuredProviders.models.removeNumber", { number: index + 1 })}
                onClick={() => onChange(models.filter((entry) => entry.rowId !== model.rowId))}>{text("actions.remove")}</Button>
            </div>
          </div>
          {editingRowId === model.rowId && <fieldset id={`${editorPrefix}-${model.rowId}`} disabled={disabled}
            aria-label={t("configuredProviders.models.number", { number: index + 1 })} className="min-w-0 space-y-4 pt-4 pb-3">
          <div className="grid gap-4 md:grid-cols-2">
            <Field label={text("models.modelType")} required>
              <SelectInput aria-label={t("configuredProviders.models.typeNumber", { number: index + 1 })}
                value={model.modelType} disabled={disabled} onValueChange={(value) => {
                  if (!value) return;
                  const modelType = value as ConfiguredProviderModelType;
                  update(model.rowId, modelType === "Text" ? { modelType, inputCapabilities: ["Text"] } : {
                    modelType, reasoning: false, supportedReasoningEfforts: [], defaultReasoningEffort: null, supportsDeveloperRole: false,
                    inputCapabilities: isMultimodalModel(modelType) ? ["Text", "Image"] : modelType === "SpeechRecognition" ? ["Audio"] : modelType === "RealtimeSpeech" ? ["Text", "Audio"] : ["Text"],
                    maxOutputTokens: "", contextWindowTokens: "",
                  });
                }}>
                {configuredProviderModelTypes.map((type) => <SelectItem key={type} value={type}
                  disabled={!supportsModelType(type, protocol)}>{text(`modelTypes.${type}`)}</SelectItem>)}
              </SelectInput>
            </Field>
            <Field label={text("models.modelId")} required>
              <TextInput aria-label={t("configuredProviders.models.idNumber", { number: index + 1 })} value={model.modelId}
                autoComplete="off" spellCheck={false} onChange={(event) => update(model.rowId, { modelId: event.currentTarget.value })} />
            </Field>
            {["TextEmbedding", "MultimodalEmbedding", "TextRerank", "MultimodalRerank"].includes(model.modelType) && <Field label={<span className="inline-flex items-center gap-1">{text("models.logicalCode")}<InfoTooltip ariaLabel={text("models.logicalCode")}>{text("models.logicalCodeHelp")}</InfoTooltip></span>}>
              <TextInput aria-label={text("models.logicalCode")} value={model.logicalModelCode ?? ""} autoComplete="off" spellCheck={false} onChange={event => update(model.rowId, { logicalModelCode: event.currentTarget.value })} />
            </Field>}
            <Field label={text("models.name")} required>
              <TextInput aria-label={t("configuredProviders.models.nameNumber", { number: index + 1 })} value={model.name}
                onChange={(event) => update(model.rowId, { name: event.currentTarget.value })} />
            </Field>
            {!isMediaModel(model.modelType) && <Field label={text("models.contextWindowTokens")} required>
              <ContextWindowInput label={t("configuredProviders.models.contextNumber", { number: index + 1 })}
                value={model.contextWindowTokens} disabled={disabled}
                onChange={(contextWindowTokens) => update(model.rowId, { contextWindowTokens })} />
            </Field>}
            {(model.modelType === "TextEmbedding" || model.modelType === "MultimodalEmbedding") && <Field label={text("models.dimension")} required><TextInput type="text" inputMode="numeric" pattern="[0-9]*" value={model.dimension} onChange={(event) => { if (/^\d*$/.test(event.currentTarget.value)) update(model.rowId, { dimension: event.currentTarget.value }); }} /></Field>}
            {model.modelType === "Text" && <Field label={text("models.maxOutputTokens")}>
              <TextInput aria-label={t("configuredProviders.models.outputNumber", { number: index + 1 })} type="text" inputMode="numeric" pattern="[0-9]*"
                value={model.maxOutputTokens} placeholder={text("models.unspecified")}
                onChange={(event) => { if (/^\d*$/.test(event.currentTarget.value)) update(model.rowId, { maxOutputTokens: event.currentTarget.value }); }} />
            </Field>}
          </div>
          <div className="flex flex-wrap gap-x-6 gap-y-3">
            <ToggleSwitch label={text("models.enabled")} ariaLabel={t("configuredProviders.models.enabledNumber", { number: index + 1 })}
              checked={model.enabled} disabled={disabled} onCheckedChange={(enabled) => update(model.rowId, { enabled })} />
            {model.modelType === "Text" && <ToggleSwitch label={text("models.reasoning")} ariaLabel={t("configuredProviders.models.reasoningNumber", { number: index + 1 })}
              checked={model.reasoning} disabled={disabled} onCheckedChange={(reasoning) => update(model.rowId, { reasoning, ...(reasoning ? {} : { supportedReasoningEfforts: [], defaultReasoningEffort: null }) })} />}
            {protocol === "OpenAiChatCompletions" && model.modelType === "Text" && <ToggleSwitch label={text("models.developerRole")} ariaLabel={t("configuredProviders.models.developerNumber", { number: index + 1 })}
              checked={model.supportsDeveloperRole} disabled={disabled} onCheckedChange={(supportsDeveloperRole) => update(model.rowId, { supportsDeveloperRole })} />}
          </div>
          {model.modelType === "Text" && model.reasoning ? (() => {
            const effectiveEfforts = model.supportedReasoningEfforts.length
              ? model.supportedReasoningEfforts
              : defaultReasoningEfforts;
            return <div className="grid gap-4 md:grid-cols-2">
              <Field label={text("models.reasoningEfforts")} hint={text("models.reasoningEffortsHint")} hintAriaLabel={text("models.reasoningEfforts")}>
                <SelectInput multiple aria-label={t("configuredProviders.models.reasoningEffortsNumber", { number: index + 1 })}
                  value={model.supportedReasoningEfforts} disabled={disabled}
                  onValueChange={(values) => {
                    const supportedReasoningEfforts = effortValues(values ?? []);
                    update(model.rowId, {
                      supportedReasoningEfforts,
                      defaultReasoningEffort: model.defaultReasoningEffort && supportedReasoningEfforts.includes(model.defaultReasoningEffort)
                        ? model.defaultReasoningEffort
                        : null,
                    });
                  }}>
                  {reasoningEffortOptions.map((option) => <SelectItem key={option.value} value={option.value}>{effortLabel(option.value)}</SelectItem>)}
                </SelectInput>
              </Field>
              <Field label={text("models.defaultReasoningEffort")} hint={text("models.defaultReasoningEffortHint")} hintAriaLabel={text("models.defaultReasoningEffort")}>
                <SelectInput aria-label={t("configuredProviders.models.defaultReasoningEffortNumber", { number: index + 1 })}
                  value={model.defaultReasoningEffort ?? "__provider_default__"} disabled={disabled || effectiveEfforts.length === 0}
                  onValueChange={(value) => update(model.rowId, { defaultReasoningEffort: value === "__provider_default__" ? null : value as ReasoningEffort })}>
                  <SelectItem value="__provider_default__">{t("common:reasoningEffort.providerDefault")}</SelectItem>
                  {reasoningEffortOptions.filter((option) => effectiveEfforts.includes(option.value)).map((option) => (
                    <SelectItem key={option.value} value={option.value}>{effortLabel(option.value)}</SelectItem>
                  ))}
                </SelectInput>
              </Field>
            </div>;
          })() : null}
          <ModelInputCapabilities model={model} number={index + 1} disabled={disabled}
            onChange={(inputCapabilities) => update(model.rowId, { inputCapabilities })} />
          </fieldset>}
        </div>
      ))}
    </section>
  );
}

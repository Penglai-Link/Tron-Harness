import { useTranslation } from "react-i18next";
import { Field, TextInput, ToggleSwitch } from "@/components/forms";
import { Button } from "@/components/ui/button";
import type { ConfiguredProviderProtocol, ProviderInputCapability } from "@/domain/model/api/configuredProviderApi";
import { emptyProviderModel, type ProviderModelDraft } from "./configuredProviderForm";

export function ConfiguredProviderModels({ models, protocol, disabled, onChange }: {
  models: ProviderModelDraft[];
  protocol: ConfiguredProviderProtocol;
  disabled: boolean;
  onChange: (models: ProviderModelDraft[]) => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const update = (rowId: string, patch: Partial<ProviderModelDraft>) => onChange(models.map((model) => model.rowId === rowId ? { ...model, ...patch } : model));
  const inputs: ProviderInputCapability[] = ["Image"];
  return (
    <section className="space-y-4 border-t border-[var(--color-outline-variant)] pt-6" aria-labelledby="provider-models-heading">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 id="provider-models-heading" className="text-base font-semibold">{text("models.title")}</h2>
          <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{text("models.hint")}</p>
        </div>
        <Button type="button" variant="secondary" disabled={disabled} onClick={() => onChange([...models, emptyProviderModel()])}>{text("models.add")}</Button>
      </div>
      {!models.length && <p className="text-sm text-[var(--color-on-surface-variant)]">{text("models.empty")}</p>}
      {models.map((model, index) => (
        <fieldset key={model.rowId} disabled={disabled} className="min-w-0 space-y-4 border-b border-[var(--color-outline-variant)] py-5 last:border-b-0">
          <legend className="flex w-full items-center justify-between gap-3 text-sm font-semibold">
            <span>{t("configuredProviders.models.number", { number: index + 1 })}</span>
            <Button type="button" variant="ghost" disabled={disabled}
              aria-label={t("configuredProviders.models.removeNumber", { number: index + 1 })}
              onClick={() => onChange(models.filter((entry) => entry.rowId !== model.rowId))}>{text("actions.remove")}</Button>
          </legend>
          <div className="grid gap-4 md:grid-cols-2">
            <Field label={text("models.modelId")} required>
              <TextInput aria-label={t("configuredProviders.models.idNumber", { number: index + 1 })} value={model.modelId}
                autoComplete="off" spellCheck={false} onChange={(event) => update(model.rowId, { modelId: event.currentTarget.value })} />
            </Field>
            <Field label={text("models.name")} required>
              <TextInput aria-label={t("configuredProviders.models.nameNumber", { number: index + 1 })} value={model.name}
                onChange={(event) => update(model.rowId, { name: event.currentTarget.value })} />
            </Field>
            <Field label={text("models.contextWindowTokens")}>
              <TextInput aria-label={t("configuredProviders.models.contextNumber", { number: index + 1 })} type="number" min={1} step={1}
                value={model.contextWindowTokens} placeholder={text("models.unspecified")}
                onChange={(event) => update(model.rowId, { contextWindowTokens: event.currentTarget.value })} />
            </Field>
            <Field label={text("models.maxOutputTokens")}>
              <TextInput aria-label={t("configuredProviders.models.outputNumber", { number: index + 1 })} type="number" min={1} step={1}
                value={model.maxOutputTokens} placeholder={text("models.unspecified")}
                onChange={(event) => update(model.rowId, { maxOutputTokens: event.currentTarget.value })} />
            </Field>
          </div>
          <div className="flex flex-wrap gap-x-6 gap-y-3">
            <ToggleSwitch label={text("models.enabled")} ariaLabel={t("configuredProviders.models.enabledNumber", { number: index + 1 })}
              checked={model.enabled} disabled={disabled} onCheckedChange={(enabled) => update(model.rowId, { enabled })} />
            <ToggleSwitch label={text("models.reasoning")} ariaLabel={t("configuredProviders.models.reasoningNumber", { number: index + 1 })}
              checked={model.reasoning} disabled={disabled} onCheckedChange={(reasoning) => update(model.rowId, { reasoning })} />
            {protocol === "OpenAiChatCompletions" && <ToggleSwitch label={text("models.developerRole")} ariaLabel={t("configuredProviders.models.developerNumber", { number: index + 1 })}
              checked={model.supportsDeveloperRole} disabled={disabled} onCheckedChange={(supportsDeveloperRole) => update(model.rowId, { supportsDeveloperRole })} />}
          </div>
          <div className="flex flex-wrap items-center gap-x-6 gap-y-3">
            <span className="text-sm text-[var(--color-on-surface-variant)]">{text("models.inputTypes")}</span>
            <span className="text-sm">{text("inputs.Text")}</span>
            {inputs.map((input) => <ToggleSwitch key={input} label={text(`inputs.${input}`)}
              ariaLabel={t("configuredProviders.models.inputNumber", { number: index + 1, input: text(`inputs.${input}`) })}
              checked={model.inputCapabilities.includes(input)} disabled={disabled}
              onCheckedChange={(checked) => update(model.rowId, { inputCapabilities: checked
                ? [...model.inputCapabilities.filter((value) => value !== input), input]
                : model.inputCapabilities.filter((value) => value !== input) })} />)}
          </div>
        </fieldset>
      ))}
    </section>
  );
}

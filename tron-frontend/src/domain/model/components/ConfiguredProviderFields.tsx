import { useTranslation } from "react-i18next";
import { useState } from "react";
import { Field, SelectInput, TextInput, ToggleSwitch } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import type { ConfiguredProviderProtocol } from "@/domain/model/api/configuredProviderApi";
import { configuredProviderProtocols, draftRowId, type ConfiguredProviderDraft } from "./configuredProviderForm";

export function ConfiguredProviderFields({ draft, disabled, onChange }: {
  draft: ConfiguredProviderDraft;
  disabled: boolean;
  onChange: (draft: ConfiguredProviderDraft) => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const [editingApiKey, setEditingApiKey] = useState(false);
  const showKeySummary = !editingApiKey && !draft.apiKey && draft.hasApiKey && Boolean(draft.apiKeySummary);
  const set = (patch: Partial<ConfiguredProviderDraft>) => onChange({ ...draft, ...patch });
  return (
    <section className="space-y-5" aria-labelledby="provider-connection-heading">
      <h2 id="provider-connection-heading" className="text-base font-semibold">{text("connection")}</h2>
      <div className="grid gap-5 md:grid-cols-2">
        <Field label={text("fields.displayName")} required>
          <TextInput aria-label={text("fields.displayName")} value={draft.displayName} disabled={disabled}
            onChange={(event) => set({ displayName: event.currentTarget.value })} />
        </Field>
        <Field label={text("fields.protocol")} required>
          <SelectInput value={draft.protocol} disabled={disabled} aria-label={text("fields.protocol")}
            onValueChange={(value: ConfiguredProviderProtocol | null) => {
              if (value) set({ protocol: value, models: draft.models.map((model) => ({
                ...model, supportsDeveloperRole: value === "OpenAiChatCompletions" && model.supportsDeveloperRole,
                inputCapabilities: model.inputCapabilities.filter((input) => input === "Text" || input === "Image"),
              })) });
            }}>
            {configuredProviderProtocols.map((protocol) => <SelectItem key={protocol} value={protocol}>{text(`protocols.${protocol}`)}</SelectItem>)}
          </SelectInput>
        </Field>
        <Field label={text("fields.baseUrl")} required hint={text("hints.baseUrl")} hintAriaLabel={text("fields.baseUrl")}>
          <TextInput aria-label={text("fields.baseUrl")} value={draft.baseUrl} disabled={disabled}
            type="url" autoComplete="off" spellCheck={false}
            onChange={(event) => set({ baseUrl: event.currentTarget.value })} />
        </Field>
        <Field label={text("fields.apiKey")} hint={draft.hasApiKey ? text("hints.keySaved") : text("hints.keyOptional")} hintAriaLabel={text("fields.apiKey")}>
          <TextInput aria-label={text("fields.apiKey")} type={showKeySummary ? "text" : "password"} autoComplete="new-password"
            value={showKeySummary ? draft.apiKeySummary ?? "" : draft.apiKey} disabled={disabled}
            onFocus={() => setEditingApiKey(true)} onBlur={() => setEditingApiKey(false)}
            onChange={(event) => set({ apiKey: event.currentTarget.value })} />
        </Field>
        <div className="flex items-center">
          <ToggleSwitch label={text("fields.enabled")} ariaLabel={text("fields.enabled")} checked={draft.enabled} disabled={disabled}
            onCheckedChange={(enabled) => set({ enabled })} />
        </div>
      </div>
      <details className="border-t border-[var(--color-outline-variant)] pt-4">
        <summary className="cursor-pointer text-sm font-medium"><span className="inline-flex items-center gap-1">{text("headers.title")}<InfoTooltip ariaLabel={text("headers.title")}>{text("headers.hint")}</InfoTooltip></span></summary>
        <div className="mt-3 space-y-3">
          {draft.headers.map((header, index) => (
            <div className="grid grid-cols-[1fr_1fr_auto] items-end gap-3" key={header.rowId}>
              <Field label={text("headers.name")}>
                <TextInput aria-label={t("configuredProviders.headers.nameNumber", { number: index + 1 })} value={header.name} disabled={disabled}
                  autoComplete="off" spellCheck={false}
                  onChange={(event) => set({ headers: draft.headers.map((entry) => entry.rowId === header.rowId ? { ...entry, name: event.currentTarget.value } : entry) })} />
              </Field>
              <Field label={text("headers.value")}>
                <TextInput aria-label={t("configuredProviders.headers.valueNumber", { number: index + 1 })} value={header.value} disabled={disabled}
                  autoComplete="off" spellCheck={false}
                  onChange={(event) => set({ headers: draft.headers.map((entry) => entry.rowId === header.rowId ? { ...entry, value: event.currentTarget.value } : entry) })} />
              </Field>
              <Button type="button" variant="ghost" disabled={disabled}
                aria-label={t("configuredProviders.headers.removeNumber", { number: index + 1 })}
                onClick={() => set({ headers: draft.headers.filter((entry) => entry.rowId !== header.rowId) })}>{text("actions.remove")}</Button>
            </div>
          ))}
          <Button type="button" variant="secondary" disabled={disabled}
            onClick={() => set({ headers: [...draft.headers, { rowId: draftRowId(), name: "", value: "" }] })}>{text("headers.add")}</Button>
        </div>
      </details>
    </section>
  );
}

import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput, ToggleSwitch } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import type { ConfiguredProviderProtocol } from "@/domain/model/api/configuredProviderApi";
import { configuredProviderProtocols, draftRowId, type ConfiguredProviderDraft } from "./configuredProviderForm";

export function ConfiguredProviderFields({ draft, editing, disabled, onChange }: {
  draft: ConfiguredProviderDraft;
  editing: boolean;
  disabled: boolean;
  onChange: (draft: ConfiguredProviderDraft) => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`configuredProviders.${key}`);
  const set = (patch: Partial<ConfiguredProviderDraft>) => onChange({ ...draft, ...patch });
  return (
    <section className="space-y-5" aria-labelledby="provider-connection-heading">
      <h2 id="provider-connection-heading" className="text-base font-semibold">{text("connection")}</h2>
      <div className="grid gap-5 md:grid-cols-2">
        <Field label={text("fields.providerId")} required>
          <TextInput aria-label={text("fields.providerId")} value={draft.providerId} disabled={editing || disabled}
            onChange={(event) => set({ providerId: event.currentTarget.value })} autoComplete="off" spellCheck={false} />
          <p className="text-xs text-[var(--color-on-surface-variant)]">{text("hints.providerId")}</p>
        </Field>
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
        <Field label={text("fields.baseUrl")} required>
          <TextInput aria-label={text("fields.baseUrl")} value={draft.baseUrl} disabled={disabled}
            type="url" autoComplete="off" spellCheck={false}
            onChange={(event) => set({ baseUrl: event.currentTarget.value })} />
          <p className="text-xs text-[var(--color-on-surface-variant)]">{text("hints.baseUrl")}</p>
        </Field>
        <Field label={text("fields.apiKey")}>
          <TextInput aria-label={text("fields.apiKey")} type="password" autoComplete="new-password" value={draft.apiKey} disabled={disabled}
            placeholder={draft.hasApiKey ? text("hints.keySaved") : text("hints.keyOptional")}
            onChange={(event) => set({ apiKey: event.currentTarget.value })} />
          <p className="text-xs text-[var(--color-on-surface-variant)]">{draft.hasApiKey ? text("hints.keySaved") : text("hints.keyOptional")}</p>
        </Field>
        <div className="flex items-center">
          <ToggleSwitch label={text("fields.enabled")} ariaLabel={text("fields.enabled")} checked={draft.enabled} disabled={disabled}
            onCheckedChange={(enabled) => set({ enabled })} />
        </div>
      </div>
      <details className="border-t border-[var(--color-outline-variant)] pt-4">
        <summary className="cursor-pointer text-sm font-medium">{text("headers.title")}</summary>
        <p className="mt-3 text-xs text-[var(--color-on-surface-variant)]">{text("headers.hint")}</p>
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

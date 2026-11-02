import { AgentAvatarPicker } from "./AgentAvatarPicker";
import { AgentDescriptionField } from "./AgentDescriptionField";
import type { ReasoningEffort } from "@/domain/model/model/ModelRegistryModel";
import { AgentActivationSwitch } from "./AgentActivationSwitch";
import { useToast } from "@/components/ui/toast";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Combobox, ComboboxInput, ComboboxPopup, ComboboxList, ComboboxItem, ComboboxEmpty } from "@/components/ui/combobox";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { InfoTooltip } from "@/components/info-tooltip";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import type { PlatformSystemAgentRecord, SystemAgentProfilePayload } from "../model";

export function SystemAgentProfileDialog({ agents, record, onSave, onClose }: {
  agents: PlatformSystemAgentRecord[];
  record?: PlatformSystemAgentRecord;
  onSave: (payload: SystemAgentProfilePayload & { enabled: boolean }) => Promise<void>;
  onClose: () => void;
}) {
  const { t } = useTranslation(["pages", "common"]);
  const text = (key: string) => t(`pages:settingsAgents.profile.${key}`);
  const bases = agents.filter((agent) => !agent.baseAgentCode && (Boolean(record) || (agent.enabled && agent.available)));
  const [baseAgentCode, setBase] = useState(record?.baseAgentCode ?? record?.agentCode ?? bases[0]?.agentCode ?? "");
  const [agentCode, setCode] = useState(record?.agentCode ?? "");
  const [name, setName] = useState(record?.name ?? "");
  const [description, setDescription] = useState(record?.description ?? "");
  const [modelId, setModel] = useState(record?.modelId ?? "");
  const isBase = Boolean(record && !record.baseAgentCode);
  const [avatarId, setAvatar] = useState(record?.avatarId ?? "robot");
  const [effort, setEffort] = useState<ReasoningEffort | "Default">(record?.reasoningEffort ?? "Default");
  const [fastMode, setFastMode] = useState(record?.fastMode ?? false);
  const [enabled, setEnabled] = useState(record?.enabled ?? true);
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const base = bases.find((agent) => agent.agentCode === baseAgentCode);
  const isCodex = baseAgentCode === "codex";
  const codeValid = /^[a-z][a-z0-9_-]{0,63}$/.test(agentCode);
  const modelValid = isBase || (Boolean(modelId.trim()) && (!["opencode", "dsh"].includes(baseAgentCode) || /^[^/\s]+\/\S+$/.test(modelId.trim())));
  const selectedModel = base?.models.find((model) => model.modelId === modelId.trim());
  const effortOptions: ReasoningEffort[] = selectedModel?.supportedReasoningEfforts?.length
    ? selectedModel.supportedReasoningEfforts : baseAgentCode === "dsh" ? [] : ["None", "Minimal", "Low", "Medium", "High", "XHigh", "Max", "Ultra"];
  async function save() {
    if (busy || !codeValid || !name.trim() || !modelValid || !baseAgentCode) return;
    setBusy(true);
    try { await onSave({ agentCode, name: name.trim(), description: description.trim(), baseAgentCode, modelId: modelId.trim(), reasoningEffort: effort === "Default" ? null : effort, fastMode: isCodex && fastMode, avatarId, enabled }); onClose(); }
    catch (cause) { toast.error(cause instanceof Error ? cause.message : text("saveFailed")); }
    finally { setBusy(false); }
  }
  return <Dialog disablePointerDismissal open onOpenChange={(open, details) => {
    if (busy || details.reason === "outside-press") { details.cancel(); return; }
    if (!open) onClose();
  }}>
    <DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-2xl" showCloseButton={!busy}>
      <DialogHeader><div className="flex items-center justify-between gap-4 pr-8"><DialogTitle>{text(record ? "editTitle" : "createTitle")}</DialogTitle><AgentActivationSwitch checked={enabled} disabled={busy} onChange={setEnabled} /></div></DialogHeader>
      <form className="space-y-4" onSubmit={(event) => { event.preventDefault(); void save(); }}>
        <Field label={text("base")} required>
          <SelectInput value={baseAgentCode} disabled={busy || Boolean(record)} onValueChange={(value) => { setBase(String(value)); setModel(""); setEffort("Default"); setFastMode(false); }}>
            {bases.map((agent) => <SelectItem key={agent.agentCode} value={agent.agentCode}>{agent.name}</SelectItem>)}
            {record?.baseAgentCode && !base ? <SelectItem value={record.baseAgentCode}>{record.baseAgentCode}</SelectItem> : null}
          </SelectInput>
        </Field>
        {!bases.length && !record ? <p role="status">{text("scanFirst")}</p> : null}
        <AgentAvatarPicker value={avatarId} disabled={busy} onChange={setAvatar} />
        <Field label={text("name")} required><TextInput value={name} disabled={busy || isBase} maxLength={128} onChange={(event) => setName(event.target.value)} /></Field>
        <AgentDescriptionField value={description} disabled={busy} onChange={setDescription} />
        {!record ? <Field label={text("code")} required error={agentCode && !codeValid ? text("codeHelp") : undefined}>
          <TextInput value={agentCode} disabled={busy} maxLength={64} onChange={(event) => setCode(event.target.value)} />
        </Field> : null}
        {!isBase && <Field label={text("model")} required>
          <Combobox
            items={base?.models.map((model) => model.modelId) ?? []}
            value={modelId || null}
            inputValue={modelId}
            disabled={busy}
            onValueChange={(value) => { if (value !== null) { setModel(value); setEffort("Default"); } }}
            onInputValueChange={(value, details) => {
              if (["input-change", "input-clear", "clear-press"].includes(details.reason)) {
                setModel(value); setEffort("Default");
              }
            }}
          >
            <ComboboxInput aria-label={text("model")} maxLength={256} placeholder={text("modelPlaceholder")} autoComplete="off" spellCheck={false} />
            <ComboboxPopup className="w-(--anchor-width)">
              <ComboboxEmpty>{text("customModelHelp")}</ComboboxEmpty>
              <ComboboxList>
                {(id: string) => {
                  const model = base?.models.find((item) => item.modelId === id);
                  return <ComboboxItem key={id} value={id}>
                    <div className="min-w-0 py-1">
                      <div className="break-all text-sm">{id}</div>
                      {model?.displayName !== id && <div className="truncate text-xs text-[var(--color-on-surface-variant)]">{model?.displayName}</div>}
                    </div>
                  </ComboboxItem>;
                }}
              </ComboboxList>
            </ComboboxPopup>
          </Combobox>
          {["opencode", "dsh"].includes(baseAgentCode) ? <p className="text-xs text-[var(--color-on-surface-variant)]">{text(baseAgentCode === "dsh" ? "dshModelHelp" : "openCodeModelHelp")}</p> : null}
        </Field>}
        {!isBase && <Field label={t("common:reasoningEffort.chatLabel")}>
          <SelectInput value={effort} disabled={busy} onValueChange={(value) => setEffort(value as ReasoningEffort | "Default")}>
            <SelectItem value="Default">{t("common:reasoningEffort.providerDefault")}</SelectItem>
            {effortOptions.map((value) => <SelectItem key={value} value={value}>{t(`common:reasoningEffort.values.${value}`)}</SelectItem>)}
          </SelectInput>
        </Field>}
        {isCodex && <div className="flex items-center justify-between gap-4">
          <div className="inline-flex items-center gap-1.5">
            <label htmlFor="system-agent-fast-mode" className="text-sm font-medium">{text("fastMode")}</label>
            <InfoTooltip ariaLabel={text("fastMode")}>{text("fastModeHelp")}</InfoTooltip>
          </div>
          <Switch id="system-agent-fast-mode" checked={fastMode} disabled={busy} onCheckedChange={setFastMode} />
        </div>}
        <DialogFooter>
          <Button type="button" variant="outline" disabled={busy} onClick={onClose}>{text("cancel")}</Button>
          <Button type="submit" disabled={busy || !codeValid || !name.trim() || !modelValid || !baseAgentCode}>{text(busy ? "saving" : "save")}</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>;
}

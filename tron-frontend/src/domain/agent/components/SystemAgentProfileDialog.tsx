import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import type { PlatformSystemAgentRecord, SystemAgentProfilePayload } from "../model";

export function SystemAgentProfileDialog({ agents, record, onSave, onClose }: {
  agents: PlatformSystemAgentRecord[];
  record?: PlatformSystemAgentRecord;
  onSave: (payload: SystemAgentProfilePayload) => Promise<void>;
  onClose: () => void;
}) {
  const { t } = useTranslation(["pages", "common"]);
  const text = (key: string) => t(`pages:settingsAgents.profile.${key}`);
  const bases = agents.filter((agent) => !agent.baseAgentCode);
  const [baseAgentCode, setBase] = useState(record?.baseAgentCode ?? bases[0]?.agentCode ?? "");
  const [agentCode, setCode] = useState(record?.agentCode ?? "");
  const [name, setName] = useState(record?.name ?? "");
  const [modelId, setModel] = useState(record?.modelId ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const modelList = useId();
  const base = bases.find((agent) => agent.agentCode === baseAgentCode);
  const codeValid = /^[a-z][a-z0-9_-]{0,63}$/.test(agentCode);
  const modelValid = Boolean(modelId.trim()) && (baseAgentCode !== "opencode" || /^[^/\s]+\/\S+$/.test(modelId.trim()));
  async function save() {
    if (busy || !codeValid || !name.trim() || !modelValid || !baseAgentCode) return;
    setBusy(true); setError("");
    try { await onSave({ agentCode, name: name.trim(), baseAgentCode, modelId: modelId.trim() }); onClose(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : text("saveFailed")); }
    finally { setBusy(false); }
  }
  return <Dialog disablePointerDismissal open onOpenChange={(open, details) => {
    if (busy || details.reason === "outside-press") { details.cancel(); return; }
    if (!open) onClose();
  }}>
    <DialogContent className="sm:max-w-lg" showCloseButton={!busy}>
      <DialogHeader><DialogTitle>{text(record ? "editTitle" : "createTitle")}</DialogTitle></DialogHeader>
      <form className="space-y-4" onSubmit={(event) => { event.preventDefault(); void save(); }}>
        <Field label={text("base")} required>
          <SelectInput value={baseAgentCode} disabled={busy || Boolean(record)} onValueChange={(value) => { setBase(String(value)); setModel(""); }}>
            {bases.map((agent) => <SelectItem key={agent.agentCode} value={agent.agentCode}>{agent.name}</SelectItem>)}
            {record?.baseAgentCode && !base ? <SelectItem value={record.baseAgentCode}>{record.baseAgentCode}</SelectItem> : null}
          </SelectInput>
        </Field>
        {!bases.length && !record ? <p role="status">{text("scanFirst")}</p> : null}
        <Field label={text("name")} required><TextInput value={name} disabled={busy} maxLength={128} onChange={(event) => setName(event.target.value)} /></Field>
        {!record ? <Field label={text("code")} required error={agentCode && !codeValid ? text("codeHelp") : undefined}>
          <TextInput value={agentCode} disabled={busy} maxLength={64} onChange={(event) => setCode(event.target.value)} />
        </Field> : null}
        <Field label={text("model")} required>
          <TextInput value={modelId} list={modelList} disabled={busy} maxLength={256} placeholder={text("modelPlaceholder")} onChange={(event) => setModel(event.target.value)} />
          <datalist id={modelList}>{base?.models.map((model) => <option key={model.modelId} value={model.modelId}>{model.displayName}</option>)}</datalist>
          {baseAgentCode === "opencode" ? <p className="text-xs text-[var(--color-on-surface-variant)]">{text("openCodeModelHelp")}</p> : null}
        </Field>
        <p className="text-xs text-[var(--color-on-surface-variant)]">{text("help")}</p>
        {error ? <p role="alert" className="text-destructive-foreground">{error}</p> : null}
        <DialogFooter>
          <Button type="button" variant="outline" disabled={busy} onClick={onClose}>{text("cancel")}</Button>
          <Button type="submit" disabled={busy || !codeValid || !name.trim() || !modelValid || !baseAgentCode}>{text(busy ? "saving" : "save")}</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>;
}

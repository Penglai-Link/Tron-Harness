import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { createWorkspaceInternalAgent } from "@/domain/agent/api";
import { createWorkspaceInternalAgentMock } from "@/domain/agent/mock";
import { notifyInternalAgentRegistryChanged } from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { AgentRolePromptField } from "./AgentRolePromptField";

import { AgentAvatarPicker } from "./AgentAvatarPicker";
import { useInternalAgentGroups } from "../hooks/useInternalAgentGroups";
import { InternalAgentGroupField } from "./InternalAgentGroupField";
import { AgentLoopField } from "./AgentLoopField";
import { AgentWorkspaceField } from "./AgentWorkspaceField";
import { persistAgentWorkspaceGrants } from "../hooks/agentWorkspaceGrants";

export function CreateInternalAgentDialog({ workspaceCode, onCreated, allowWorkspaceAssignment = false }: { workspaceCode: string; onCreated: () => Promise<void>; allowWorkspaceAssignment?: boolean }) {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const [open, setOpen] = useState(false);
  const [agentCode, setAgentCode] = useState("");
  const [name, setName] = useState("");
  const [loopCode, setLoopCode] = useState("builtin.react-loop");
  const [rolePrompt, setRolePrompt] = useState("");
  const [groupName, setGroupName] = useState("");
  const { groups, create: createGroup } = useInternalAgentGroups(workspaceCode);
  const [avatarId, setAvatarId] = useState("robot");
  const [saving, setSaving] = useState(false);
  const [workspaceCodes, setWorkspaceCodes] = useState<string[]>([]);
  const [createdAgentCode, setCreatedAgentCode] = useState<string | null>(null);
  const definitionDisabled = saving || Boolean(createdAgentCode);
  const text = (key: string, fallback: string) => t(`pages:workspaceAgents.internal.definition.${key}`, { defaultValue: fallback });
  const codeValid = /^[a-z][a-z0-9_-]{0,63}$/.test(agentCode.trim());
  const codeHelp = text("codeHelp", "Use 1–64 characters, starting with a lowercase letter. Lowercase letters, numbers, hyphens and underscores are allowed. The code cannot be changed after creation.");

  async function create() {
    if (!codeValid || !name.trim() || saving) return;
    setSaving(true);
    try {
      const payload = { agentCode: agentCode.trim(), name: name.trim(), rolePrompt, loopCode, avatarId, groupName };
      // Keep the created identity if assignment fails so retry cannot create twice.
      if (!createdAgentCode) {
        if (groupName && !groups.includes(groupName)) await createGroup(groupName);
        if (getServiceDataSource("agent") === "api") {
          await createWorkspaceInternalAgent(workspaceCode, payload);
        } else {
          createWorkspaceInternalAgentMock(workspaceCode, payload);
        }
        setCreatedAgentCode(payload.agentCode);
        notifyInternalAgentRegistryChanged();
      }
      if (allowWorkspaceAssignment) await persistAgentWorkspaceGrants(createdAgentCode ?? payload.agentCode, workspaceCodes);
      await onCreated();
      setOpen(false);
      setAgentCode("");
      setName("");
      setRolePrompt("");
      setLoopCode("builtin.react-loop");
      setAvatarId("robot");
      setGroupName("");
      setWorkspaceCodes([]);
      setCreatedAgentCode(null);
      notifyInternalAgentRegistryChanged();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("createFailed", "Could not create agent"));
    } finally {
      setSaving(false);
    }
  }

  return <>
    <Button onClick={() => { setCreatedAgentCode(null); setAgentCode(""); setName(""); setRolePrompt(""); setAvatarId("robot"); setLoopCode("builtin.react-loop"); setWorkspaceCodes([]); setGroupName(""); setOpen(true); }}>{text("create", "New")}</Button>
    <Dialog disablePointerDismissal open={open} onOpenChange={(next, details) => { if (details.reason === "outside-press" || saving) { details.cancel(); return; } setOpen(next); }}>
      <DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-xl" closeLabel={text("close", "Close")} showCloseButton={!saving}>
        <DialogHeader>
          <DialogTitle>{text("title", "New internal agent")}</DialogTitle>
          <DialogDescription>{text("description", "Define its role, then select this agent when starting a conversation.")}</DialogDescription>
        </DialogHeader>
        <form className="space-y-4" onSubmit={(event) => { event.preventDefault(); void create(); }}>
          <AgentAvatarPicker value={avatarId} disabled={definitionDisabled} onChange={setAvatarId} />
          <Field label={text("code", "Code")} required hint={codeHelp} hintAriaLabel={text("codeHelpAria", "Agent code help")}
            error={agentCode.trim() && !codeValid ? text("codeInvalid", "Use 1–64 lowercase letters, numbers, hyphens or underscores, starting with a letter.") : undefined}>
            <TextInput autoFocus aria-label={text("code", "Code")} maxLength={64} required disabled={definitionDisabled} value={agentCode} onChange={(event) => setAgentCode(event.target.value)} aria-invalid={Boolean(agentCode.trim() && !codeValid)} />
          </Field>
          <Field label={text("name", "Name")} required>
            <TextInput aria-label={text("name", "Name")} maxLength={100} required disabled={definitionDisabled} value={name} onChange={(event) => setName(event.target.value)} />
          </Field>
          {allowWorkspaceAssignment ? <AgentWorkspaceField name={name} disabled={saving} selectedCodes={workspaceCodes} onSelectionChange={setWorkspaceCodes} /> : null}
          {createdAgentCode ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{t("pages:platformData.createdAssignmentPending")}</p> : null}
          <InternalAgentGroupField workspaceCode={workspaceCode} value={groupName} disabled={definitionDisabled} onChange={setGroupName} />
          <AgentLoopField value={loopCode} disabled={definitionDisabled} onChange={setLoopCode} />
          <AgentRolePromptField workspaceCode={workspaceCode} rows={8} disabled={definitionDisabled} value={rolePrompt} onChange={setRolePrompt} />
          <DialogFooter>
            <Button type="button" variant="outline" disabled={saving} onClick={() => setOpen(false)}>{text("cancel", "Cancel")}</Button>
            <Button type="submit" disabled={saving || !codeValid || !name.trim()}>{saving ? text("saving", "Saving...") : createdAgentCode ? t("pages:platformData.save") : text("create", "New")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </>;
}

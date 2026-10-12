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
import { AgentCandidateModelsField } from "./AgentCandidateModelsField";
import { AgentLoopField } from "./AgentLoopField";
import { AgentWorkspaceField } from "./AgentWorkspaceField";
import { persistAgentWorkspaceGrants } from "../hooks/agentWorkspaceGrants";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export function CreateInternalAgentDialog({ workspaceCode, onCreated, allowWorkspaceAssignment = false }: { workspaceCode: string; onCreated: () => Promise<void>; allowWorkspaceAssignment?: boolean }) {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const [open, setOpen] = useState(false);
  const [section, setSection] = useState("profile");
  const [agentCode, setAgentCode] = useState("");
  const [name, setName] = useState("");
  const [loopCode, setLoopCode] = useState("");
  const [rolePrompt, setRolePrompt] = useState("");
  const [groupName, setGroupName] = useState("");
  const { groups, create: createGroup } = useInternalAgentGroups(workspaceCode);
  const [candidateModelCodes, setCandidateModelCodes] = useState<string[]>([]);
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
      if (!loopCode) throw new Error(text("loopRequired", "Select an agent loop"));
      const payload = { agentCode: agentCode.trim(), name: name.trim(), rolePrompt, loopCode, avatarId, groupName, candidateModelCodes };
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
      // Workspace authorization is optional. Leaving the tab untouched keeps
      // the backend's default policy: the internal agent is available to all workspaces.
      if (allowWorkspaceAssignment && section === "workspace") {
        await persistAgentWorkspaceGrants(createdAgentCode ?? payload.agentCode, workspaceCodes);
      }
      await onCreated();
      setOpen(false);
      setAgentCode("");
      setName("");
      setRolePrompt("");
      setLoopCode("");
      setAvatarId("robot"); setCandidateModelCodes([]);
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
    <Button onClick={() => { setCreatedAgentCode(null); setAgentCode(""); setName(""); setRolePrompt(""); setAvatarId("robot"); setCandidateModelCodes([]); setLoopCode(""); setWorkspaceCodes([]); setGroupName(""); setSection("profile"); setOpen(true); }}>{text("create", "New")}</Button>
    <Dialog disablePointerDismissal open={open} onOpenChange={(next, details) => { if (details.reason === "outside-press" || saving) { details.cancel(); return; } setOpen(next); }}>
      <DialogContent className="flex h-[min(78dvh,48rem)] max-h-[90dvh] flex-col overflow-hidden sm:max-w-3xl" closeLabel={text("close", "Close")} showCloseButton={!saving}>
        <DialogHeader>
          <DialogTitle>{text("title", "New internal agent")}</DialogTitle>
        </DialogHeader>
        <form className="flex min-h-0 flex-1 flex-col" noValidate onSubmit={(event) => { event.preventDefault(); void create(); }}>
          <Tabs className="min-h-0 flex-1" value={section} onValueChange={(value) => { if (typeof value === "string") setSection(value); }}>
            <TabsList variant="line" className="w-full shrink-0 justify-start border-b border-[var(--color-outline-variant)]">
              <TabsTrigger value="profile" className="flex-none px-3">{text("tabs.profile", "Basic information")}</TabsTrigger>
              <TabsTrigger value="runtime" className="flex-none px-3">{text("tabs.runtime", "Runtime behavior")}</TabsTrigger>
              {allowWorkspaceAssignment ? <TabsTrigger value="workspace" className="flex-none px-3">{text("tabs.workspace", "Workspace access")}</TabsTrigger> : null}
            </TabsList>
            <TabsContent value="profile" keepMounted className="min-h-0 flex-1 space-y-4 overflow-y-auto px-1 py-3">
              <AgentAvatarPicker value={avatarId} disabled={definitionDisabled} onChange={setAvatarId} />
              <Field label={text("code", "Code")} required hint={codeHelp} hintAriaLabel={text("codeHelpAria", "Agent code help")}
                error={agentCode.trim() && !codeValid ? text("codeInvalid", "Use 1–64 lowercase letters, numbers, hyphens or underscores, starting with a letter.") : undefined}>
                <TextInput autoFocus aria-label={text("code", "Code")} maxLength={64} required disabled={definitionDisabled} value={agentCode} onChange={(event) => setAgentCode(event.target.value)} aria-invalid={Boolean(agentCode.trim() && !codeValid)} />
              </Field>
              <Field label={text("name", "Name")} required>
                <TextInput aria-label={text("name", "Name")} maxLength={100} required disabled={definitionDisabled} value={name} onChange={(event) => setName(event.target.value)} />
              </Field>
              <InternalAgentGroupField workspaceCode={workspaceCode} value={groupName} disabled={definitionDisabled} onChange={setGroupName} />
            </TabsContent>
            <TabsContent value="runtime" keepMounted className="min-h-0 flex-1 space-y-4 overflow-y-auto px-1 py-3">
              <AgentLoopField value={loopCode} disabled={definitionDisabled} onChange={setLoopCode} />
              <AgentCandidateModelsField workspaceCode={workspaceCode} value={candidateModelCodes} onChange={setCandidateModelCodes} disabled={definitionDisabled} />
              <AgentRolePromptField rows={8} disabled={definitionDisabled} value={rolePrompt} onChange={setRolePrompt} />
            </TabsContent>
            {allowWorkspaceAssignment ? <TabsContent value="workspace" keepMounted className="min-h-0 flex-1 space-y-4 overflow-y-auto px-1 py-3">
              <AgentWorkspaceField name={name} disabled={saving} selectedCodes={workspaceCodes} onSelectionChange={setWorkspaceCodes} />
              {createdAgentCode ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{t("pages:platformData.createdAssignmentPending")}</p> : null}
            </TabsContent> : null}
          </Tabs>
          <DialogFooter>
            <Button type="button" variant="outline" disabled={saving} onClick={() => setOpen(false)}>{text("cancel", "Cancel")}</Button>
            <Button type="submit" disabled={saving || !codeValid || !name.trim() || !loopCode}>{saving ? text("saving", "Saving...") : createdAgentCode ? t("pages:platformData.save") : text("create", "New")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </>;
}

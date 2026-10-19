import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";
import { createWorkspaceInternalAgent, patchWorkspaceInternalAgentDefinition } from "@/domain/agent/api";
import { createWorkspaceInternalAgentMock, patchWorkspaceInternalAgentDefinitionMock } from "@/domain/agent/mock";
import { notifyInternalAgentRegistryChanged } from "@/domain/agent/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useInternalAgentGroups } from "../hooks/useInternalAgentGroups";
import { DEFAULT_REACT_LOOP_CODE } from "../model/builtinLoops";
import { AgentWorkspaceField } from "./AgentWorkspaceField";
import { AgentActivationSwitch } from "./AgentActivationSwitch";
import { persistAgentWorkspaceGrants } from "../hooks/agentWorkspaceGrants";
import { INTERNAL_AGENT_DIALOG_CLASS, INTERNAL_AGENT_SECTIONS, InternalAgentProfileFields, InternalAgentRuntimeFields } from "./InternalAgentFormFields";
import { AgentResourcesEditor, type AgentResourcesEditorHandle } from "./AgentResourcesEditor";

const initialDraft = () => ({ name: "", description: "", rolePrompt: "", avatarId: "robot", groupName: "", loopCode: DEFAULT_REACT_LOOP_CODE, candidateModelCodes: [] as string[], enabled: true });
export function CreateInternalAgentDialog({ workspaceCode, onCreated, allowWorkspaceAssignment = false }: { workspaceCode: string; onCreated: () => Promise<void>; allowWorkspaceAssignment?: boolean }) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [open, setOpen] = useState(false);
  const [section, setSection] = useState("profile");
  const [agentCode, setAgentCode] = useState("");
  const [draft, setDraft] = useState(initialDraft);
  const [saving, setSaving] = useState(false);
  const savingRef = useRef(false);
  const [createdCode, setCreatedCode] = useState<string | null>(null);
  const createdCodeRef = useRef<string | null>(null);
  const enabledBaseline = useRef(true);
  const [workspaceCodes, setWorkspaceCodes] = useState<string[] | undefined>();
  const savedWorkspaces = useRef<string | undefined>(undefined);
  const resources = useRef<AgentResourcesEditorHandle>(null);
  const { groups, create: createGroup } = useInternalAgentGroups(workspaceCode);
  const patch = (change: Partial<ReturnType<typeof initialDraft>>) => setDraft((current) => ({ ...current, ...change }));
  const text = (key: string, fallback: string) => t(`workspaceAgents.internal.definition.${key}`, { defaultValue: fallback });
  const codeValid = /^[a-z][a-z0-9_-]{0,63}$/.test(agentCode.trim());
  async function create() {
    if (!codeValid || !draft.name.trim() || !draft.loopCode || savingRef.current) return;
    savingRef.current = true; setSaving(true);
    try {
      resources.current?.validate();
      const code = createdCodeRef.current ?? agentCode.trim();
      const api = getServiceDataSource("agent") === "api";
      if (!createdCodeRef.current) {
        if (draft.groupName && !groups.includes(draft.groupName)) await createGroup(draft.groupName);
        const { enabled: _enabled, ...definition } = draft;
        const payload = { ...definition, agentCode: code, name: draft.name.trim(), description: draft.description.trim() };
        if (api) await createWorkspaceInternalAgent(workspaceCode, payload);
        else createWorkspaceInternalAgentMock(workspaceCode, payload);
        createdCodeRef.current = code; setCreatedCode(code);
        notifyInternalAgentRegistryChanged();
      }
      if (draft.enabled !== enabledBaseline.current) {
        if (api) await patchWorkspaceInternalAgentDefinition(workspaceCode, code, { enabled: draft.enabled });
        else patchWorkspaceInternalAgentDefinitionMock(workspaceCode, code, { enabled: draft.enabled });
        enabledBaseline.current = draft.enabled;
      }
      await resources.current?.save(code);
      if (allowWorkspaceAssignment && workspaceCodes !== undefined && savedWorkspaces.current !== JSON.stringify(workspaceCodes)) {
        await persistAgentWorkspaceGrants(code, workspaceCodes);
        savedWorkspaces.current = JSON.stringify(workspaceCodes);
      }
      await onCreated();
      setOpen(false); notifyInternalAgentRegistryChanged();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : text("createFailed", "Could not create agent"));
    } finally { savingRef.current = false; setSaving(false); }
  }
  return <>
    <Button onClick={() => { setAgentCode(""); setDraft(initialDraft()); setSection("profile"); setWorkspaceCodes(undefined); savedWorkspaces.current = undefined; enabledBaseline.current = true; createdCodeRef.current = null; setCreatedCode(null); setOpen(true); }}>{text("create", "New")}</Button>
    <Dialog disablePointerDismissal open={open} onOpenChange={(next, details) => { if (details.reason === "outside-press" || savingRef.current) { details.cancel(); return; } setOpen(next); }}>
      <DialogContent className={INTERNAL_AGENT_DIALOG_CLASS} closeLabel={text("close", "Close")} showCloseButton={!saving}>
        <DialogHeader><div className="flex items-center justify-between gap-4 pr-8"><DialogTitle>{text("title", "New internal agent")}</DialogTitle>{allowWorkspaceAssignment ? <AgentActivationSwitch checked={draft.enabled} disabled={saving} onChange={(enabled) => patch({ enabled })} /> : null}</div></DialogHeader>
        <form className="flex min-h-0 flex-1 flex-col" noValidate onSubmit={(event) => { event.preventDefault(); void create(); }}>
          <Tabs className="min-h-0 flex-1" value={section} onValueChange={(value) => { if (typeof value === "string") setSection(value); }}>
            <TabsList variant="line" className="w-full shrink-0 justify-start border-b border-[var(--color-outline-variant)]">
              {INTERNAL_AGENT_SECTIONS.map(([value, label]) => <TabsTrigger key={value} value={value} className="flex-none px-3">{text(`tabs.${value}`, label)}</TabsTrigger>)}
              {allowWorkspaceAssignment ? <TabsTrigger value="workspace" className="flex-none px-3">{text("tabs.workspace", "Workspace")}</TabsTrigger> : null}
            </TabsList>
            <TabsContent value="profile" keepMounted className="tron-scrollbar min-h-0 flex-1 space-y-3 overflow-y-auto px-1 py-3"><InternalAgentProfileFields code={agentCode} onCodeChange={setAgentCode} draft={draft} onChange={patch} workspaceCode={workspaceCode} disabled={saving || !!createdCode} /></TabsContent>
            <TabsContent value="runtime" keepMounted className="tron-scrollbar min-h-0 flex-1 space-y-3 overflow-y-auto px-1 py-3"><InternalAgentRuntimeFields workspaceCode={workspaceCode} loopCode={draft.loopCode} candidateModelCodes={draft.candidateModelCodes} disabled={saving || !!createdCode} onChange={patch} /></TabsContent>
            <AgentResourcesEditor ref={resources} agentName={draft.name} workspaceCode={workspaceCode} disabled={saving} />
            {allowWorkspaceAssignment ? <TabsContent value="workspace" keepMounted className="tron-scrollbar min-h-0 flex-1 space-y-3 overflow-y-auto px-1 py-3"><AgentWorkspaceField name={draft.name} disabled={saving} selectedCodes={workspaceCodes} onSelectionChange={setWorkspaceCodes} /></TabsContent> : null}
          </Tabs>
          {createdCode ? <p role="status" className="text-sm text-[var(--color-on-surface-variant)]">{t("platformData.createdAssignmentPending")}</p> : null}
          <DialogFooter><Button type="button" variant="outline" disabled={saving} onClick={() => setOpen(false)}>{text("cancel", "Cancel")}</Button><Button type="submit" disabled={saving || !codeValid || !draft.name.trim() || !draft.loopCode}>{saving ? text("saving", "Saving...") : createdCode ? t("pages:platformData.save") : text("create", "New")}</Button></DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </>;
}

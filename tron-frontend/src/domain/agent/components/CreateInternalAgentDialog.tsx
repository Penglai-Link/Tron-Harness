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

export function CreateInternalAgentDialog({ workspaceCode, onCreated }: { workspaceCode: string; onCreated: () => Promise<void> }) {
  const { t } = useTranslation(["pages"]);
  const toast = useToast();
  const [open, setOpen] = useState(false);
  const [agentCode, setAgentCode] = useState("");
  const [name, setName] = useState("");
  const [rolePrompt, setRolePrompt] = useState("");
  const [saving, setSaving] = useState(false);
  const text = (key: string, fallback: string) => t(`pages:workspaceAgents.internal.definition.${key}`, { defaultValue: fallback });
  const codeValid = /^[a-z][a-z0-9_-]{0,63}$/.test(agentCode.trim());
  const codeHelp = text("codeHelp", "Use 1–64 characters, starting with a lowercase letter. Lowercase letters, numbers, hyphens and underscores are allowed. The code cannot be changed after creation.");

  async function create() {
    if (!codeValid || !name.trim() || saving) return;
    setSaving(true);
    try {
      const payload = { agentCode: agentCode.trim(), name: name.trim(), rolePrompt };
      if (getServiceDataSource("agent") === "api") {
        await createWorkspaceInternalAgent(workspaceCode, payload);
      } else {
        createWorkspaceInternalAgentMock(workspaceCode, payload);
      }
      setOpen(false);
      setAgentCode("");
      setName("");
      setRolePrompt("");
      notifyInternalAgentRegistryChanged();
      await onCreated();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : text("createFailed", "Could not create agent"));
    } finally {
      setSaving(false);
    }
  }

  return <>
    <Button onClick={() => setOpen(true)}>{text("create", "New")}</Button>
    <Dialog open={open} onOpenChange={(next) => { if (!saving) setOpen(next); }}>
      <DialogContent className="sm:max-w-xl" closeLabel={text("close", "Close")} showCloseButton={!saving}>
        <DialogHeader>
          <DialogTitle>{text("title", "New internal agent")}</DialogTitle>
          <DialogDescription>{text("description", "Define its role, then select this agent when starting a conversation.")}</DialogDescription>
        </DialogHeader>
        <form className="space-y-4" onSubmit={(event) => { event.preventDefault(); void create(); }}>
          <Field label={text("code", "Code")} required hint={codeHelp} hintAriaLabel={text("codeHelpAria", "Agent code help")}
            error={agentCode.trim() && !codeValid ? text("codeInvalid", "Use 1–64 lowercase letters, numbers, hyphens or underscores, starting with a letter.") : undefined}>
            <TextInput autoFocus aria-label={text("code", "Code")} maxLength={64} required disabled={saving} value={agentCode} onChange={(event) => setAgentCode(event.target.value)} aria-invalid={Boolean(agentCode.trim() && !codeValid)} />
          </Field>
          <Field label={text("name", "Name")} required>
            <TextInput aria-label={text("name", "Name")} maxLength={100} required disabled={saving} value={name} onChange={(event) => setName(event.target.value)} />
          </Field>
          <AgentRolePromptField workspaceCode={workspaceCode} rows={8} disabled={saving} value={rolePrompt} onChange={setRolePrompt} />
          <DialogFooter>
            <Button type="button" variant="outline" disabled={saving} onClick={() => setOpen(false)}>{text("cancel", "Cancel")}</Button>
            <Button type="submit" disabled={saving || !codeValid || !name.trim()}>{saving ? text("saving", "Saving...") : text("create", "New")}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </>;
}

import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Field, TextInput } from "@/components/forms";
import { AgentAvatarPicker } from "./AgentAvatarPicker";
import { AgentDescriptionField } from "./AgentDescriptionField";
import { AgentRolePromptField } from "./AgentRolePromptField";
import { InternalAgentGroupField } from "./InternalAgentGroupField";
import { AgentLoopField } from "./AgentLoopField";
import { AgentCandidateModelsField } from "./AgentCandidateModelsField";

export const INTERNAL_AGENT_DIALOG_CLASS = "flex h-[min(78dvh,48rem)] max-h-[90dvh] flex-col overflow-hidden sm:max-w-4xl";
export const INTERNAL_AGENT_SECTIONS = [["profile", "Basic"], ["runtime", "Runtime"], ["resources", "Capabilities"], ["advanced", "Advanced"]] as const;
export interface InternalAgentProfileDraft { name: string; description: string; avatarId: string; rolePrompt: string; groupName: string }

export function InternalAgentProfileFields({ code, onCodeChange, draft, onChange, workspaceCode, disabled, avatar }: {
  code: string; onCodeChange?: (code: string) => void; draft: InternalAgentProfileDraft;
  onChange: (patch: Partial<InternalAgentProfileDraft>) => void; workspaceCode: string; disabled: boolean; avatar?: ReactNode;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string, fallback: string) => t(`workspaceAgents.internal.definition.${key}`, { defaultValue: fallback });
  const codeInvalid = !!onCodeChange && !!code.trim() && !/^[a-z][a-z0-9_-]{0,63}$/.test(code.trim());
  return <div className="space-y-3">
    {avatar ?? <AgentAvatarPicker value={draft.avatarId} disabled={disabled} onChange={(avatarId) => onChange({ avatarId })} />}
    <Field label={text("code", "Code")} required={!!onCodeChange}
      hint={text("codeHelp", "Use 1–64 characters, starting with a lowercase letter. Lowercase letters, numbers, hyphens and underscores are allowed. The code cannot be changed after creation.")}
      hintAriaLabel={text("codeHelpAria", "Agent code help")}
      error={codeInvalid ? text("codeInvalid", "Use 1–64 lowercase letters, numbers, hyphens or underscores, starting with a letter.") : undefined}>
      <TextInput aria-label={text("code", "Code")} maxLength={64} required={!!onCodeChange} readOnly={!onCodeChange} disabled={disabled} value={code} onChange={(event) => onCodeChange?.(event.target.value)} aria-invalid={codeInvalid} />
    </Field>
    <Field label={text("name", "Name")} required><TextInput aria-label={text("name", "Name")} maxLength={100} disabled={disabled} value={draft.name} onChange={(event) => onChange({ name: event.target.value })} /></Field>
    <AgentDescriptionField value={draft.description} disabled={disabled} onChange={(description) => onChange({ description })} />
    <AgentRolePromptField rows={8} value={draft.rolePrompt} disabled={disabled} onChange={(rolePrompt) => onChange({ rolePrompt })} />
    <InternalAgentGroupField workspaceCode={workspaceCode} value={draft.groupName} disabled={disabled} onChange={(groupName) => onChange({ groupName })} />
  </div>;
}

export function InternalAgentRuntimeFields({ workspaceCode, loopCode, candidateModelCodes, disabled, onChange }: {
  workspaceCode: string; loopCode: string; candidateModelCodes: string[]; disabled: boolean;
  onChange: (patch: { loopCode?: string; candidateModelCodes?: string[] }) => void;
}) {
  return <>
    <AgentLoopField value={loopCode} disabled={disabled} onChange={(value) => onChange({ loopCode: value })} />
    <AgentCandidateModelsField workspaceCode={workspaceCode} value={candidateModelCodes} disabled={disabled} onChange={(value) => onChange({ candidateModelCodes: value })} />
  </>;
}

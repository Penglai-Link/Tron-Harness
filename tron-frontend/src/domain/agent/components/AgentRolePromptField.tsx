import { useTranslation } from "react-i18next";
import { Field, TextArea } from "@/components/forms";
import { RolePromptTemplatePicker } from "@/domain/prompt/components/RolePromptTemplatePicker";
import { PLATFORM_INTERNAL_AGENT_SCOPE } from "@/domain/agent/model";

export function AgentRolePromptField({ workspaceCode, value, disabled, rows = 4, onChange }: { workspaceCode: string; value: string; disabled?: boolean; rows?: number; onChange: (value: string) => void }) {
  const { t } = useTranslation(["pages"]);
  const label = t("pages:workspaceAgents.internal.definition.rolePrompt", { defaultValue: "Role prompt" });
  const platform = workspaceCode === PLATFORM_INTERNAL_AGENT_SCOPE;
  return (
    <Field className="w-full" label={label}
      hint={t("pages:workspaceAgents.internal.definition.roleHelp", { defaultValue: "Sets this agent's role and instructions. Changes apply to its next run." })}
      hintAriaLabel={t("pages:workspaceAgents.internal.definition.roleHelpAria", { defaultValue: "Role prompt help" })}
      actions={<RolePromptTemplatePicker scopeKind={platform ? "PLATFORM" : "WORKSPACE"} workspaceCode={platform ? undefined : workspaceCode} value={value} disabled={disabled} onApply={onChange} />}
    >
      <TextArea aria-label={label} rows={rows} disabled={disabled} value={value} onChange={(event) => onChange(event.target.value)} />
    </Field>
  );
}

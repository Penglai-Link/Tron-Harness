import { useTranslation } from "react-i18next";
import { Field, TextArea } from "@/components/forms";

export function AgentRolePromptField({ value, disabled, required = false, rows = 4, onChange }: { value: string; disabled?: boolean; required?: boolean; rows?: number; onChange: (value: string) => void }) {
  const { t } = useTranslation(["pages"]);
  const label = t("pages:workspaceAgents.internal.definition.rolePrompt", { defaultValue: "Role prompt" });
  return (
    <Field className="w-full" label={label} required={required}
      hint={t("pages:workspaceAgents.internal.definition.roleHelp", { defaultValue: "Sets this agent's role and instructions. Changes apply to its next run." })}
      hintAriaLabel={t("pages:workspaceAgents.internal.definition.roleHelpAria", { defaultValue: "Role prompt help" })}
    >
      <TextArea aria-label={label} rows={rows} required={required} disabled={disabled} value={value} onChange={(event) => onChange(event.target.value)} />
    </Field>
  );
}

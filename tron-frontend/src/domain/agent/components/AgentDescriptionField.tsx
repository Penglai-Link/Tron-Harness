import { useTranslation } from "react-i18next";
import { Field, TextArea } from "@/components/forms";

export function AgentDescriptionField({ value, disabled, onChange }: {
  value: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  const { t } = useTranslation("pages");
  const prefix = "workspaceAgents.internal.definition.";
  return <Field label={t(prefix + "shortDescription")} hint={t(prefix + "descriptionHelp")} hintAriaLabel={t(prefix + "descriptionHelpAria")}>
    <TextArea aria-label={t(prefix + "shortDescription")} rows={2} maxLength={200} disabled={disabled}
      placeholder={t(prefix + "descriptionPlaceholder")} value={value} onChange={(event) => onChange(event.target.value)} />
  </Field>;
}

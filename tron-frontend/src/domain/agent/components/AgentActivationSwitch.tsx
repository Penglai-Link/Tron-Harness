import { useTranslation } from "react-i18next";
import { Switch } from "@/components/ui/switch";
export function AgentActivationSwitch({ checked, disabled, onChange }: { checked: boolean; disabled?: boolean; onChange: (enabled: boolean) => void }) {
  const { t } = useTranslation("pages");
  return <label className="flex shrink-0 items-center gap-2 text-sm">
    <span>{t(checked ? "agentActivation.active" : "agentActivation.disabled")}</span>
    <Switch checked={checked} disabled={disabled} onCheckedChange={onChange} aria-label={t("agentActivation.active")} />
  </label>;
}

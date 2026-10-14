import type { ReactNode } from "react";
import { Ban } from "lucide-react";
import { useTranslation } from "react-i18next";
export function AgentAvailabilityAvatar({ enabled, children }: { enabled: boolean; children: ReactNode }) {
  const { t } = useTranslation("pages");
  return <span className="relative inline-flex shrink-0">
    <span className={enabled ? undefined : "grayscale opacity-40"}>{children}</span>
    {!enabled && <span role="img" aria-label={t("agentActivation.disabled")} title={t("agentActivation.disabled")} className="absolute bottom-0 left-0 rounded-full border border-[var(--color-outline)] bg-[var(--color-surface-container)] p-1 text-[var(--color-on-surface)]"><Ban size={16} /></span>}
  </span>;
}

import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { InfoTooltip } from "@/components/info-tooltip";

export function ContextBlockHelp({ name, description, children }: { name: string; description?: string; children?: ReactNode }) {
  const { t } = useTranslation("pages");
  return <InfoTooltip ariaLabel={name} align="start">
    <div className="max-w-[22rem] space-y-2 whitespace-normal text-xs leading-5">
      <p>{description || t("chat.shortTermMemory.contextPanel.descriptionUnavailable")}</p>
      {children}
    </div>
  </InfoTooltip>;
}

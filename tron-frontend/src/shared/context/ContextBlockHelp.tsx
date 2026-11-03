import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { InfoTooltip } from "@/components/info-tooltip";
import type { ContextBlockUpdateMode } from "./contextBlockPresentation";

export function ContextBlockHelp({ name, description, updateMode = "Unspecified", children }: { name: string; description?: string; updateMode?: ContextBlockUpdateMode; children?: ReactNode }) {
  const { t } = useTranslation("pages");
  return <InfoTooltip ariaLabel={name} align="start">
    <div className="max-w-[22rem] space-y-2 whitespace-normal text-xs leading-5">
      <p>{description || t("chat.shortTermMemory.contextPanel.descriptionUnavailable")}</p>
      <p>{t(`contextLayout.updateModeDescriptions.${updateMode}`)}</p>
      {children}
    </div>
  </InfoTooltip>;
}

import { useTranslation } from "react-i18next";
import { InfoTooltip } from "@/components/info-tooltip";
import type { ContextCompressionMetadata } from "./contextBlockPresentation";

export function ContextCompressionLabel({ name, compression, unavailable = false }: {
  name: string;
  compression?: ContextCompressionMetadata | null;
  unavailable?: boolean;
}) {
  const { t } = useTranslation("pages");
  const declared = !unavailable && compression?.id && compression.displayName;
  const label = unavailable ? t("contextLayout.compressionUnavailable")
    : declared ? compression.displayName : t("contextLayout.compressionUnspecified");
  const description = unavailable ? t("contextLayout.compressionUnavailableHelp")
    : declared ? compression.description : t("contextLayout.compressionUnspecifiedHelp");
  return <span className="flex min-w-0 items-center justify-center gap-1 text-xs text-[var(--color-on-surface-variant)]">
    <span className="truncate" title={label}>{label}</span>
    {description ? <InfoTooltip ariaLabel={t("contextLayout.compressionFor", { name })}>
      <p className="max-w-[22rem] whitespace-normal text-xs leading-5">{description}</p>
    </InfoTooltip> : null}
  </span>;
}

import { useTranslation } from "react-i18next";
import type { ImageAnalysisSummary } from "../model/KnowledgeLibraryModel";

export function ImageAnalysisStatus({ summary }: { summary?: ImageAnalysisSummary | null }) {
  const { t } = useTranslation("pages");
  if (!summary?.imageCount) return null;
  return <div className="mt-2 space-y-1 text-xs text-[var(--color-text-secondary)]">
    {summary.originalsOnly > 0 ? <p>{t("knowledgeLibrary.imagesOriginalsOnly", { count: summary.originalsOnly })}</p> : null}
    {summary.withSourceAnnotations > 0 ? <p>{t("knowledgeLibrary.imagesSourceAnnotations", { count: summary.withSourceAnnotations })}</p> : null}
    {summary.withGeneratedDescriptions > 0 ? <p>{t("knowledgeLibrary.imagesGeneratedDescriptions", { count: summary.withGeneratedDescriptions })}</p> : null}
    {summary.withUnattributedBodyText > 0 ? <p>{t("knowledgeLibrary.imagesBodyText", { count: summary.withUnattributedBodyText })}</p> : null}
    {summary.generationMode === "disabled_by_tier" && (!summary.supplementStatus || summary.supplementStatus === "disabled") ? <p>{t("knowledgeLibrary.imageGenerationDisabled")}</p> : null}
    {summary.supplementStatus === "pending" ? <p>{t("knowledgeLibrary.imageSupplementPending")}</p> : null}
  </div>;
}

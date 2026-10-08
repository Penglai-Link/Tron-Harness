import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import type { ImageAnalysisSummary } from "../model/KnowledgeLibraryModel";

export function imageAnalysisMessages(summary: ImageAnalysisSummary | null | undefined, t: TFunction<"pages">): string[] {
  if (!summary?.imageCount) return [];
  const messages: string[] = [];
  if (summary.originalsOnly > 0) messages.push(t("knowledgeLibrary.imagesOriginalsOnly", { count: summary.originalsOnly }));
  if (summary.withSourceAnnotations > 0) messages.push(t("knowledgeLibrary.imagesSourceAnnotations", { count: summary.withSourceAnnotations }));
  if (summary.withGeneratedDescriptions > 0) messages.push(t("knowledgeLibrary.imagesGeneratedDescriptions", { count: summary.withGeneratedDescriptions }));
  if (summary.withUnattributedBodyText > 0) messages.push(t("knowledgeLibrary.imagesBodyText", { count: summary.withUnattributedBodyText }));
  if (summary.generationMode === "disabled_by_tier" && (!summary.supplementStatus || summary.supplementStatus === "disabled")) messages.push(t("knowledgeLibrary.imageGenerationDisabled"));
  if (summary.supplementStatus === "pending") messages.push(t("knowledgeLibrary.imageSupplementPending"));
  return messages;
}

export function ImageAnalysisStatus({ summary }: { summary?: ImageAnalysisSummary | null }) {
  const { t } = useTranslation("pages");
  const messages = imageAnalysisMessages(summary, t);
  if (!messages.length) return null;
  return <div className="mt-2 space-y-1 text-xs text-[var(--color-text-secondary)]">
    {messages.map((message) => <p key={message}>{message}</p>)}
  </div>;
}

import { TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";
import { formatIngestionWaitReason } from "../model/ingestionWaitReason";
import { formatKnowledgeDocumentError } from "../model/knowledgeDocumentError";
import { imageAnalysisMessages } from "./ImageAnalysisStatus";

export function KnowledgeDocumentName({ document }: { document: KnowledgeDocument }) {
  const { t } = useTranslation("pages");
  const warnings = (document.warnings ?? [])
    .filter((warning) => !document.imageAnalysis || warning !== "图片原件和位置已保留；图片语义描述需要单独验证")
    .map((warning) => {
      if (warning === "图片原件和位置已保留；图片语义描述需要单独验证") return t("knowledgeLibrary.imageNotice");
      if (warning === "Markdown图片链接原样保留；仅已有资源引用可供授权Agent读取，不自动下载外部图片") return t("knowledgeLibrary.markdownImageNotice");
      return warning;
    });
  const notices = [...warnings, ...imageAnalysisMessages(document.imageAnalysis, t)];
  const error = document.error
    ? document.waitReason ? formatIngestionWaitReason(document.waitReason, t) : formatKnowledgeDocumentError(document.error, t)
    : null;

  return <div className="flex items-start gap-2">
    {error || notices.length ? <Tooltip>
      <TooltipTrigger
        aria-label={t("knowledgeLibrary.documentWarnings", { name: document.fileName })}
        render={<button type="button" className="mt-0.5 shrink-0 cursor-help rounded-sm text-[var(--color-warning)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring" />}
      >
        <TriangleAlert size={16} aria-hidden="true" />
      </TooltipTrigger>
      <TooltipContent side="top" align="start" className="max-w-sm whitespace-normal break-words">
        <div className="space-y-2 text-left">
          {error ? <p role="alert">{error}</p> : null}
          {notices.map((notice, index) => <p key={index}>{notice}</p>)}
        </div>
      </TooltipContent>
    </Tooltip> : null}
    <p className="min-w-0 break-words whitespace-normal font-medium">{document.fileName}</p>
  </div>;
}

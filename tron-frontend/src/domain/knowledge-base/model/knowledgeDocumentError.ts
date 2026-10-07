import type { TFunction } from "i18next";

/** Parse errors keep the service's diagnostic, with localized UI framing. */
export function formatKnowledgeDocumentError(error: string | null | undefined, t: TFunction<"pages">): string {
  if (!error) return "";
  const marker = "MINERU_PARSE_FAILED:";
  const start = error.indexOf(marker);
  if (start < 0) return error;
  try {
    const detail: unknown = JSON.parse(error.slice(start + marker.length));
    if (!detail || typeof detail !== "object" || !("jobId" in detail) || typeof detail.jobId !== "string") return error;
    const reason = "reason" in detail && typeof detail.reason === "string" && detail.reason.trim()
      ? detail.reason : t("knowledgeLibrary.parseFailureUnknown");
    return t("knowledgeLibrary.parseFailure", { job: detail.jobId, reason });
  } catch {
    return error;
  }
}

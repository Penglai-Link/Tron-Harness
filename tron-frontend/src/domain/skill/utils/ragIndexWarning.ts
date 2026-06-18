import type {
  SkillRagIndexBatchSummary,
  SkillRagIndexSummary,
} from "@/domain/skill/model";

type RagIndexResult = SkillRagIndexBatchSummary | SkillRagIndexSummary | null | undefined;

function joinWarnings(warnings: string[]) {
  const messages = warnings.map((warning) => warning.trim()).filter(Boolean);
  return messages.length ? messages.join("; ") : null;
}

export function skillIndexWarningMessage(
  ragIndex: RagIndexResult,
  fallback = "Skill RAG index completed with warnings",
) {
  if (!ragIndex) return null;

  if ("failedCount" in ragIndex) {
    const hasWarning = ragIndex.failedCount > 0 || ragIndex.warningCount > 0;
    if (!hasWarning) return null;
    return joinWarnings(ragIndex.warnings) ?? fallback;
  }

  const hasWarning =
    ragIndex.vectorStatus === "FAILED"
    || ragIndex.graphStatus === "FAILED"
    || ragIndex.vectorStatus === "WARNING"
    || ragIndex.graphStatus === "WARNING"
    || ragIndex.warnings.length > 0;
  if (!hasWarning) return null;
  return joinWarnings(ragIndex.warnings) ?? ragIndex.lastError ?? fallback;
}

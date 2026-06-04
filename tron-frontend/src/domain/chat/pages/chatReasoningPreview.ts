export type ReasoningPreviewToolCard = {
  started: boolean;
};

export function truncateLabel(value: string, maxLength = 64) {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) return normalized;
  return `${normalized.slice(0, Math.max(0, maxLength - 3)).trimEnd()}...`;
}

export function reasoningPreviewLabel(value: string, maxLength = 168) {
  return truncateLabel(value.replace(/[#>*_`~-]+/g, " "), maxLength);
}

export function reasoningFirstClause(value: string) {
  const normalized = value
    .replace(/[#>*_`~-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!normalized) return "";
  const stripped = normalized
    .replace(/^the user (?:wants|asked|is asking)[^.?!。！？]*[.?!。！？]\s*/i, "")
    .replace(/^i should\s+/i, "")
    .replace(/^i need to\s+/i, "")
    .replace(/^let me\s+/i, "");
  const sentenceBoundary = stripped.search(/[.?!。！？]/);
  if (sentenceBoundary >= 0) {
    return stripped.slice(0, sentenceBoundary + 1).trim();
  }
  return stripped || normalized;
}

export function deriveReasoningPreview(detail: string, toolCards: ReasoningPreviewToolCard[]) {
  const clause = reasoningFirstClause(detail);
  const toolCount = toolCards.filter((toolCard) => toolCard.started).length;
  const previewSource = clause.length >= 20 ? clause : detail;
  const preview = reasoningPreviewLabel(previewSource, 120);
  if (!toolCount) return preview;
  const summary = preview
    ? `${preview} Used ${toolCount} ${toolCount === 1 ? "tool" : "tools"}.`
    : `Used ${toolCount} ${toolCount === 1 ? "tool" : "tools"} before replying.`;
  return truncateLabel(summary, 120);
}

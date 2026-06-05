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

function normalizeReasoningText(value: string) {
  const normalized = value
    .replace(/[#>*_`~-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  if (!normalized) return "";
  return normalized
    .replace(/^the user (?:wants|asked|is asking)[^.?!。！？]*[.?!。！？]\s*/i, "")
    .replace(/^i should\s+/i, "")
    .replace(/^i need to\s+/i, "")
    .replace(/^let me\s+/i, "");
}

function splitReasoningSentences(value: string) {
  return value.match(/[^.?!。！？]+[.?!。！？]?/g)?.map((sentence) => sentence.trim()).filter(Boolean) ?? [];
}

export function reasoningLastSentence(value: string) {
  const normalized = normalizeReasoningText(value);
  if (!normalized) return "";
  const sentences = splitReasoningSentences(normalized);
  if (sentences.length === 0) return normalized;
  return sentences[sentences.length - 1] || normalized;
}

export function reasoningFirstClause(value: string) {
  const normalized = normalizeReasoningText(value);
  if (!normalized) return "";
  const stripped = normalized;
  const sentenceBoundary = stripped.search(/[.?!。！？]/);
  if (sentenceBoundary >= 0) {
    return stripped.slice(0, sentenceBoundary + 1).trim();
  }
  return stripped || normalized;
}

export function deriveReasoningPreview(detail: string, toolCards: ReasoningPreviewToolCard[]) {
  const sentence = reasoningLastSentence(detail);
  const toolCount = toolCards.filter((toolCard) => toolCard.started).length;
  const previewSource = sentence || detail;
  const preview = reasoningPreviewLabel(previewSource, 120);
  if (!toolCount) return preview;
  const summary = preview
    ? `${preview} Used ${toolCount} ${toolCount === 1 ? "tool" : "tools"}.`
    : `Used ${toolCount} ${toolCount === 1 ? "tool" : "tools"} before replying.`;
  return truncateLabel(summary, 120);
}

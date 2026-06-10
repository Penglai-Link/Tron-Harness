export function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function sessionAgentTagLabel(agentCode: string | null | undefined) {
  const normalized = agentCode?.trim().toLowerCase();
  if (!normalized || normalized === "main-agent") return "MAIN";
  return normalized.toUpperCase();
}

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

export function deriveReasoningPreview(detail: string) {
  const sentence = reasoningLastSentence(detail);
  const previewSource = sentence || detail;
  return reasoningPreviewLabel(previewSource, 120);
}

export function truncateLabel(value: string, maxLength = 64) {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLength) return normalized;
  return `${normalized.slice(0, Math.max(0, maxLength - 3)).trimEnd()}...`;
}

export function reasoningPreviewLabel(value: string, maxLength = 168) {
  return truncateLabel(value.replace(/[#>*_`~-]+/g, " "), maxLength);
}

export function deriveReasoningPreview(summary: string) {
  return reasoningPreviewLabel(summary, 120);
}

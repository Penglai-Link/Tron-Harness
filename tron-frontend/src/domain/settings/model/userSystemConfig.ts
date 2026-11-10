// This is a product-facing allowlist, not a YAML editor. New backend parameters
// stay hidden until they have a user-facing purpose, label and help text.
export const userSystemConfigGroups = [
  { id: "systemAgents", paths: [
    "systemAgents.codexCommand", "systemAgents.opencodeCommand", "systemAgents.dshCommand",
  ] },
  { id: "mineru", paths: [
    "mineru.visualContextEnabled", "mineru.imageDescription.enabled", "mineru.imageDescription.modelCode",
  ] },
  { id: "ragRetrieval", paths: [
    "ragRetrieval.minVectorSimilarity", "ragRetrieval.minRerankScore",
  ] },
  { id: "shortTermMemory", paths: [
    "shortTermMemory.compaction.autoEnabled", "shortTermMemory.compaction.triggerRatio", "shortTermMemory.goalLoop.maxRuns",
  ] },
] as const;

export function isRetrievalThreshold(path: string) {
  return path === "ragRetrieval.minVectorSimilarity" || path === "ragRetrieval.minRerankScore";
}

export function retrievalThresholdError(path: string, draft: string) {
  if (!isRetrievalThreshold(path) || draft === "") return null;
  const value = Number(draft);
  if (!/^-?(?:\d+(?:\.\d+)?|\.\d+)$/.test(draft) || !Number.isFinite(value)) return "invalidNumber";
  if (path === "ragRetrieval.minVectorSimilarity" && (value < 0 || value > 1)) return "unitInterval";
  if (value < 0) return "nonnegative";
  return null;
}

export const externalAgentCommandDefaults = {
  "systemAgents.codexCommand": "codex",
  "systemAgents.opencodeCommand": "opencode",
  "systemAgents.dshCommand": "dsh",
} as const;

export function defaultExternalAgentCommand(path: string) {
  return Object.entries(externalAgentCommandDefaults).find(([key]) => key === path)?.[1];
}

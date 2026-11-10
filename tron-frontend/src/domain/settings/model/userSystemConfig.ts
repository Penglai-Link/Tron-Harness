// This is a product-facing allowlist, not a YAML editor. New backend parameters
// stay hidden until they have a user-facing purpose, label and help text.
export const userSystemConfigGroups = [
  { id: "systemAgents", paths: [
    "systemAgents.codexCommand", "systemAgents.opencodeCommand", "systemAgents.dshCommand",
  ] },
  { id: "mineru", paths: [
    "mineru.visualContextEnabled", "mineru.imageDescription.enabled", "mineru.imageDescription.modelCode",
  ] },
  { id: "shortTermMemory", paths: [
    "shortTermMemory.compaction.autoEnabled", "shortTermMemory.compaction.triggerRatio", "shortTermMemory.goalLoop.maxRuns",
  ] },
] as const;

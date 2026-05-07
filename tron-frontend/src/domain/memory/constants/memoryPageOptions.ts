export const defaultMemoryFilters = ["Namespace: UserPrefs", "Entity: usr_982x", "Confidence: > 0.85"] as const;

export const memoryStrategyTabs = [
  { label: "Hybrid", value: "hybrid" },
  { label: "Semantic Only", value: "semantic" },
  { label: "Keyword Only", value: "keyword" },
] as const;

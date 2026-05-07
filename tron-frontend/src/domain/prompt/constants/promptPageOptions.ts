export const promptWorkspaceTabs = [
  { label: "Dashboard", value: "dashboard" },
  { label: "Prompts", value: "prompts" },
  { label: "Models", value: "models" },
  { label: "Datasets", value: "datasets" },
  { label: "Logs", value: "logs" },
] as const;

export const promptVisibilityOptions = ["Workspace", "Private", "Public"] as const;

export const promptCategoryFilters = ["All", "System", "Persona", "Tools", "Templates"] as const;

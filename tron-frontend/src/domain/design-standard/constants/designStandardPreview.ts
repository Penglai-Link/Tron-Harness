export const designStandardSidebarItems = ["Dashboard", "Apps", "Tasks", "Agents", "Data", "Security"] as const;

export const designStandardTableRows = [
  { workspace: "Core Platform", source: "Local", model: "GPT-5.4", usage: "284k", status: "Healthy" },
  { workspace: "Agent Lab", source: "Inherited", model: "Claude Sonnet", usage: "119k", status: "Review" },
  { workspace: "Ops Console", source: "Override", model: "Gemini", usage: "76k", status: "Healthy" },
] as const;

export const designStandardSignalCards = ["Workspace isolation", "Inheritance aware", "Observable execution"] as const;

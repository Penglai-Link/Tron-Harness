export const tronColors = [
  { name: "Background", token: "background", value: "#fdf8f8", role: "Base canvas" },
  { name: "Surface", token: "surface", value: "#fdf8f8", role: "Workspace plane" },
  { name: "Container", token: "surface-container", value: "#f1edec", role: "Low emphasis fills" },
  { name: "Card Border", token: "card-border", value: "#e5e5e5", role: "Quiet separation" },
  { name: "Primary", token: "primary", value: "#000000", role: "Primary actions" },
  { name: "Accent", token: "accent", value: "#2f5f9f", role: "Focus and active state" },
  { name: "Success", token: "success", value: "#4f6f52", role: "Healthy state" },
  { name: "Warning", token: "warning", value: "#8a6a1f", role: "Risk state" },
  { name: "Error", token: "error", value: "#ba1a1a", role: "Failure state" },
] as const;

export const typeScale = [
  { name: "Display", className: "text-[40px] font-semibold leading-[1.1]", usage: "Major workspace title" },
  { name: "H1", className: "text-2xl font-semibold leading-[1.3]", usage: "Page header" },
  { name: "H2", className: "text-xl font-semibold leading-[1.3]", usage: "Section title" },
  { name: "Body LG", className: "text-base leading-6", usage: "Primary prose" },
  { name: "Body MD", className: "text-sm leading-[1.5]", usage: "Dense workspace text" },
  { name: "Body SM", className: "text-[13px] leading-[1.4]", usage: "Metadata and table cells" },
  { name: "Label Caps", className: "text-[11px] font-semibold uppercase leading-none tracking-[0.05em]", usage: "Table headers and labels" },
] as const;

export const componentPrinciples = [
  "Fixed 240px agent sidebar with fluid main canvas",
  "Top bar plus optional contextual right panel for management views",
  "High-density lists with 8px vertical rhythm and tabular numeric data",
  "Primary action in near-black; accent blue reserved for active and focus states",
  "Mock data behind hook/service boundaries until backend APIs are connected",
] as const;

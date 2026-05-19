export type StatusTone = "neutral" | "accent" | "success" | "warning" | "error";

export function getVariableSourceTone(status: string): StatusTone {
  const normalized = status.toLowerCase();
  if (normalized.includes("local")) {
    return "success";
  }
  if (normalized.includes("override")) {
    return "warning";
  }
  if (normalized.includes("inherited")) {
    return "accent";
  }
  return "neutral";
}

export function getStatusTone(status: string): StatusTone {
  return getVariableSourceTone(status);
}

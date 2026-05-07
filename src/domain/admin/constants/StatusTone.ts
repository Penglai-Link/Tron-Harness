export type StatusTone = "neutral" | "accent" | "success" | "warning" | "error";

export function getStatusTone(status: string): StatusTone {
  const normalized = status.toLowerCase();
  if (normalized.includes("healthy") || normalized.includes("active") || normalized.includes("connected") || normalized.includes("completed") || normalized.includes("success")) {
    return "success";
  }
  if (normalized.includes("review") || normalized.includes("sync") || normalized.includes("progress") || normalized.includes("override") || normalized.includes("invited") || normalized.includes("warning")) {
    return "warning";
  }
  if (normalized.includes("suspend") || normalized.includes("disabled") || normalized.includes("deprecated") || normalized.includes("failed") || normalized.includes("error")) {
    return "error";
  }
  return "neutral";
}

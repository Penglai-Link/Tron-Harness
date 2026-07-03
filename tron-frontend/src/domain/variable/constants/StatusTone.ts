import type { BadgeProps } from "@/components/ui/badge"
export function getVariableSourceTone(status: string): BadgeProps['variant'] {
  const normalized = status.toLowerCase();
  if (normalized.includes("local")) {
    return "success";
  }
  if (normalized.includes("override")) {
    return "warning";
  }
  if (normalized.includes("inherited")) {
    return "info";
  }
  return "outline";
}

export function getStatusTone(status: string): BadgeProps['variant'] {
  return getVariableSourceTone(status);
}

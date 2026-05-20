import type {
  AutomationExecutionStatus,
  AutomationOperationalStatus,
  AutomationStatus,
} from "../model";

export const editableAutomationStatuses: AutomationOperationalStatus[] = [
  "Active",
  "Paused",
];

export function getAutomationStatusTone(status: AutomationStatus) {
  if (status === "Active") return "success";
  if (status === "Paused") return "warning";
  return "neutral";
}

export function getAutomationExecutionStatusTone(
  status: AutomationExecutionStatus | null | undefined,
) {
  if (status === "Success") return "success";
  if (status === "Failed") return "error";
  if (status === "Running") return "accent";
  if (status === "Skipped" || status === "Canceled") return "warning";
  return "neutral";
}

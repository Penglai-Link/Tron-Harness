import type { AutomationRecord } from "../model";

export const automationInspectorTabs = [
  { label: "Details", value: "details" },
  { label: "Logs", value: "logs" },
  { label: "Traces", value: "traces" },
] as const;

export const defaultAutomationPrerequisites = ["Data Extract Job (ID: 8821)", "Validation Check (ID: 9012)"] as const;

export function getAutomationStatusTone(status: AutomationRecord["status"]) {
  if (status === "Success" || status === "Running") {
    return "success";
  }
  if (status === "Failed") {
    return "error";
  }
  return "neutral";
}

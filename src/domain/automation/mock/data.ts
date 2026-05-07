import type { AutomationRecord } from "@/domain/automation/model/AutomationModel";

const automations: AutomationRecord[] = [
  {
    id: "auto-daily-sync",
    name: "Daily Data Sync",
    cadence: "Every day at midnight",
    cron: "0 0 * * *",
    lastRun: "2h ago",
    status: "Success",
    owner: "Agent Alpha",
    dependencies: [
      { label: "Task B (Prereq)", status: "ok" },
      { label: "Daily Data Sync", status: "ok" },
      { label: "Model Eval Matrix", status: "failed" },
    ],
    logs: ["00:00 trigger accepted", "00:02 Salesforce extraction complete", "00:07 warehouse load complete"],
    traces: ["root.span sync.daily 432ms", "tool.salesforce.fetch 2.2s", "warehouse.merge 4.8s"],
  },
  {
    id: "auto-refresh",
    name: "Data Refresh Monitor",
    cadence: "Every 12 hours",
    cron: "0 */12 * * *",
    lastRun: "Running",
    status: "Running",
    owner: "Global Pool",
    dependencies: [
      { label: "Metric Snapshot", status: "ok" },
      { label: "Refresh Monitor", status: "ok" },
    ],
    logs: ["12:00 trigger accepted", "12:01 checking stale partitions"],
    traces: ["root.span refresh.monitor active", "tool.snowflake.inspect 1.1s"],
  },
  {
    id: "auto-hourly",
    name: "Hourly Guardrail Audit",
    cadence: "Every hour",
    cron: "0 * * * *",
    lastRun: "18m ago",
    status: "Failed",
    owner: "Agent Beta",
    dependencies: [
      { label: "Prompt Snapshot", status: "ok" },
      { label: "Guardrail Audit", status: "failed" },
    ],
    logs: ["14:00 trigger accepted", "14:02 prompt source timed out", "14:03 retry exhausted"],
    traces: ["root.span audit.hourly failed", "prompt.registry.fetch timeout"],
  },
];

export function getAutomations() {
  return automations;
}

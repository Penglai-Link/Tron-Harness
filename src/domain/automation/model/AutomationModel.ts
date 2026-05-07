export interface AutomationRecord {
  id: string;
  name: string;
  cadence: string;
  cron: string;
  lastRun: string;
  status: "Running" | "Success" | "Failed";
  owner: string;
  dependencies: { label: string; status: "ok" | "blocked" | "failed" }[];
  logs: string[];
  traces: string[];
}

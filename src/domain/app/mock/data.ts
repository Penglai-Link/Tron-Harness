import type {
  AppRecord,
  ImportApplicationDefaults,
} from "@/domain/app/model/AppModel";

const apps: AppRecord[] = [
  { id: "app-customer", name: "Customer Portal", source: "Frontend", path: "/apps/customer-portal", worktree: "main", tags: ["React", "Public"], updated: "4m ago" },
  { id: "app-auth", name: "Auth Service", source: "Backend", path: "/services/auth", worktree: "release/2.8", tags: ["Go", "Critical"], updated: "18m ago" },
  { id: "app-data", name: "Data Pipeline", source: "Worker", path: "/jobs/data-pipeline", worktree: "feature/sync-v3", tags: ["Python", "ETL"], updated: "42m ago" },
  { id: "app-billing", name: "Billing Console", source: "Frontend", path: "/apps/billing", worktree: "main", tags: ["React", "Finance"], updated: "1h ago" },
];

export function getApps() {
  return apps;
}

export function getImportApplicationDefaults(source: ImportApplicationDefaults["source"]): ImportApplicationDefaults {
  return {
    source,
    repositoryUrl: source === "github" ? "https://github.com/acme/refund-agent" : "",
    accessToken: "",
    localPath: source === "local" ? "/Users/username/projects/my-app" : "/Users/username/Projects",
    appName: source === "local" ? "local-agent-workbench" : "refund-agent",
    tags: ["Frontend", "React"],
    branch: "main",
  };
}

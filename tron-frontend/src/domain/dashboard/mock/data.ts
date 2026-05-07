import type {
  DashboardMetric,
  DashboardActivity,
} from "@/domain/dashboard/model/DashboardModel";

export function getDashboardData() {
  return {
    metrics: [
      { label: "System Health", value: "99.8%", trend: "+0.2% from last week" },
      { label: "Active Agents", value: "142", trend: "Across 12 clusters" },
      { label: "Running Tasks", value: "3,845", trend: "75% capacity, 2.1k queued" },
      { label: "Models", value: "28", trend: "18 inherited, 10 local" },
      { label: "MCP Servers", value: "16", trend: "12 healthy, 4 monitored" },
      { label: "Skills", value: "84", trend: "67 inherited, 17 custom" },
      { label: "Apps", value: "37", trend: "29 active, 8 in review" },
      { label: "Memory Items", value: "128K", trend: "Across graph and vectors" },
    ] satisfies DashboardMetric[],
    activities: [
      { title: "Skill Refund_Processing deployed", meta: "Just now - admin@system", tone: "success" },
      { title: "High latency detected in Model Router", meta: "2m ago - System Alert", tone: "warning" },
      { title: "Inheritance policy updated (Global)", meta: "15m ago - sec-ops@system", tone: "neutral" },
      { title: "New agent cluster provisioned", meta: "1h ago - auto-scaler", tone: "neutral" },
      { title: "Vector DB sync completed", meta: "3h ago - daily-job", tone: "success" },
    ] satisfies DashboardActivity[],
  };
}

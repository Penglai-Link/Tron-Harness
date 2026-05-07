export interface DashboardMetric {
  label: string;
  value: string;
  trend: string;
}

export interface DashboardActivity {
  title: string;
  meta: string;
  tone: "neutral" | "warning" | "success";
}

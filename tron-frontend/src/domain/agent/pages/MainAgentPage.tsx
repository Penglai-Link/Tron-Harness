import { Badge } from "@/components/ui/badge";
import {
  MetricCard,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";

import { useAuthSession } from "@/domain/auth";
import { MainAgentChatWindow } from "@/domain/agent/components/MainAgentChatWindow";

export function MainAgentPage() {
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string) => t(`mainAgent.${key}`, { defaultValue });
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const { session } = useAuthSession();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";

  return (
    <div className="relative min-h-[760px]">
        <div className="grid gap-4 opacity-55 lg:grid-cols-3">
          <MetricCard label={pageT("metrics.systemHealth", "System Health")} value="99.98%" trend={pageT("metrics.allServices", "All services operational")} />
          <MetricCard label={pageT("metrics.activeAgents", "Active Agents")} value="42" trend={pageT("metrics.agentsTrend", "+3 since last hour")} />
          <MetricCard label={pageT("metrics.pendingTasks", "Pending Tasks")} value="1,204" trend={pageT("metrics.tasksTrend", "Avg processing time: 1.2s")} />
          <SurfaceCard className="lg:col-span-2"><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{pageT("sections.throughput", "Throughput Analytics")}</h2><div className="h-48 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)]" /></SurfaceCard>
          <SurfaceCard><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{pageT("sections.activity", "Recent Activity")}</h2><p className="text-sm">{pageT("messages.deployment", "Deployment triggered in us-east-1")}</p></SurfaceCard>
        </div>
        <div className="absolute inset-0 bg-[var(--color-background)]/45" />
        <MainAgentChatWindow
          currentUserDisplayName={session?.user.displayName ?? ""}
          footerAddon={(
            <div className="flex items-center gap-[12px]">
              <Badge>{pageT("workspace", "Workspace")}</Badge>
              <span className="text-xs text-[var(--color-on-surface-variant)]">
                {activeWorkspaceCode || pageT("noWorkspace", "No workspace selected")}
              </span>
            </div>
          )}
          workspaceCode={activeWorkspaceCode}
        />
      </div>
  );
}
import { useTranslation } from "react-i18next";

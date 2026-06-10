import { Badge } from "@/shared/ui/Badge";
import {
  MetricCard,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";

import { useAuthSession } from "@/domain/auth";
import { MainAgentChatWindow } from "@/domain/agent/components/MainAgentChatWindow";

export function MainAgentPage() {
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const { session } = useAuthSession();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";

  return (
    <div className="relative min-h-[760px]">
        <div className="grid gap-4 opacity-55 lg:grid-cols-3">
          <MetricCard label="System Health" value="99.98%" trend="All services operational" />
          <MetricCard label="Active Agents" value="42" trend="+3 since last hour" />
          <MetricCard label="Pending Tasks" value="1,204" trend="Avg processing time: 1.2s" />
          <SurfaceCard className="lg:col-span-2"><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Throughput Analytics</h2><div className="h-48 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)]" /></SurfaceCard>
          <SurfaceCard><h2 className="mb-6 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Recent Activity</h2><p className="text-sm">Deployment triggered in us-east-1</p></SurfaceCard>
        </div>
        <div className="absolute inset-0 bg-[rgba(253,248,248,0.45)]" />
        <MainAgentChatWindow
          currentUserDisplayName={session?.user.displayName ?? ""}
          footerAddon={(
            <div className="flex items-center gap-2">
              <Badge>Workspace</Badge>
              <span className="text-xs text-[var(--color-on-surface-variant)]">
                {activeWorkspaceCode || "No workspace selected"}
              </span>
            </div>
          )}
          workspaceCode={activeWorkspaceCode}
        />
      </div>
  );
}

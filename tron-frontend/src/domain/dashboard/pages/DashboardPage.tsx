import {
  useMemo,
  useState,
} from "react";
import {
  Bot,
  CheckCircle2,
  CircleAlert,
  Database,
  FolderOpen,
  Maximize2,
  Network,
  RefreshCw,
  Settings2,
  Shield,
  Sparkles,
  Zap,
} from "lucide-react";

import {
  MetricCard,
  PageHeader,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";
import { useDashboardOverview } from "@/domain/dashboard/hooks";
import {
  buildWorkspaceTree,
  type WorkspaceTreeNode,
} from "@/shared/lib/workspaceTree";
import { cn } from "@/shared/lib/utils";
import type {
  PrimarySpace,
  SecondarySpace,
} from "@/shared/types/WorkspaceSpace";
import { Button } from "@/components/ui/button";

function DashboardContent() {
  const { activities, metrics } = useDashboardOverview();
  const { secondarySpaces, selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const [expanded, setExpanded] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const primaryName = selectedPrimarySpace?.name ?? "Enterprise";
  const primaryMeta = formatPrimarySpaceMeta(selectedPrimarySpace);
  const selectedWorkspaceCode = selectedSecondarySpace?.workspaceCode;
  const workspaceTree = useMemo(
    () => buildWorkspaceTree(secondarySpaces),
    [secondarySpaces],
  );

  return (
    <>
      <PageHeader
        title="Workspace Overview"
        actions={
          <Button size="lg" variant="outline" onClick={() => setShowAll((current) => !current)}>
            Export Report
          </Button>
        }
      />
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {metrics.map((metric) => (
          <MetricCard
            key={metric.label}
            label={metric.label}
            value={metric.value}
            trend={metric.trend}
            icon={getDashboardMetricIcon(metric.label)}
          />
        ))}
      </div>

      <div className="mt-5 grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(320px,1fr)]">
        <SurfaceCard>
          <div className="mb-4 flex items-center justify-between border-b border-[var(--color-outline-variant)] pb-3">
            <h2 className="text-xl font-semibold">Inheritance Map</h2>
            <Button size="icon" variant="ghost" aria-pressed={expanded} onClick={() => setExpanded((current) => !current)}>
              <Maximize2 size={17} aria-hidden="true" />
            </Button>
          </div>
          <div className={cn("text-sm", expanded ? "min-h-[360px]" : "min-h-[260px]")}>
            <TreeNode
              active={!selectedWorkspaceCode}
              title={`${primaryName}${!selectedWorkspaceCode ? " (Current)" : ""}`}
              meta={primaryMeta}
              root
            />
            {workspaceTree.length ? (
              <WorkspaceTreeBranch nodes={workspaceTree} selectedWorkspaceCode={selectedWorkspaceCode} />
            ) : (
              <div className="relative mt-4 pl-12">
                <p className="rounded-lg bg-[var(--color-surface-container-low)] p-3 text-[13px] text-[var(--color-on-surface-variant)]">
                  No secondary spaces configured for this primary space.
                </p>
              </div>
            )}
          </div>
          <p className="mt-3 text-[13px] text-[var(--color-on-surface-variant)]">{expanded ? "Expanded map controls are visible." : "Current navigation context."}</p>
        </SurfaceCard>

        <SurfaceCard className="p-0">
          <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-5 py-4">
            <h2 className="text-xl font-semibold">Recent Activity</h2>
            <Button size="lg" variant="outline" onClick={() => setShowAll((current) => !current)}>
              View All
            </Button>
          </div>
          <ul className="max-h-[360px] overflow-y-auto p-3">
            {activities.slice(0, showAll ? activities.length : 4).map((activity) => (
              <li className="flex gap-3 rounded-lg p-3 hover:bg-[var(--color-hover-surface)]" key={activity.title}>
                <span className="mt-0.5 flex size-7 items-center justify-center rounded-full bg-[var(--color-surface-container)]">
                  {activity.tone === "warning" ? <CircleAlert size={15} /> : <CheckCircle2 size={15} />}
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-sm">{activity.title}</span>
                  <span className="block text-[12px] text-[var(--color-on-surface-variant)]">{activity.meta}</span>
                </span>
              </li>
            ))}
          </ul>
        </SurfaceCard>
      </div>
    </>
  );
}

function formatPrimarySpaceMeta(primarySpace?: PrimarySpace) {
  if (!primarySpace) {
    return "Active · 0 child spaces · No description";
  }

  return [
    primarySpace.status,
    `${primarySpace.childCount} ${primarySpace.childCount === 1 ? "child space" : "child spaces"}`,
    primarySpace.description ?? "No description",
  ].join(" · ");
}

function formatSecondarySpaceMeta(space: SecondarySpace) {
  const parts = [
    `${space.effectiveUserCount} ${space.effectiveUserCount === 1 ? "effective user" : "effective users"}`,
    `${space.childCount} ${space.childCount === 1 ? "child space" : "child spaces"}`,
    space.status,
  ];

  if (space.description) {
    parts.push(space.description);
  }

  return parts.join(" · ");
}

function WorkspaceTreeBranch({
  nodes,
  selectedWorkspaceCode,
}: {
  nodes: WorkspaceTreeNode<SecondarySpace>[];
  selectedWorkspaceCode?: string;
}) {
  return (
    <div className="relative mt-4 pl-12">
      <span className="absolute bottom-7 left-[17px] top-0 w-px bg-[var(--color-outline-variant)]" aria-hidden="true" />
      {nodes.map((node) => {
        const active = node.space.workspaceCode === selectedWorkspaceCode;

        return (
          <div
            className="relative pb-3 last:pb-0"
            data-workspace-code={node.space.workspaceCode}
            data-workspace-depth={node.space.depth}
            key={node.space.workspaceCode}
          >
            <span className={cn("absolute left-[-30px] top-6 h-px w-8", active ? "bg-[var(--color-primary)]" : "bg-[var(--color-outline-variant)]")} aria-hidden="true" />
            <span
              className={cn(
                "absolute left-[-37px] top-[18px] size-4 rounded-full border-2 bg-white",
                active ? "border-[var(--color-primary)]" : "border-[var(--color-outline-variant)]",
              )}
              aria-hidden="true"
            />
            <TreeNode
              active={active}
              title={`${node.space.name}${active ? " (Current)" : ""}`}
              meta={formatSecondarySpaceMeta(node.space)}
            />
            {node.children.length ? (
              <WorkspaceTreeBranch nodes={node.children} selectedWorkspaceCode={selectedWorkspaceCode} />
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

function getDashboardMetricIcon(label: string) {
  switch (label) {
    case "System Health":
      return <Shield size={18} aria-hidden="true" />;
    case "Active Agents":
      return <Bot size={18} aria-hidden="true" />;
    case "Running Tasks":
      return <RefreshCw size={18} aria-hidden="true" />;
    case "Models":
      return <Sparkles size={18} aria-hidden="true" />;
    case "MCP Servers":
      return <Network size={18} aria-hidden="true" />;
    case "Skills":
      return <Zap size={18} aria-hidden="true" />;
    case "Apps":
      return <FolderOpen size={18} aria-hidden="true" />;
    case "Memory Items":
      return <Database size={18} aria-hidden="true" />;
    default:
      return <Settings2 size={18} aria-hidden="true" />;
  }
}

function TreeNode({ active, meta, root, title }: { active?: boolean; meta: string; root?: boolean; title: string }) {
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-xl border p-3",
        active
          ? "border-[var(--color-primary)] bg-white shadow-[0_10px_28px_rgba(0,0,0,0.06)]"
          : root
            ? "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)]"
            : "border-transparent bg-[var(--color-surface-container-low)]",
      )}
    >
      <span
        className={cn(
          "flex size-9 shrink-0 items-center justify-center rounded-full",
          active ? "bg-[var(--color-primary)] text-white" : root ? "bg-white text-[var(--color-on-surface-variant)]" : "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface-variant)]",
        )}
      >
        <Network size={16} aria-hidden="true" />
      </span>
      <span className="min-w-0">
        <span className="block font-semibold">{title}</span>
        <span className="mt-1 block text-[13px] text-[var(--color-on-surface-variant)]">{meta}</span>
      </span>
    </div>
  );
}

export function DashboardPage() {
  return <DashboardContent />;
}

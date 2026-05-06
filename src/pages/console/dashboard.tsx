import { useMemo, useState } from "react";
import {
  ArrowLeft,
  Bot,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  CircleAlert,
  Code2,
  Database,
  Download,
  Eye,
  FolderOpen,
  GitBranch,
  Maximize2,
  MessageSquare,
  Minus,
  Network,
  Plus,
  RefreshCw,
  Search,
  Send,
  Settings2,
  Shield,
  Sparkles,
  Terminal,
  Upload,
  X,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTable, Field, MetricCard, PageHeader, SegmentedTabs, SelectInput, SurfaceCard, TextArea, TextInput, TronShell, useWorkspaceSpaces } from "@/components/tron";
import { ClientLink, useNavigate } from "@/lib/client-router";
import { cn } from "@/lib/utils";
import {
  getDashboardData,
  getGraphData,
  getImportApplicationDefaults,
  getMainAgentMessages,
  getMemoryFragments,
  getModels,
  type GraphEntity,
  type ImportApplicationDefaults,
  type MemoryFragment,
  type ModelRegistryItem,
} from "@/services/console/workspace-data";

function ConsolePageShell({ activeItem, children }: { activeItem: string; children: React.ReactNode }) {
  return (
    <TronShell activeItem={activeItem} searchPlaceholder="Search Tron...">
      {children}
    </TronShell>
  );
}

function DashboardContent() {
  const { activities, metrics } = useMemo(getDashboardData, []);
  const { secondarySpaces, selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const [expanded, setExpanded] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const primaryName = selectedPrimarySpace?.name ?? "Enterprise Workspace";
  const primaryMeta = selectedPrimarySpace
    ? `${selectedPrimarySpace.region} · ${selectedPrimarySpace.status} · ${selectedPrimarySpace.description}`
    : "Global · Active · Global governance, security, and shared AI resource policy.";

  return (
    <>
      <PageHeader
        title="Workspace Overview"
        description="Monitor agent health, task execution, and system inheritance."
        actions={
          <Button variant="secondary" onClick={() => setShowAll((current) => !current)}>
            <Download size={16} aria-hidden="true" />
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
            <TreeNode title={primaryName} meta={primaryMeta} root />
            <div className="relative mt-4 pl-12">
              {secondarySpaces.length ? (
                <>
                  <span className="absolute bottom-7 left-[17px] top-0 w-px bg-[var(--color-outline-variant)]" aria-hidden="true" />
                  {secondarySpaces.map((space) => {
                  const active = space.id === selectedSecondarySpace?.id;
                  return (
                    <div className="relative pb-3 last:pb-0" key={space.id}>
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
                        title={`${space.name}${active ? " (Current)" : ""}`}
                        meta={`${space.description} · ${space.inheritedResources} inherited resources · ${space.localOverrides} local overrides · ${space.status}`}
                      />
                    </div>
                  );
                  })}
                </>
              ) : (
                <p className="rounded-lg bg-[var(--color-surface-container-low)] p-3 text-[13px] text-[var(--color-on-surface-variant)]">No secondary spaces configured for this primary space.</p>
              )}
            </div>
          </div>
          <p className="mt-3 text-[13px] text-[var(--color-on-surface-variant)]">{expanded ? "Expanded map controls are visible." : "Current navigation context."}</p>
        </SurfaceCard>

        <SurfaceCard className="p-0">
          <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-5 py-4">
            <h2 className="text-xl font-semibold">Recent Activity</h2>
            <button className="text-[11px] font-semibold uppercase text-[var(--color-accent)]" type="button" onClick={() => setShowAll((current) => !current)}>
              View All
            </button>
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
  return (
    <ConsolePageShell activeItem="Dashboard">
      <DashboardContent />
    </ConsolePageShell>
  );
}

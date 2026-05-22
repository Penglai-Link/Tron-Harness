import {
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  Search,
  Sparkles,
  Zap,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SurfaceCard,
  TronShell,
  useToast,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import {
  removeWorkspaceModel,
  restoreWorkspaceModel,
} from "@/domain/model/api";
import { useModelList } from "@/domain/model/hooks";
import {
  removeWorkspaceModelRecord,
  restoreWorkspaceModelRecord,
} from "@/domain/model/mock/data";
import type {
  WorkspaceModelRecord,
  WorkspaceModelStatus,
} from "@/domain/model/model";
import { modelVendorFilters } from "@/domain/model/constants";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";

const statusFilters = ["All", "Available", "Removed"] as const;

function RoutePageShell({ activeItem, children }: { activeItem: string; children: React.ReactNode }) {
  return (
    <TronShell activeItem={activeItem} searchPlaceholder="Search workspace models, vendors, and source policies...">
      {children}
    </TronShell>
  );
}

function SmallStatus({ children, tone = "neutral" }: { children: React.ReactNode; tone?: "neutral" | "accent" | "success" | "warning" | "error" }) {
  return <Badge tone={tone}>{children}</Badge>;
}

function Meta({ label, value }: { label: string; value: string }) {
  return <div><p className="text-[11px] text-[var(--color-on-surface-variant)]">{label}</p><p className="mt-1 font-medium">{value}</p></div>;
}

function statusTone(status: WorkspaceModelStatus) {
  return status === "Removed" ? "warning" : "success";
}

function ModelInspector({
  activeWorkspaceCode,
  activeWorkspaceLabel,
  model,
  mutating,
  onRemove,
  onRestore,
}: {
  activeWorkspaceCode: string;
  activeWorkspaceLabel: string;
  model: WorkspaceModelRecord;
  mutating: boolean;
  onRemove: (model: WorkspaceModelRecord) => void;
  onRestore: (model: WorkspaceModelRecord) => void;
}) {
  return (
    <SurfaceCard className="p-0">
      <div className="border-b border-[var(--color-outline-variant)] p-5">
        <div className="mb-4 flex items-start justify-between">
          <span className="flex size-12 items-center justify-center rounded-lg border border-[var(--color-outline-variant)]">
            <Sparkles size={24} />
          </span>
          <SmallStatus tone={statusTone(model.status)}>{model.status}</SmallStatus>
        </div>
        <h2 className="text-xl font-semibold">{model.name}</h2>
        <p className="mb-4 mt-1 text-sm text-[var(--color-on-surface-variant)]">{model.vendor} - {model.price}</p>
        {activeWorkspaceCode === "PLATFORM" ? (
          <p className="rounded-lg bg-[var(--color-surface-container-low)] p-3 text-sm text-[var(--color-on-surface-variant)]">
            Platform is the source of truth. Child workspaces inherit every enabled platform model by default.
          </p>
        ) : model.status === "Removed" ? (
          <Button className="w-full" disabled={mutating} variant="secondary" onClick={() => onRestore(model)}>
            Restore In {activeWorkspaceLabel}
          </Button>
        ) : (
          <Button className="w-full" disabled={mutating} variant="secondary" onClick={() => onRemove(model)}>
            Remove From {activeWorkspaceLabel}
          </Button>
        )}
      </div>
      <div className="space-y-6 p-5">
        <section>
          <h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Workspace Effective State</h3>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <Meta label="Current Workspace" value={activeWorkspaceLabel} />
            <Meta label="Source" value={model.sourceWorkspaceName} />
            <Meta label="Inheritance" value={model.status === "Removed" ? `Removed in ${activeWorkspaceCode}` : `Inherited from ${model.sourceWorkspaceName}`} />
            <Meta label="Removed At" value={model.removedAt ?? "Not removed"} />
          </div>
        </section>
        <section>
          <h3 className="mb-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">Configuration Details</h3>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <Meta label="Max Context" value={model.context} />
            <Meta label="Max Output" value={model.maxOutput} />
            <Meta label="Training Data" value={model.trainingData} />
            <Meta label="Source Workspace" value={model.sourceWorkspaceCode} />
          </div>
        </section>
        <div className="flex items-center justify-between rounded-lg bg-[var(--color-surface-container-low)] p-3 text-sm">
          <span className="flex items-center gap-2"><Zap size={16} />Avg. Generation Speed</span>
          <span>{model.speed}</span>
        </div>
        <p className="text-[13px] text-[var(--color-on-surface-variant)]">
          Removing a model only hides it in {activeWorkspaceLabel}. The platform configuration stays unchanged.
        </p>
      </div>
    </SurfaceCard>
  );
}

export function ModelRegistryPage() {
  const toast = useToast();
  const dataSource = getServiceDataSource("model");
  const {
    selectedPrimarySpace,
    selectedSecondarySpace,
  } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceLabel = selectedSecondarySpace?.label ?? selectedPrimarySpace?.name ?? "No workspace selected";
  const { loading, models, refresh } = useModelList(activeWorkspaceCode);
  const [vendor, setVendor] = useState("All Vendors");
  const [status, setStatus] = useState<WorkspaceModelStatus | "All">("All");
  const [query, setQuery] = useState("");
  const [selectedModelCode, setSelectedModelCode] = useState("");
  const [mutatingModelCode, setMutatingModelCode] = useState<string | null>(null);

  const visible = useMemo(() => models.filter((model) => (
    (vendor === "All Vendors" || model.vendor === vendor)
    && (status === "All" || model.status === status)
    && [model.modelCode, model.name, model.purpose].join(" ").toLowerCase().includes(query.toLowerCase())
  )), [models, query, status, vendor]);
  const availableCount = models.filter((model) => model.status === "Available").length;
  const removedCount = models.filter((model) => model.status === "Removed").length;
  const selected = visible.find((model) => model.modelCode === selectedModelCode) ?? visible[0] ?? models[0];

  useEffect(() => {
    if (!selected) {
      setSelectedModelCode("");
      return;
    }
    if (selected.modelCode !== selectedModelCode) {
      setSelectedModelCode(selected.modelCode);
    }
  }, [selected, selectedModelCode]);

  async function mutateModel(action: "remove" | "restore", model: WorkspaceModelRecord) {
    if (!activeWorkspaceCode) return;

    setMutatingModelCode(model.modelCode);
    try {
      if (action === "remove") {
        if (dataSource === "api") {
          await removeWorkspaceModel(activeWorkspaceCode, model.modelCode);
        } else {
          removeWorkspaceModelRecord(activeWorkspaceCode, model.modelCode);
        }
        toast.success(`${model.name} removed from ${activeWorkspaceCode}`);
      } else {
        if (dataSource === "api") {
          await restoreWorkspaceModel(activeWorkspaceCode, model.modelCode);
        } else {
          restoreWorkspaceModelRecord(activeWorkspaceCode, model.modelCode);
        }
        toast.success(`${model.name} restored in ${activeWorkspaceCode}`);
      }
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : `Model ${action} failed`);
    } finally {
      setMutatingModelCode(null);
    }
  }

  return (
    <RoutePageShell activeItem="Models">
      <div className="grid min-h-[740px] gap-4 xl:grid-cols-[minmax(0,1fr)_380px]">
        <section>
          <PageHeader
            title="Workspace Models"
            description={`Manage the effective model catalog for ${activeWorkspaceLabel}. Every workspace inherits all enabled platform models by default and can remove them locally without mutating the platform source.`}
          />
          <div className="mb-4 grid gap-3 md:grid-cols-3">
            <SurfaceCard className="p-4">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Workspace</p>
              <p className="mt-2 text-base font-semibold text-[var(--color-on-surface)]">{activeWorkspaceLabel}</p>
            </SurfaceCard>
            <SurfaceCard className="p-4">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Available</p>
              <p className="mt-2 text-base font-semibold text-[var(--color-on-surface)]">{availableCount}</p>
            </SurfaceCard>
            <SurfaceCard className="p-4">
              <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Removed Locally</p>
              <p className="mt-2 text-base font-semibold text-[var(--color-on-surface)]">{removedCount}</p>
            </SurfaceCard>
          </div>
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] pb-4">
            <div className="flex flex-wrap gap-2">
              {modelVendorFilters.map((item) => (
                <button className={cn("rounded-full border px-3 py-1.5 text-sm", vendor === item ? "bg-[var(--color-accent-container)] text-[var(--color-on-accent-container)]" : "border-[var(--color-outline-variant)] bg-white")} type="button" key={item} onClick={() => setVendor(item)}>
                  {item}
                </button>
              ))}
              {statusFilters.map((item) => (
                <button className={cn("rounded-full border px-3 py-1.5 text-sm", status === item ? "bg-[var(--color-surface-container-high)] text-[var(--color-on-surface)]" : "border-[var(--color-outline-variant)] bg-white")} type="button" key={item} onClick={() => setStatus(item)}>
                  {item}
                </button>
              ))}
            </div>
            <div className="flex h-9 w-64 items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-white px-3">
              <Search size={15} />
              <input className="min-w-0 flex-1 bg-transparent text-sm outline-none" placeholder="Search models..." value={query} onChange={(event) => setQuery(event.target.value)} />
            </div>
          </div>
          <div className="overflow-hidden rounded-xl border border-[var(--color-outline-variant)] bg-white">
            <div className="grid grid-cols-12 gap-4 border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">
              <div className="col-span-4">Model Name</div><div className="col-span-2">Vendor</div><div className="col-span-2">Context</div><div className="col-span-2">Source</div><div className="col-span-2 text-right">Status</div>
            </div>
            {loading && !models.length ? (
              <div className="px-4 py-6 text-sm text-[var(--color-on-surface-variant)]">Loading workspace models...</div>
            ) : visible.length ? visible.map((model) => (
              <button className={cn("grid w-full grid-cols-12 items-center gap-4 border-b border-[var(--color-outline-variant)] px-4 py-3 text-left last:border-0 hover:bg-[var(--color-hover-surface)]", selected?.modelCode === model.modelCode ? "bg-[var(--color-surface-container)]" : "", model.status === "Removed" ? "opacity-75" : "")} key={model.modelCode} type="button" onClick={() => setSelectedModelCode(model.modelCode)}>
                <div className="col-span-4 flex items-center gap-3"><span className="flex size-8 items-center justify-center rounded border border-[var(--color-outline-variant)]"><Sparkles size={16} /></span><span><span className="block text-sm font-medium">{model.name}</span><span className="block text-[13px] text-[var(--color-on-surface-variant)]">{model.purpose}</span></span></div>
                <div className="col-span-2 text-sm">{model.vendor}</div><div className="col-span-2 font-mono text-[13px]">{model.context}</div><div className="col-span-2 text-sm">{model.sourceWorkspaceName}</div><div className="col-span-2 text-right"><SmallStatus tone={statusTone(model.status)}>{model.status}</SmallStatus></div>
              </button>
            )) : (
              <div className="px-4 py-6 text-sm text-[var(--color-on-surface-variant)]">
                {activeWorkspaceCode ? "No models match the current filters." : "Select a workspace in the shell header to load models."}
              </div>
            )}
          </div>
        </section>
        {selected ? (
          <ModelInspector
            activeWorkspaceCode={activeWorkspaceCode}
            activeWorkspaceLabel={activeWorkspaceLabel}
            model={selected}
            mutating={mutatingModelCode === selected.modelCode}
            onRemove={(model) => void mutateModel("remove", model)}
            onRestore={(model) => void mutateModel("restore", model)}
          />
        ) : (
          <SurfaceCard>
            <h2 className="text-base font-semibold text-[var(--color-on-surface)]">No model selected</h2>
            <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
              {activeWorkspaceCode ? "Choose a model from the table to inspect its effective workspace state." : "Select a workspace from the shell switcher to load models."}
            </p>
          </SurfaceCard>
        )}
      </div>
    </RoutePageShell>
  );
}

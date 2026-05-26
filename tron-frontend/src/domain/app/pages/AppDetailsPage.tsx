import { useState } from "react";
import {
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  Archive,
  Pencil,
  RotateCcw,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  InspectorPanel,
  PageHeader,
  SurfaceCard,
  useConsoleShellConfig,
  useToast,
} from "@/shared";

import {
  archiveApp,
  restoreApp,
} from "@/domain/app/api";
import {
  AppDetailField,
  AppRuntimeRoleBadge,
  AppStatusBadge,
  AppTagList,
  appArchivedLabel,
} from "@/domain/app/components";
import { useAppDetail } from "@/domain/app/hooks";
import {
  archiveMockApp,
  restoreMockApp,
} from "@/domain/app/mock";
import { formatAppTimestamp } from "@/domain/app/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function AppDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("app");
  const appCode = searchParams.get("appCode") ?? "";
  const { app, loading, refresh } = useAppDetail(appCode);
  const [updatingLifecycle, setUpdatingLifecycle] = useState(false);
  const rightPanel = app ? (
    <InspectorPanel title="App Metadata">
      <div className="space-y-5">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Metadata</p>
          <div className="mt-2 space-y-3 text-sm">
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Created</span><span className="font-medium">{formatAppTimestamp(app.createdAt)}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Updated</span><span className="font-medium">{formatAppTimestamp(app.updatedAt)}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Archived At</span><span className="font-medium">{appArchivedLabel(app.archivedAt)}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">Identifier</span><span className="font-medium">{app.id}</span></div>
          </div>
        </div>
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Local Repository</p>
          <div className="mt-2 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-sm">
            <p>{app.localDirectory}</p>
            <p className="mt-2 text-[var(--color-on-surface-variant)]">{app.repositoryUrl ?? "Not set"}</p>
            <p className="mt-2 text-[var(--color-on-surface-variant)]">{app.defaultBranch ?? "Not set"}</p>
          </div>
        </div>
      </div>
    </InspectorPanel>
  ) : undefined;

  useConsoleShellConfig({ rightPanel });

  async function handleLifecycle(action: "archive" | "restore") {
    if (!app) return;
    setUpdatingLifecycle(true);
    try {
      if (action === "archive") {
        if (dataSource === "api") {
          await archiveApp(app.appCode);
        } else {
          archiveMockApp(app.appCode);
        }
        toast.success(`${app.name} archived`);
      } else {
        if (dataSource === "api") {
          await restoreApp(app.appCode);
        } else {
          restoreMockApp(app.appCode);
        }
        toast.success(`${app.name} restored`);
      }
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "App lifecycle update failed");
    } finally {
      setUpdatingLifecycle(false);
    }
  }

  if (loading && !app) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: appCode || "Loading" }]}
          title="App Detail"
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Loading app detail</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">Retrieving {appCode} from the app detail endpoint.</p>
        </SurfaceCard>
      </>
    );
  }

  if (!app) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: appCode || "Not Found" }]}
          title="App Detail"
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">App not found</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{appCode ? `No app matches ${appCode}.` : "No app was requested."}</p>
        </SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: app.appCode }]}
        title={app.name}
        actions={(
          <>
            <Button type="button" variant="secondary" onClick={() => navigate(`/apps/edit?appCode=${encodeURIComponent(app.appCode)}`)}><Pencil size={15} aria-hidden="true" />Edit</Button>
            {app.status === "Archived" ? (
              <Button type="button" disabled={updatingLifecycle} variant="secondary" onClick={() => void handleLifecycle("restore")}>
                <RotateCcw size={15} aria-hidden="true" />
                Restore
              </Button>
            ) : (
              <Button type="button" disabled={updatingLifecycle} variant="secondary" onClick={() => void handleLifecycle("archive")}>
                <Archive size={15} aria-hidden="true" />
                Archive
              </Button>
            )}
          </>
        )}
      />
      <SurfaceCard>
        <div className="mb-5 flex flex-wrap items-center gap-2">
          <AppRuntimeRoleBadge runtimeRole={app.runtimeRole} />
          <AppStatusBadge status={app.status} />
        </div>
        <div className="grid gap-4 md:grid-cols-2">
          <AppDetailField label="App Code" value={app.appCode} />
          <AppDetailField label="Local Directory" value={app.localDirectory} />
          <AppDetailField label="Repository URL" value={app.repositoryUrl ?? "Not set"} />
          <AppDetailField label="Default Branch" value={app.defaultBranch ?? "Not set"} />
        </div>
        <div className="mt-5">
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Description</p>
          <p className="mt-2 whitespace-pre-wrap text-sm text-[var(--color-on-surface)]">{app.description ?? "No description provided."}</p>
        </div>
        <div className="mt-5">
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">Tags</p>
          <div className="mt-2">
            <AppTagList tags={app.tags} />
          </div>
        </div>
      </SurfaceCard>
    </>
  );
}

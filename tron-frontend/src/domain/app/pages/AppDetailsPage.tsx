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
  PageHeader,
  SurfaceCard,
  TronShell,
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
      <TronShell activeItem="Apps" searchPlaceholder="Search apps, repositories, and tags...">
        <PageHeader
          breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: appCode || "Loading" }]}
          title="App Detail"
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Loading app detail</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">Retrieving {appCode} from the app detail endpoint.</p>
        </SurfaceCard>
      </TronShell>
    );
  }

  if (!app) {
    return (
      <TronShell activeItem="Apps" searchPlaceholder="Search apps, repositories, and tags...">
        <PageHeader
          breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: appCode || "Not Found" }]}
          title="App Detail"
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">App not found</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{appCode ? `No app matches ${appCode}.` : "No app was requested."}</p>
        </SurfaceCard>
      </TronShell>
    );
  }

  return (
    <TronShell activeItem="Apps" searchPlaceholder="Search apps, repositories, and tags...">
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
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_320px]">
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
        <SurfaceCard className="self-start">
          <div className="grid gap-4">
            <AppDetailField label="Created" value={formatAppTimestamp(app.createdAt)} />
            <AppDetailField label="Updated" value={formatAppTimestamp(app.updatedAt)} />
            <AppDetailField label="Archived At" value={appArchivedLabel(app.archivedAt)} />
            <AppDetailField label="Identifier" value={app.id} />
          </div>
        </SurfaceCard>
      </div>
    </TronShell>
  );
}

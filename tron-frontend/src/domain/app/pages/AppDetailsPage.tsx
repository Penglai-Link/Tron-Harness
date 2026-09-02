import {
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  useNavigate,
  useSearchParams,
} from "react-router";

import { Button } from "@/components/ui/button";
import {
  InspectorPanel,
  PageHeader,
  SurfaceCard,
  useLocaleFormatters,
  useTabPanelConfig,
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
  const { t } = useTranslation("pages");
  const pageT = (key: string, options?: Record<string, unknown>) => t(`apps.${key}`, options);
  const toast = useToast();
  const { currentLocale } = useLocaleFormatters();
  const dataSource = getServiceDataSource("app");
  const appCode = searchParams.get("appCode") ?? "";
  const { app, loading, refresh } = useAppDetail(appCode);
  const [updatingLifecycle, setUpdatingLifecycle] = useState(false);
  const rightPanel = useMemo(() => (app ? (
    <InspectorPanel title={pageT("detail.metadata")}>
      <div className="space-y-5">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("detail.metadata")}</p>
          <div className="mt-2 space-y-3 text-sm">
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.created")}</span><span className="font-medium">{formatAppTimestamp(app.createdAt, currentLocale)}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.updated")}</span><span className="font-medium">{formatAppTimestamp(app.updatedAt, currentLocale)}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.archivedAt")}</span><span className="font-medium">{appArchivedLabel(app.archivedAt, currentLocale, pageT("detail.notArchived"))}</span></div>
            <div className="flex justify-between gap-3"><span className="text-[var(--color-on-surface-variant)]">{pageT("detail.identifier")}</span><span className="font-medium">{app.id}</span></div>
          </div>
        </div>
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("detail.localRepository")}</p>
          <div className="mt-2 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-sm">
            <p>{app.localDirectory}</p>
            <p className="mt-2 text-[var(--color-on-surface-variant)]">{app.repositoryUrl ?? pageT("detail.notSet")}</p>
            <p className="mt-2 text-[var(--color-on-surface-variant)]">{app.defaultBranch ?? pageT("detail.notSet")}</p>
          </div>
        </div>
      </div>
    </InspectorPanel>
  ) : undefined), [app, currentLocale]);

  useTabPanelConfig({ rightPanel });

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
        toast.success(pageT("messages.archived", { name: app.name }));
      } else {
        if (dataSource === "api") {
          await restoreApp(app.appCode);
        } else {
          restoreMockApp(app.appCode);
        }
        toast.success(pageT("messages.restored", { name: app.name }));
      }
      refresh();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("messages.lifecycleUpdateFailed"));
    } finally {
      setUpdatingLifecycle(false);
    }
  }

  if (loading && !app) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: pageT("breadcrumbs.apps"), href: "/apps" }, { label: appCode || pageT("breadcrumbs.loading") }]}
          title={pageT("detail.title")}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("detail.loadingTitle")}</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{pageT("detail.loadingDescription", { code: appCode })}</p>
        </SurfaceCard>
      </>
    );
  }

  if (!app) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: pageT("breadcrumbs.apps"), href: "/apps" }, { label: appCode || pageT("breadcrumbs.notFound") }]}
          title={pageT("detail.title")}
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pageT("detail.notFoundTitle")}</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{appCode ? pageT("detail.noAppMatches", { code: appCode }) : pageT("detail.noAppRequested")}</p>
        </SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: pageT("breadcrumbs.apps"), href: "/apps" }, { label: app.appCode }]}
        title={app.name}
        actions={(
          <>
            <Button type="button" size="lg" variant="outline" onClick={() => navigate(`/apps/edit?appCode=${encodeURIComponent(app.appCode)}`)}>{pageT("actions.edit")}</Button>
            {app.status === "Archived" ? (
              <Button size="lg" type="button" disabled={updatingLifecycle} variant="destructive" onClick={() => void handleLifecycle("restore")}>
                {pageT("actions.restore")}
              </Button>
            ) : (
              <Button size="lg" type="button" disabled={updatingLifecycle} onClick={() => void handleLifecycle("archive")}>
                {pageT("actions.archive")}
              </Button>
            )}
          </>
        )}
      />
      <SurfaceCard>
        <div className="mb-5 flex flex-wrap items-center gap-[12px]">
          <AppRuntimeRoleBadge runtimeRole={app.runtimeRole} />
          <AppStatusBadge status={app.status} />
        </div>
        <div className="grid gap-4 md:grid-cols-2">
          <AppDetailField label={pageT("detail.appCode")} value={app.appCode} />
          <AppDetailField label={pageT("detail.localDirectory")} value={app.localDirectory} />
          <AppDetailField label={pageT("detail.repositoryUrl")} value={app.repositoryUrl ?? pageT("detail.notSet")} />
          <AppDetailField label={pageT("detail.defaultBranch")} value={app.defaultBranch ?? pageT("detail.notSet")} />
        </div>
        <div className="mt-5">
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("detail.description")}</p>
          <p className="mt-2 whitespace-pre-wrap text-sm text-[var(--color-on-surface)]">{app.description ?? pageT("detail.noDescription")}</p>
        </div>
        <div className="mt-5">
          <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{pageT("detail.tags")}</p>
          <div className="mt-2">
            <AppTagList tags={app.tags} />
          </div>
        </div>
      </SurfaceCard>
    </>
  );
}

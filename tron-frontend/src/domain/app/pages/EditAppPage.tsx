import {
  type FormEvent,
  useEffect,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  useNavigate,
  useSearchParams,
} from "react-router";

import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

import { updateApp } from "@/domain/app/api";
import {
  AppFormFields,
  AppStatusBadge,
  AppRuntimeRoleBadge,
  buildAppPayload,
  canSubmitAppForm,
  emptyAppForm,
  formFromApp,
  type AppFormState,
} from "@/domain/app/components";
import { useAppDetail } from "@/domain/app/hooks";
import { updateMockApp } from "@/domain/app/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function EditAppPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { t } = useTranslation("pages");
  const pageT = (key: string, options?: Record<string, unknown>) => t(`apps.${key}`, options);
  const toast = useToast();
  const dataSource = getServiceDataSource("app");
  const appCode = searchParams.get("appCode") ?? "";
  const { app, loading, setApp } = useAppDetail(appCode);
  const [form, setForm] = useState<AppFormState>(emptyAppForm);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!app) return;
    setForm(formFromApp(app));
  }, [app]);

  function updateForm<K extends keyof AppFormState>(key: K, value: AppFormState[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!app) return;
    if (app.status === "Archived") {
      toast.error(pageT("messages.restoreBeforeEdit"));
      return;
    }

    setSubmitting(true);
    try {
      const updated = dataSource === "api"
        ? await updateApp(app.appCode, buildAppPayload(form))
        : updateMockApp(app.appCode, buildAppPayload(form));
      setApp(updated);
      toast.success(pageT("messages.updated", { name: updated.name }));
      navigate(`/apps/detail?appCode=${encodeURIComponent(updated.appCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("messages.updateFailed"));
    } finally {
      setSubmitting(false);
    }
  }

  if (loading && !app) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: pageT("breadcrumbs.apps"), href: "/apps" }, { label: appCode || pageT("breadcrumbs.edit") }]}
          title={pageT("editTitle")}
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
          title={pageT("editTitle")}
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
        breadcrumbs={[{ label: pageT("breadcrumbs.apps"), href: "/apps" }, { label: app.appCode, href: `/apps/detail?appCode=${encodeURIComponent(app.appCode)}` }, { label: pageT("breadcrumbs.edit") }]}
        title={`${pageT("breadcrumbs.edit")} ${app.name}`}
        actions={(
          <>
            <Button type="button" variant="outline" size="lg" onClick={() => navigate(`/apps/detail?appCode=${encodeURIComponent(app.appCode)}`)}>{pageT("actions.cancel")}</Button>
            <Button type="submit" form="edit-app-form" size="lg" disabled={submitting || !canSubmitAppForm(form) || app.status === "Archived"}>
              {submitting ? pageT("actions.saving") : pageT("actions.save")}
            </Button>
          </>
        )}
      />
      <SurfaceCard>
        <div className="mb-5 flex flex-wrap gap-[12px]">
          <AppRuntimeRoleBadge runtimeRole={form.runtimeRole} />
          <AppStatusBadge status={app.status} />
        </div>
        <form className="space-y-6" id="edit-app-form" onSubmit={handleSubmit}>
          <AppFormFields disabled={submitting || app.status === "Archived"} form={form} onChange={updateForm} />
        </form>
      </SurfaceCard>
    </>
  );
}

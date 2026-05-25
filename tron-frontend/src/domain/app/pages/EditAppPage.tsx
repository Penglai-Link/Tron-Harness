import {
  type FormEvent,
  useEffect,
  useState,
} from "react";
import {
  useNavigate,
  useSearchParams,
} from "react-router";
import { Save } from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SurfaceCard,
  TronShell,
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
      toast.error("Restore this app before editing");
      return;
    }

    setSubmitting(true);
    try {
      const updated = dataSource === "api"
        ? await updateApp(app.appCode, buildAppPayload(form))
        : updateMockApp(app.appCode, buildAppPayload(form));
      setApp(updated);
      toast.success(`${updated.name} updated`);
      navigate(`/apps/detail?appCode=${encodeURIComponent(updated.appCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "App could not be updated");
    } finally {
      setSubmitting(false);
    }
  }

  if (loading && !app) {
    return (
      <TronShell activeItem="Apps" searchPlaceholder="Search apps, repositories, and tags...">
        <PageHeader
          breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: appCode || "Edit" }]}
          title="Edit App"
        />
        <SurfaceCard>
          <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Loading app detail</h2>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">Preparing the app editor for {appCode}.</p>
        </SurfaceCard>
      </TronShell>
    );
  }

  if (!app) {
    return (
      <TronShell activeItem="Apps" searchPlaceholder="Search apps, repositories, and tags...">
        <PageHeader
          breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: appCode || "Not Found" }]}
          title="Edit App"
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
        breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: app.appCode, href: `/apps/detail?appCode=${encodeURIComponent(app.appCode)}` }, { label: "Edit" }]}
        title={`Edit ${app.name}`}
        actions={(
          <>
            <Button type="button" variant="secondary" onClick={() => navigate(`/apps/detail?appCode=${encodeURIComponent(app.appCode)}`)}>Cancel</Button>
            <Button type="submit" form="edit-app-form" disabled={submitting || !canSubmitAppForm(form) || app.status === "Archived"}>
              <Save size={15} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        )}
      />
      <SurfaceCard>
        <div className="mb-5 flex flex-wrap gap-2">
          <AppRuntimeRoleBadge runtimeRole={form.runtimeRole} />
          <AppStatusBadge status={app.status} />
        </div>
        <form className="space-y-6" id="edit-app-form" onSubmit={handleSubmit}>
          <AppFormFields disabled={submitting || app.status === "Archived"} form={form} onChange={updateForm} />
        </form>
      </SurfaceCard>
    </TronShell>
  );
}

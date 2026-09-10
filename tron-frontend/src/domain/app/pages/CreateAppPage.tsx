import {
  type FormEvent,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
  useWorkspaceSpaces,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import { createApp } from "@/domain/app/api";
import {
  AppFormFields,
  buildAppPayload,
  canSubmitAppForm,
  emptyAppForm,
  type AppFormState,
} from "@/domain/app/components";
import { createMockApp } from "@/domain/app/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

export function CreateAppPage() {
  const navigate = useNavigate();
  const { t } = useTranslation("pages");
  const pageT = (key: string, options?: Record<string, unknown>) => t(`apps.${key}`, options);
  const toast = useToast();
  const dataSource = getServiceDataSource("app");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const [form, setForm] = useState<AppFormState>(emptyAppForm);
  const [submitting, setSubmitting] = useState(false);

  function updateForm<K extends keyof AppFormState>(key: K, value: AppFormState[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!activeWorkspaceCode) {
      toast.error(pageT("messages.workspaceRequired"));
      return;
    }

    setSubmitting(true);
    try {
      const created = dataSource === "api"
        ? await createApp(activeWorkspaceCode, buildAppPayload(form))
        : createMockApp(activeWorkspaceCode, buildAppPayload(form));
      toast.success(pageT("messages.created", { name: created.name }));
      navigate(`/apps/detail?appCode=${encodeURIComponent(created.appCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("messages.createFailed"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: pageT("breadcrumbs.apps"), href: "/apps" }, { label: pageT("breadcrumbs.new") }]}
        title={pageT("createTitle")}
      />
      <SurfaceCard>
        <form className="space-y-6" onSubmit={handleSubmit}>
          <AppFormFields disabled={submitting} form={form} onChange={updateForm} />
          <div className="flex flex-wrap justify-end gap-[12px]">
            <Button size="lg" type="button" variant="outline" onClick={() => navigate("/apps")}>{pageT("actions.cancel")}</Button>
            <Button size="lg" type="submit" disabled={submitting || !activeWorkspaceCode || !canSubmitAppForm(form)}>
              {submitting ? pageT("actions.saving") : pageT("actions.save")}
            </Button>
          </div>
        </form>
      </SurfaceCard>
    </>
  );
}

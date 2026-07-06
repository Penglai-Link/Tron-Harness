import {
  type FormEvent,
  useState,
} from "react";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

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
      toast.error("Select a workspace in the shell header before creating an app");
      return;
    }

    setSubmitting(true);
    try {
      const created = dataSource === "api"
        ? await createApp(activeWorkspaceCode, buildAppPayload(form))
        : createMockApp(activeWorkspaceCode, buildAppPayload(form));
      toast.success(`${created.name} created`);
      navigate(`/apps/detail?appCode=${encodeURIComponent(created.appCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "App could not be created");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Apps", href: "/apps" }, { label: "New" }]}
        title="Create App"
      />
      <SurfaceCard>
        <form className="space-y-6" onSubmit={handleSubmit}>
          <AppFormFields disabled={submitting} form={form} onChange={updateForm} />
          <div className="flex flex-wrap justify-end gap-2">
            <Button size="lg" type="button" variant="outline" onClick={() => navigate("/apps")}>Cancel</Button>
            <Button size="lg" type="submit" disabled={submitting || !activeWorkspaceCode || !canSubmitAppForm(form)}>
              {submitting ? "Saving..." : "Save"}
            </Button>
          </div>
        </form>
      </SurfaceCard>
    </>
  );
}

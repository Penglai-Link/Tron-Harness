import { useState } from "react";
import { useNavigate } from "react-router";
import { Save } from "lucide-react";

import {
  createPlatformOpenApp,
  type CreateOpenAppPayload,
} from "@/domain/open-app/api";
import {
  emptyMetadataDraft,
  OpenAppMetadataFormFields,
  useOpenAppPageT,
} from "@/domain/open-app/components";
import { createPlatformOpenAppMock } from "@/domain/open-app/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Button,
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

export function CreatePlatformOpenAppPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const pageT = useOpenAppPageT();
  const dataSource = getServiceDataSource("admin");
  const [metadataDraft, setMetadataDraft] = useState(emptyMetadataDraft);
  const [submitting, setSubmitting] = useState(false);

  async function handleCreate() {
    const payload: CreateOpenAppPayload = {
      description: metadataDraft.description,
      name: metadataDraft.name,
    };
    setSubmitting(true);
    try {
      const result = dataSource === "api"
        ? await createPlatformOpenApp(payload)
        : createPlatformOpenAppMock(payload);
      toast.success(pageT("messages.created", "{{name}} created", { name: result.openApp.name }));
      navigate(`/settings/openapi/detail?openAppCode=${encodeURIComponent(result.openApp.openAppCode)}`, {
        replace: true,
        state: { secretReveal: result.secretReveal },
      });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.create", "Open-app could not be created"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[
          { label: pageT("breadcrumbs.list", "OpenAPI"), href: "/settings/openapi" },
          { label: pageT("breadcrumbs.new", "New") },
        ]}
        title={pageT("title.new", "New Open App")}
        actions={(
          <>
            <Button
              type="button"
              variant="secondary"
              disabled={submitting}
              onClick={() => navigate("/settings/openapi")}
            >
              {pageT("actions.cancel", "Cancel")}
            </Button>
            <Button
              disabled={submitting || !metadataDraft.name.trim()}
              aria-busy={submitting}
              onClick={() => void handleCreate()}
            >
              <Save size={16} aria-hidden="true" />
              {submitting ? pageT("actions.creating", "Creating...") : pageT("actions.create", "Create")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-6">
        <SurfaceCard className="space-y-6">
          <div>
            <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">
              {pageT("create.cardTitle", "Create platform open-app")}
            </h2>
            <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
              {pageT("create.description", "Create the external integration identity first. After creation, the one-time app key reveal is shown and grants can be edited on the dedicated edit page.")}
            </p>
          </div>
          <OpenAppMetadataFormFields
            draft={metadataDraft}
            showStatus={false}
            onChange={setMetadataDraft}
          />
        </SurfaceCard>

        <SurfaceCard className="space-y-3">
          <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">
            {pageT("create.nextTitle", "What happens next")}
          </h2>
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            {pageT("create.nextDescription", "A new Open App Code and App Key are generated. The cleartext App Key is shown once on the detail page immediately after create. After that reveal, only the fingerprint remains visible and future changes use key rotation.")}
          </p>
        </SurfaceCard>
      </div>
    </>
  );
}

import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";
import {
  PageHeader,
  SurfaceCard,
} from "@/shared";
import { useToast } from "@/components/ui/toast";

import {
  createPlatformModel,
  fetchPlatformModels,
  verifyPlatformModelDraft,
  type ModelMutationPayload,
} from "@/domain/model/api";
import {
  ModelFormFields,
  type ModelFormState,
  emptyModelFormState,
  modelFormToPayload,
} from "@/domain/model/components";
import {
  createPlatformModelRecord,
  listPlatformModelsMock,
  verifyPlatformModelCreateDraftRecord,
} from "@/domain/model/mock";
import type { ModelRecord } from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function platformPayload(
  form: ModelFormState,
  apiKeyRef: string,
  enabled: boolean,
): ModelMutationPayload {
  return modelFormToPayload(form, apiKeyRef, { enabled });
}

export function CreatePlatformModelPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation(["pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`modelRegistry.${key}`, { defaultValue, ...options })
  );
  const dataSource = getServiceDataSource("model");
  const [records, setRecords] = useState<ModelRecord[]>([]);
  const [form, setForm] = useState<ModelFormState>(emptyModelFormState());
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const apiKeyInputRef = useRef<HTMLInputElement | null>(null);

  const duplicateDisplayName = form.displayName.trim()
    ? records.some((record) => record.displayName === form.displayName.trim())
    : false;

  useEffect(() => {
    let active = true;

    async function loadPlatformModels() {
      setLoading(true);
      try {
        const nextRecords = dataSource === "api"
          ? await fetchPlatformModels()
          : listPlatformModelsMock();
        if (active) setRecords(nextRecords);
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : pageT("errors.load", "Platform models could not be loaded"));
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadPlatformModels();

    return () => {
      active = false;
    };
  }, [dataSource, toast]);

  async function saveModel(options?: { verifyAfterCreate?: boolean }) {
    if (duplicateDisplayName) {
      toast.error(pageT("errors.duplicatePlatformDisplayName", "displayName already exists in platform models"));
      return;
    }
    const apiKeyRef = apiKeyInputRef.current?.value.trim() ?? "";
    if (!apiKeyRef) {
      toast.error(pageT("errors.apiKeyRequired", "apiKey is required"));
      return;
    }

    const verifyAfterCreate = Boolean(options?.verifyAfterCreate);
    setSubmitting(!verifyAfterCreate);
    setVerifying(verifyAfterCreate);
    try {
      const payload = platformPayload(form, apiKeyRef, true);
      if (verifyAfterCreate) {
        const verified = dataSource === "api"
          ? await verifyPlatformModelDraft(payload)
          : verifyPlatformModelCreateDraftRecord(payload);
        toast.success(pageT("messages.verified", "{{name}} verified", { name: verified.displayName }));
        return;
      }

      const saved = dataSource === "api"
        ? await createPlatformModel(payload)
        : createPlatformModelRecord(payload);
      if (!verifyAfterCreate) {
        toast.success(pageT("messages.created", "{{name}} created", { name: saved.displayName }));
      }
      navigate(`/settings/models/detail?modelCode=${encodeURIComponent(saved.modelCode)}`);
    } catch (error) {
      toast.error(error instanceof Error
        ? error.message
        : (verifyAfterCreate ? pageT("errors.verify", "Verify failed") : pageT("errors.createPlatform", "Platform model could not be created")));
    } finally {
      setSubmitting(false);
      setVerifying(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: pageT("title", "Models"), href: "/settings/models" }, { label: pageT("actions.new", "New") }]}
        title={pageT("detail.createPlatformModel", "Create Platform Model")}
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/settings/models")}>{pageT("actions.cancel", "Cancel")}</Button>
            <Button
              variant="secondary"
              disabled={loading || submitting || verifying || !form.displayName.trim()}
              aria-busy={verifying}
              onClick={() => void saveModel({ verifyAfterCreate: true })}
            >

              {verifying ? pageT("actions.verifying", "Verifying...") : pageT("actions.verify", "Verify")}
            </Button>
            <Button
              disabled={loading || submitting || verifying || !form.displayName.trim()}
              aria-busy={submitting}
              onClick={() => void saveModel()}
            >

              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl">
        <SurfaceCard>
          <ModelFormFields
            state={form}
            onChange={setForm}
            apiKeyInputRef={apiKeyInputRef}
            syncModelNameFromDisplayName
            displayNameError={duplicateDisplayName ? pageT("errors.duplicatePlatformDisplayName", "displayName already exists in platform models") : undefined}
          />
        </SurfaceCard>
      </div>
    </>
  );
}

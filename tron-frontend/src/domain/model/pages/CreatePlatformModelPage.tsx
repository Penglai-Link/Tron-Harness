import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  Save,
  Zap,
} from "lucide-react";

import { Button } from "@/shared/ui/Button";
import {
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

import {
  createPlatformModel,
  fetchPlatformModels,
  verifyPlatformModel,
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
  verifyPlatformModelRecord,
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
        if (active) toast.error(error instanceof Error ? error.message : "Platform models could not be loaded");
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
      toast.error("displayName already exists in platform models");
      return;
    }
    const apiKeyRef = apiKeyInputRef.current?.value.trim() ?? "";
    if (!apiKeyRef) {
      toast.error("apiKey is required");
      return;
    }

    setSubmitting(true);
    setVerifying(Boolean(options?.verifyAfterCreate));
    try {
      const saved = dataSource === "api"
        ? await createPlatformModel(platformPayload(form, apiKeyRef, true))
        : createPlatformModelRecord(platformPayload(form, apiKeyRef, true));
      toast.success(`${saved.displayName} created`);
      if (options?.verifyAfterCreate) {
        try {
          const verified = dataSource === "api"
            ? await verifyPlatformModel(saved.modelCode)
            : verifyPlatformModelRecord(saved.modelCode);
          toast.success(`${verified.displayName} verified`);
        } catch (error) {
          toast.error(error instanceof Error ? error.message : "Verify failed");
        }
      }
      navigate(`/settings/models/detail?modelCode=${encodeURIComponent(saved.modelCode)}`);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform model could not be created");
    } finally {
      setSubmitting(false);
      setVerifying(false);
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Models", href: "/settings/models" }, { label: "New" }]}
        title="Create Platform Model"
        actions={(
          <>
            <Button variant="secondary" onClick={() => navigate("/settings/models")}>Cancel</Button>
            <Button
              variant="secondary"
              disabled={loading || submitting || verifying || !form.displayName.trim()}
              aria-busy={verifying}
              onClick={() => void saveModel({ verifyAfterCreate: true })}
            >
              <Zap size={16} aria-hidden="true" />
              {verifying ? "Creating + Verifying..." : "Create and Verify"}
            </Button>
            <Button
              disabled={loading || submitting || verifying || !form.displayName.trim()}
              aria-busy={submitting}
              onClick={() => void saveModel()}
            >
              <Save size={16} aria-hidden="true" />
              {submitting ? "Creating..." : "Create Platform Model"}
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
            displayNameError={duplicateDisplayName ? "displayName already exists in platform models" : undefined}
          />
        </SurfaceCard>
      </div>
    </>
  );
}

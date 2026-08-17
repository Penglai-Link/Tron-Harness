import {
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Navigate,
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  Edit3,
  Save,
  ShieldCheck,
  Trash2,
  X,
  Zap,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  Field,
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

import {
  deletePlatformModel,
  disablePlatformModel,
  enablePlatformModel,
  fetchPlatformModel,
  setPlatformDefaultModel,
  updatePlatformModel,
  verifyPlatformModel,
  type ModelMutationPayload,
} from "@/domain/model/api";
import {
  ModelFormFields,
  ModelSourceBadge,
  VerifyStatusCard,
  type ModelFormState,
  emptyModelFormState,
  modelFormFromRecord,
  modelFormToPayload,
} from "@/domain/model/components";
import {
  formatModelCapabilities,
  formatModelProtocol,
  formatReasoningEffort,
} from "@/domain/model/constants/modelPageOptions";
import {
  deletePlatformModelRecord,
  getPlatformModelRecord,
  setPlatformDefaultRecord,
  setPlatformModelEnabled,
  updatePlatformModelRecord,
  verifyPlatformModelDraftRecord,
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

function ReadValue({ children }: { children: string }) {
  return (
    <div className="min-h-6 text-sm leading-6 text-[var(--color-on-surface)] break-words">
      {children || "None"}
    </div>
  );
}

function formatOptionalNumber(value: number | null) {
  return value === null ? "" : String(value);
}

function formatOptionalText(value: string | null) {
  return value?.trim() ?? "";
}

function maskApiKeyValue(value: string) {
  return value.trim() ? "*****" : "";
}

export function PlatformModelDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation(["common"]);
  const dataSource = getServiceDataSource("model");
  const modelCode = searchParams.get("modelCode") ?? "";
  const startsInEditMode = searchParams.get("mode") === "edit";
  const [selectedRecord, setSelectedRecord] = useState<ModelRecord | null>(null);
  const [form, setForm] = useState<ModelFormState>(emptyModelFormState());
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [verifyPreview, setVerifyPreview] = useState<ModelRecord | null>(null);
  const apiKeyInputRef = useRef<HTMLInputElement | null>(null);

  const verifyStatusRecord = verifyPreview ?? selectedRecord;

  async function refreshPlatformModel() {
    setLoading(true);
    try {
      const nextRecord = dataSource === "api"
        ? await fetchPlatformModel(modelCode)
        : getPlatformModelRecord(modelCode);
      setSelectedRecord(nextRecord);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform models could not be loaded");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!modelCode) return;

    let active = true;

    async function loadPlatformModels() {
      setLoading(true);
      try {
        const nextRecord = dataSource === "api"
          ? await fetchPlatformModel(modelCode)
          : getPlatformModelRecord(modelCode);
        if (active) setSelectedRecord(nextRecord);
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
  }, [dataSource, modelCode, toast]);

  useEffect(() => {
    if (!selectedRecord) return;
    setForm(modelFormFromRecord(selectedRecord));
    setEditing(startsInEditMode);
    setVerifyPreview(null);
  }, [selectedRecord, startsInEditMode]);

  useEffect(() => {
    if (apiKeyInputRef.current) {
      apiKeyInputRef.current.value = "";
    }
    setApiKeyDirty(false);
  }, [editing, selectedRecord?.modelCode]);

  useEffect(() => {
    if (editing) {
      setVerifyPreview(null);
    }
  }, [apiKeyDirty, editing, form]);

  if (!modelCode) {
    return <Navigate replace to="/settings/models" />;
  }

  async function saveModel() {
    if (!selectedRecord) return;

    setSubmitting(true);
    try {
      const apiKeyRef = apiKeyInputRef.current?.value.trim() ?? "";
      const saved = dataSource === "api"
        ? await updatePlatformModel(selectedRecord.modelCode, platformPayload(form, apiKeyRef, selectedRecord.enabled))
        : updatePlatformModelRecord(selectedRecord.modelCode, platformPayload(form, apiKeyRef, selectedRecord.enabled));
      toast.success(`${saved.displayName} saved`);
      setVerifyPreview(null);
      exitEditing();
      await refreshPlatformModel();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform model could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  async function runVerify() {
    if (!selectedRecord) return;
    setVerifying(true);
    const verifyTarget = editing ? (form.displayName.trim() || selectedRecord.displayName) : selectedRecord.displayName;
    const verifyingToastId = toast.info(`Verifying connectivity for ${verifyTarget}...`, {
      durationMs: 0,
      title: "Verify",
    });
    try {
      const payload = editing
        ? platformPayload(form, apiKeyInputRef.current?.value.trim() ?? "", selectedRecord.enabled)
        : undefined;
      const verified = dataSource === "api"
        ? await verifyPlatformModel(selectedRecord.modelCode, payload)
        : (payload
          ? verifyPlatformModelDraftRecord(selectedRecord.modelCode, payload)
          : verifyPlatformModelRecord(selectedRecord.modelCode));
      toast.dismiss(verifyingToastId);
      if (editing) {
        setVerifyPreview(verified);
      } else {
        setVerifyPreview(null);
        await refreshPlatformModel();
      }
      toast.success(`${verified.displayName} verified`);
    } catch (error) {
      toast.dismiss(verifyingToastId);
      toast.error(error instanceof Error ? error.message : "Verify failed");
    } finally {
      setVerifying(false);
    }
  }

  function resetDraft() {
    if (!selectedRecord) return;
    setForm(modelFormFromRecord(selectedRecord));
    setVerifyPreview(null);
    exitEditing();
  }

  function exitEditing() {
    setEditing(false);
    navigate(`/settings/models/detail?modelCode=${encodeURIComponent(modelCode)}`, { replace: true });
  }

  async function toggleEnabled() {
    if (!selectedRecord) return;
    try {
      const toggled = dataSource === "api"
        ? (selectedRecord.enabled
          ? await disablePlatformModel(selectedRecord.modelCode)
          : await enablePlatformModel(selectedRecord.modelCode))
        : setPlatformModelEnabled(selectedRecord.modelCode, !selectedRecord.enabled);
      toast.success(`${toggled.displayName} ${toggled.enabled ? "enabled" : "disabled"}`);
      await refreshPlatformModel();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Status update failed");
    }
  }

  async function removeModel() {
    if (!selectedRecord) return;
    setDeleting(true);
    try {
      if (dataSource === "api") {
        await deletePlatformModel(selectedRecord.modelCode);
      } else {
        deletePlatformModelRecord(selectedRecord.modelCode);
      }
      toast.success(`${selectedRecord.displayName} deleted`);
      navigate("/settings/models");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform model could not be deleted");
    } finally {
      setDeleting(false);
    }
  }

  async function makeDefault() {
    if (!selectedRecord || selectedRecord.isDefault || !selectedRecord.enabled) return;
    setSubmitting(true);
    try {
      if (dataSource === "api") {
        await setPlatformDefaultModel(selectedRecord.modelCode);
      } else {
        setPlatformDefaultRecord(selectedRecord.modelCode);
      }
      toast.success(`${selectedRecord.displayName} set as default`);
      await refreshPlatformModel();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Default model could not be updated");
    } finally {
      setSubmitting(false);
    }
  }

  if (!selectedRecord && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Models", href: "/settings/models" }, { label: "Loading" }]}
          title="Platform Model"
        />
        <SurfaceCard>Loading platform model...</SurfaceCard>
      </>
    );
  }

  if (!selectedRecord) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Models", href: "/settings/models" }, { label: "Not Found" }]}
          title="Platform Model"
        />
        <SurfaceCard>Platform model not found.</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Models", href: "/settings/models" }, { label: selectedRecord.displayName }]}
        title={selectedRecord.displayName}
        actions={editing ? (
          <>
            <Button
              variant="secondary"
              disabled={submitting || verifying || !form.displayName.trim()}
              aria-busy={verifying}
              onClick={() => void runVerify()}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              {verifying ? "Verifying..." : "Verify"}
            </Button>
            <Button variant="secondary" disabled={submitting || verifying} onClick={resetDraft}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
            <Button
              disabled={submitting || verifying || !form.displayName.trim()}
              aria-busy={submitting}
              onClick={() => void saveModel()}
            >
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={() => setEditing(true)}>
              <Edit3 size={16} aria-hidden="true" />
              Edit
            </Button>
            <Button variant="secondary" disabled={verifying} aria-busy={verifying} onClick={() => void runVerify()}>
              <ShieldCheck size={16} aria-hidden="true" />
              {verifying ? "Verifying..." : "Verify"}
            </Button>
            {!selectedRecord.isDefault && selectedRecord.enabled ? (
              <Button variant="secondary" disabled={submitting || verifying} onClick={() => void makeDefault()}>
                <Save size={16} aria-hidden="true" />
                Set Default
              </Button>
            ) : null}
            <Button variant="secondary" onClick={() => void toggleEnabled()}>
              <Zap size={16} aria-hidden="true" />
              {selectedRecord.enabled ? "Disable" : "Enable"}
            </Button>
            <Button variant="ghost" disabled={deleting} aria-busy={deleting} onClick={() => void removeModel()}>
              <Trash2 size={16} aria-hidden="true" />
              {deleting ? "Deleting..." : "Delete"}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        <VerifyStatusCard record={verifyStatusRecord} />

        <SurfaceCard className="space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <ModelSourceBadge sourceKind={selectedRecord.sourceKind} label={selectedRecord.sourceLabel} />
            <Badge tone={selectedRecord.enabled ? "success" : "info"}>{selectedRecord.enabled ? "Enabled" : "Disabled"}</Badge>
            {selectedRecord.isDefault ? <Badge tone="info">Default</Badge> : null}
            <code className="rounded-md bg-[var(--color-surface-container-low)] px-2 py-1 text-[13px] text-[var(--color-on-surface-variant)]">
              {selectedRecord.modelCode}
            </code>
          </div>

          {editing ? (
            <ModelFormFields
              state={form}
              onChange={setForm}
              apiKeyInputRef={apiKeyInputRef}
              apiKeyFieldHint="Leave blank to keep the current key."
              onApiKeyDirtyChange={setApiKeyDirty}
            />
          ) : (
            <div className="grid gap-x-8 gap-y-5 md:grid-cols-2">
              <Field label="Display Name">
                <ReadValue>{selectedRecord.displayName}</ReadValue>
              </Field>
              <Field label="Vendor Model ID">
                <ReadValue>{selectedRecord.modelName}</ReadValue>
              </Field>
              <Field label="Vendor">
                <ReadValue>{selectedRecord.vendor}</ReadValue>
              </Field>
              <Field label="Protocol">
                <ReadValue>{formatModelProtocol(selectedRecord.protocol, selectedRecord.modelType)}</ReadValue>
              </Field>
              <Field label="Model Type">
                <ReadValue>{formatModelCapabilities(selectedRecord.modelCapabilities, selectedRecord.modelType)}</ReadValue>
              </Field>
              <Field label="Base URL">
                <ReadValue>{selectedRecord.baseUrl}</ReadValue>
              </Field>
              <Field label="API Path">
                <ReadValue>{formatOptionalText(selectedRecord.apiPath)}</ReadValue>
              </Field>
              <Field label="API Key">
                <ReadValue>{maskApiKeyValue(selectedRecord.apiKeyRef)}</ReadValue>
              </Field>
              <div className="md:col-span-2">
                <Field label="Description">
                  <ReadValue>{formatOptionalText(selectedRecord.description)}</ReadValue>
                </Field>
              </div>
              <Field label="Temperature">
                <ReadValue>{formatOptionalNumber(selectedRecord.temperature)}</ReadValue>
              </Field>
              <Field label="Max Tokens">
                <ReadValue>{selectedRecord.maxTokens === null ? "" : String(selectedRecord.maxTokens)}</ReadValue>
              </Field>
              <Field label="Context Window">
                <ReadValue>{selectedRecord.contextWindowTokens === null ? "" : String(selectedRecord.contextWindowTokens)}</ReadValue>
              </Field>
              <Field label="Top P">
                <ReadValue>{formatOptionalNumber(selectedRecord.topP)}</ReadValue>
              </Field>
              <Field label="Max Retries">
                <ReadValue>{selectedRecord.maxRetries === null ? "" : String(selectedRecord.maxRetries)}</ReadValue>
              </Field>
              <Field label="Thinking Enabled">
                <ReadValue>{selectedRecord.thinkingEnabled ? "Enabled" : "Disabled"}</ReadValue>
              </Field>
              <Field label={t("common:reasoningEffort.supportedLabel", "Supported reasoning efforts")}>
                <ReadValue>
                  {selectedRecord.supportedReasoningEfforts.map(formatReasoningEffort).join(", ")}
                </ReadValue>
              </Field>
              <Field label={t("common:reasoningEffort.defaultLabel", "Default reasoning effort")}>
                <ReadValue>
                  {selectedRecord.defaultReasoningEffort
                    ? formatReasoningEffort(selectedRecord.defaultReasoningEffort)
                    : t("common:reasoningEffort.providerDefault", "Provider default")}
                </ReadValue>
              </Field>
              <Field label="Structured Output Enabled">
                <ReadValue>{selectedRecord.structuredOutputEnabled ? "Enabled" : "Disabled"}</ReadValue>
              </Field>
              {selectedRecord.protocol === "RESPONSES" ? (
                <Field label={t("common:responsesState.label", "Responses State")}>
                  <ReadValue>
                    {selectedRecord.responsesStatefulEnabled
                      ? t("common:responsesState.stateful", "Stateful")
                      : t("common:responsesState.stateless", "Stateless")}
                  </ReadValue>
                </Field>
              ) : null}
            </div>
          )}
        </SurfaceCard>
      </div>
    </>
  );
}

import {
  useEffect,
  useRef,
  useState,
} from "react";
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
  ToggleSwitch,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  deleteWorkspaceModel,
  disableWorkspaceModel,
  enableWorkspaceModel,
  fetchWorkspaceModelCatalog,
  updateWorkspaceModel,
  verifyWorkspaceModel,
} from "@/domain/model/api";
import {
  ModelFormFields,
  ModelSourceBadge,
  VerifyStatusCard,
  emptyModelFormState,
  modelFormFromRecord,
  modelFormToPayload,
  type ModelFormState,
} from "@/domain/model/components";
import {
  formatModelCapabilities,
  formatModelProtocol,
} from "@/domain/model/constants/modelPageOptions";
import {
  deleteWorkspaceModelRecord,
  getWorkspaceModelCatalog,
  setWorkspaceModelEnabled,
  updateWorkspaceModelRecord,
  verifyWorkspaceModelDraftRecord,
  verifyWorkspaceModelRecord,
} from "@/domain/model/mock";
import type { ModelRecord } from "@/domain/model/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  isEditableWorkspaceLocalRecord,
  isEditableWorkspaceDefinitionRecord,
} from "./workspaceModelPageHelpers";

function ReadValue({ children }: { children: string }) {
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
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

function formatResponsesState(value: boolean) {
  return value ? "有状态" : "无状态";
}

export function WorkspaceModelDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("model");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const modelCode = searchParams.get("modelCode") ?? "";
  const [records, setRecords] = useState<ModelRecord[]>([]);
  const [form, setForm] = useState<ModelFormState>(emptyModelFormState());
  const [allowInheritance, setAllowInheritance] = useState(true);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [verifyPreview, setVerifyPreview] = useState<ModelRecord | null>(null);
  const apiKeyInputRef = useRef<HTMLInputElement | null>(null);

  const selectedRecord = records.find((record) => record.modelCode === modelCode) ?? null;
  const editableLocal = isEditableWorkspaceLocalRecord(selectedRecord, activeWorkspaceCode);
  const editableDefinition = isEditableWorkspaceDefinitionRecord(selectedRecord, activeWorkspaceCode);
  const verifiableRecord = Boolean(selectedRecord && selectedRecord.workspaceCode === activeWorkspaceCode);
  const toggleableRecord = verifiableRecord;
  const removableRecord = Boolean(
    selectedRecord
    && (selectedRecord.sourceKind === "LOCAL" || selectedRecord.sourceKind === "OVERRIDE"),
  );
  const duplicateDisplayName = form.displayName.trim()
    ? records.some((record) => (
      record.workspaceCode === activeWorkspaceCode
      && record.modelCode !== modelCode
      && record.displayName === form.displayName.trim()
    ))
    : false;
  const verifyStatusRecord = verifyPreview ?? selectedRecord;

  async function refreshWorkspaceModels() {
    if (!activeWorkspaceCode) {
      setRecords([]);
      return;
    }

    setLoading(true);
    try {
      const nextCatalog = dataSource === "api"
        ? await fetchWorkspaceModelCatalog(activeWorkspaceCode)
        : getWorkspaceModelCatalog(activeWorkspaceCode);
      setRecords(nextCatalog.records);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Workspace models could not be loaded");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!activeWorkspaceCode || !modelCode) {
      setRecords([]);
      return;
    }

    let active = true;

    async function loadWorkspaceModels() {
      setLoading(true);
      try {
        const nextCatalog = dataSource === "api"
          ? await fetchWorkspaceModelCatalog(activeWorkspaceCode)
          : getWorkspaceModelCatalog(activeWorkspaceCode);
        if (active) setRecords(nextCatalog.records);
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : "Workspace models could not be loaded");
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadWorkspaceModels();

    return () => {
      active = false;
    };
  }, [activeWorkspaceCode, dataSource, modelCode, toast]);

  useEffect(() => {
    if (!selectedRecord) return;
    setForm(modelFormFromRecord(selectedRecord));
    setAllowInheritance(selectedRecord.allowInheritance ?? true);
    setEditing(false);
    setVerifyPreview(null);
  }, [selectedRecord]);

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
  }, [apiKeyDirty, allowInheritance, editing, form]);

  if (!modelCode) {
    return <Navigate replace to="/models" />;
  }

  async function saveModel() {
    if (!selectedRecord || !activeWorkspaceCode || !editableDefinition) return;
    if (duplicateDisplayName) {
      toast.error(`displayName already exists in workspace ${activeWorkspaceCode}`);
      return;
    }

    setSubmitting(true);
    try {
      const apiKeyRef = apiKeyInputRef.current?.value.trim() ?? "";
      const payload = modelFormToPayload(form, apiKeyRef, {
        enabled: selectedRecord.enabled,
        allowInheritance: editableLocal ? allowInheritance : undefined,
      });
      const saved = dataSource === "api"
        ? await updateWorkspaceModel(activeWorkspaceCode, selectedRecord.modelCode, payload)
        : updateWorkspaceModelRecord(activeWorkspaceCode, selectedRecord.modelCode, payload);
      toast.success(`${saved.displayName} saved`);
      setVerifyPreview(null);
      setEditing(false);
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Workspace model could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  async function runVerify() {
    if (!selectedRecord || !activeWorkspaceCode || !verifiableRecord) return;
    setVerifying(true);
    const verifyTarget = editing ? (form.displayName.trim() || selectedRecord.displayName) : selectedRecord.displayName;
    const verifyingToastId = toast.info(`Verifying connectivity for ${verifyTarget}...`, {
      durationMs: 0,
      title: "Verify",
    });
    try {
      const payload = editing
        ? modelFormToPayload(form, apiKeyInputRef.current?.value.trim() ?? "", {
          enabled: selectedRecord.enabled,
          allowInheritance: editableLocal ? allowInheritance : undefined,
        })
        : undefined;
      const verified = dataSource === "api"
        ? await verifyWorkspaceModel(activeWorkspaceCode, selectedRecord.modelCode, payload)
        : (payload
          ? verifyWorkspaceModelDraftRecord(activeWorkspaceCode, selectedRecord.modelCode, payload)
          : verifyWorkspaceModelRecord(activeWorkspaceCode, selectedRecord.modelCode));
      toast.dismiss(verifyingToastId);
      if (editing) {
        setVerifyPreview(verified);
      } else {
        setVerifyPreview(null);
        await refreshWorkspaceModels();
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
    setAllowInheritance(selectedRecord.allowInheritance ?? true);
    setVerifyPreview(null);
    setEditing(false);
  }

  async function toggleEnabled() {
    if (!selectedRecord || !activeWorkspaceCode || !toggleableRecord) return;
    try {
      const toggled = dataSource === "api"
        ? (selectedRecord.enabled
          ? await disableWorkspaceModel(activeWorkspaceCode, selectedRecord.modelCode)
          : await enableWorkspaceModel(activeWorkspaceCode, selectedRecord.modelCode))
        : setWorkspaceModelEnabled(activeWorkspaceCode, selectedRecord.modelCode, !selectedRecord.enabled);
      toast.success(`${toggled.displayName} ${toggled.enabled ? "enabled" : "disabled"}`);
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Status update failed");
    }
  }

  async function removeModel() {
    if (!selectedRecord || !activeWorkspaceCode || !removableRecord) return;
    setDeleting(true);
    try {
      if (dataSource === "api") {
        await deleteWorkspaceModel(activeWorkspaceCode, selectedRecord.modelCode);
      } else {
        deleteWorkspaceModelRecord(activeWorkspaceCode, selectedRecord.modelCode);
      }
      toast.success(
        selectedRecord.sourceKind === "OVERRIDE"
          ? `${selectedRecord.displayName} reverted to inherited`
          : `${selectedRecord.displayName} deleted`,
      );
      navigate("/models");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Workspace model could not be removed");
    } finally {
      setDeleting(false);
    }
  }

  if (!activeWorkspaceCode) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Models", href: "/models" }, { label: "Workspace" }]}
          title="Workspace Model"
        />
        <SurfaceCard>Select a workspace in the shell to load this model.</SurfaceCard>
      </>
    );
  }

  if (!selectedRecord && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Models", href: "/models" }, { label: "Loading" }]}
          title="Workspace Model"
        />
        <SurfaceCard>Loading workspace model...</SurfaceCard>
      </>
    );
  }

  if (!selectedRecord) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Models", href: "/models" }, { label: "Not Found" }]}
          title="Workspace Model"
        />
        <SurfaceCard>Workspace model not found.</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Models", href: "/models" }, { label: selectedRecord.displayName }]}
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
            {editableDefinition ? (
              <Button variant="secondary" onClick={() => setEditing(true)}>
                <Edit3 size={16} aria-hidden="true" />
                Edit
              </Button>
            ) : null}
            {verifiableRecord ? (
              <Button variant="secondary" disabled={verifying} aria-busy={verifying} onClick={() => void runVerify()}>
                <ShieldCheck size={16} aria-hidden="true" />
                {verifying ? "Verifying..." : "Verify"}
              </Button>
            ) : null}
            {toggleableRecord ? (
              <Button variant="secondary" disabled={submitting} onClick={() => void toggleEnabled()}>
                <Zap size={16} aria-hidden="true" />
                {selectedRecord.enabled ? "Disable" : "Enable"}
              </Button>
            ) : null}
            {removableRecord ? (
              <Button variant="ghost" disabled={deleting} aria-busy={deleting} onClick={() => void removeModel()}>
                <Trash2 size={16} aria-hidden="true" />
                {deleting ? "Deleting..." : (selectedRecord.sourceKind === "OVERRIDE" ? "Revert" : "Delete")}
              </Button>
            ) : null}
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        <VerifyStatusCard record={verifyStatusRecord} />

        <SurfaceCard className="space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <ModelSourceBadge sourceKind={selectedRecord.sourceKind} label={selectedRecord.sourceLabel} />
            <Badge tone={selectedRecord.enabled ? "success" : "neutral"}>{selectedRecord.enabled ? "Enabled" : "Disabled"}</Badge>
            {selectedRecord.isDefault ? <Badge tone="accent">Default</Badge> : null}
            <code className="rounded-md bg-[var(--color-surface-container-low)] px-2 py-1 text-[13px] text-[var(--color-on-surface-variant)]">
              {selectedRecord.modelCode}
            </code>
          </div>

          {editing ? (
            <div className="space-y-5">
              <ModelFormFields
                state={form}
                onChange={setForm}
                apiKeyInputRef={apiKeyInputRef}
                apiKeyFieldHint="Leave blank to keep the current key."
                onApiKeyDirtyChange={setApiKeyDirty}
                displayNameError={duplicateDisplayName ? `displayName already exists in workspace ${activeWorkspaceCode}` : undefined}
              />

              <ToggleSwitch
                checked={allowInheritance}
                label="Allow Inheritance"
                onCheckedChange={setAllowInheritance}
              />
            </div>
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
              <Field label="Structured Output Enabled">
                <ReadValue>{selectedRecord.structuredOutputEnabled ? "Enabled" : "Disabled"}</ReadValue>
              </Field>
              {selectedRecord.protocol === "RESPONSES" ? (
                <Field label="Responses State">
                  <ReadValue>{formatResponsesState(selectedRecord.responsesStatefulEnabled)}</ReadValue>
                </Field>
              ) : null}
              {selectedRecord.sourceKind === "LOCAL" ? (
                <Field label="Allow Inheritance">
                  <ReadValue>{selectedRecord.allowInheritance ? "Enabled" : "Disabled"}</ReadValue>
                </Field>
              ) : null}
            </div>
          )}
        </SurfaceCard>
      </div>
    </>
  );
}

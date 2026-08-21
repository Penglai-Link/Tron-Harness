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


import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/components/ui/button";
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
  formatModelProtocol,
  formatReasoningEffort,
  formatModelCapability,
  normalizeModelCapabilities,
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
  formatWorkspaceDisplayLabel,
  formatWorkspaceModelError,
  isEditableWorkspaceLocalRecord,
  isEditableWorkspaceDefinitionRecord,
} from "./workspaceModelPageHelpers";

function ReadValue({ children }: { children: string }) {
  const { t } = useTranslation(["pages"]);
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
      {children || t("modelRegistry.values.none", "None")}
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

export function WorkspaceModelDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation(["common", "pages"]);
  const pageT = (key: string, defaultValue: string, options?: Record<string, unknown>) => (
    t(`pages:modelRegistry.${key}`, { defaultValue, ...options })
  );
  const dataSource = getServiceDataSource("model");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceLabel = formatWorkspaceDisplayLabel(activeWorkspace, activeWorkspaceCode);
  const modelCode = searchParams.get("modelCode") ?? "";
  const startsInEditMode = searchParams.get("mode") === "edit";
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
      toast.error(error instanceof Error ? error.message : pageT("errors.load", "Workspace models could not be loaded"));
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
    setEditing(startsInEditMode && editableDefinition);
    setVerifyPreview(null);
  }, [editableDefinition, selectedRecord, startsInEditMode]);

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
      toast.error(pageT("errors.duplicateDisplayName", "displayName already exists in workspace {{workspace}}", { workspace: activeWorkspaceLabel }));
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
      toast.success(pageT("messages.saved", "{{name}} saved", { name: saved.displayName }));
      setVerifyPreview(null);
      exitEditing();
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(formatWorkspaceModelError(error, activeWorkspaceLabel, pageT("errors.save", "Workspace model could not be saved")));
    } finally {
      setSubmitting(false);
    }
  }

  async function runVerify() {
    if (!selectedRecord || !activeWorkspaceCode || !verifiableRecord) return;
    setVerifying(true);
    const verifyTarget = editing ? (form.displayName.trim() || selectedRecord.displayName) : selectedRecord.displayName;
    const verifyingToastId = toast.info(pageT("messages.verifying", "Verifying connectivity for {{name}}...", { name: verifyTarget }), {
      durationMs: 0,
      title: pageT("actions.verify", "Verify"),
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
      toast.success(pageT("messages.verified", "{{name}} verified", { name: verified.displayName }));
    } catch (error) {
      toast.dismiss(verifyingToastId);
      toast.error(error instanceof Error ? error.message : pageT("errors.verify", "Verify failed"));
    } finally {
      setVerifying(false);
    }
  }

  function resetDraft() {
    if (!selectedRecord) return;
    setForm(modelFormFromRecord(selectedRecord));
    setAllowInheritance(selectedRecord.allowInheritance ?? true);
    setVerifyPreview(null);
    exitEditing();
  }

  function exitEditing() {
    setEditing(false);
    navigate(`/models/detail?modelCode=${encodeURIComponent(modelCode)}`, { replace: true });
  }

  async function toggleEnabled() {
    if (!selectedRecord || !activeWorkspaceCode || !toggleableRecord) return;
    try {
      const toggled = dataSource === "api"
        ? (selectedRecord.enabled
          ? await disableWorkspaceModel(activeWorkspaceCode, selectedRecord.modelCode)
          : await enableWorkspaceModel(activeWorkspaceCode, selectedRecord.modelCode))
        : setWorkspaceModelEnabled(activeWorkspaceCode, selectedRecord.modelCode, !selectedRecord.enabled);
      toast.success(pageT("messages.toggle", "{{name}} {{status}}", { name: toggled.displayName, status: toggled.enabled ? pageT("status.enabledLower", "enabled") : pageT("status.disabledLower", "disabled") }));
      await refreshWorkspaceModels();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.status", "Status update failed"));
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
          ? pageT("messages.reverted", "{{name}} reverted to inherited", { name: selectedRecord.displayName })
          : pageT("messages.deleted", "{{name}} deleted", { name: selectedRecord.displayName }),
      );
      navigate("/models");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : pageT("errors.delete", "Workspace model could not be removed"));
    } finally {
      setDeleting(false);
    }
  }

  if (!activeWorkspaceCode) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: pageT("title", "Models"), href: "/models" }, { label: pageT("source.workspace", "Workspace") }]}
          title={pageT("detail.workspaceModel", "Workspace Model")}
        />
        <SurfaceCard>{pageT("empty.selectWorkspace", "Select a workspace in the shell to load this model.")}</SurfaceCard>
      </>
    );
  }

  if (!selectedRecord && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: pageT("title", "Models"), href: "/models" }, { label: pageT("detail.loading", "Loading") }]}
          title={pageT("detail.workspaceModel", "Workspace Model")}
        />
        <SurfaceCard>{pageT("detail.loadingWorkspaceModel", "Loading workspace model...")}</SurfaceCard>
      </>
    );
  }

  if (!selectedRecord) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: pageT("title", "Models"), href: "/models" }, { label: pageT("detail.notFound", "Not Found") }]}
          title={pageT("detail.workspaceModel", "Workspace Model")}
        />
        <SurfaceCard>{pageT("detail.workspaceModelNotFound", "Workspace model not found.")}</SurfaceCard>
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
              variant="outline"
              size="lg"
              disabled={submitting || verifying || !form.displayName.trim()}
              aria-busy={verifying}
              onClick={() => void runVerify()}
            >
              {verifying ? pageT("actions.verifying", "Verifying...") : pageT("actions.verify", "Verify")}
            </Button>
            <Button
              variant="outline"
              size="lg"
              disabled={submitting || verifying}
              onClick={resetDraft}
            >
              {pageT("actions.cancel", "Cancel")}
            </Button>
            <Button
              size="lg"
              disabled={submitting || verifying || !form.displayName.trim()}
              aria-busy={submitting}
              onClick={() => void saveModel()}
            >
              {submitting ? pageT("actions.saving", "Saving...") : pageT("actions.save", "Save")}
            </Button>
          </>
        ) : (
          <>
            {editableDefinition ? (
              <Button
                variant="outline"
                size="lg"
                onClick={() => setEditing(true)}
              >
                {pageT("actions.edit", "Edit")}
              </Button>
            ) : null}
            {verifiableRecord ? (
              <Button
                variant="outline"
                size="lg"
                disabled={verifying}
                aria-busy={verifying}
                onClick={() => void runVerify()}
              >
                {verifying ? pageT("actions.verifying", "Verifying...") : pageT("actions.verify", "Verify")}
              </Button>
            ) : null}
            {toggleableRecord ? (
              <Button
                variant="outline"
                size="lg"
                disabled={submitting} onClick={() => void toggleEnabled()}
              >
                {selectedRecord.enabled ? pageT("actions.disable", "Disable") : pageT("actions.enable", "Enable")}
              </Button>
            ) : null}
            {removableRecord ? (
              <Button
                variant="destructive"
                size="lg"
                disabled={deleting}
                aria-busy={deleting}
                onClick={() => void removeModel()}
              >
                {deleting ? pageT("actions.deleting", "Deleting...") : (selectedRecord.sourceKind === "OVERRIDE" ? pageT("actions.revert", "Revert") : pageT("actions.delete", "Delete"))}
              </Button>
            ) : null}
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        <VerifyStatusCard record={verifyStatusRecord} />

        <SurfaceCard className="space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <ModelSourceBadge sourceKind={selectedRecord.sourceKind} label={pageT(`source.${selectedRecord.sourceKind.toLowerCase()}`, selectedRecord.sourceLabel)} />
            <Badge tone={selectedRecord.enabled ? "success" : "info"}>{selectedRecord.enabled ? pageT("status.enabled", "Enabled") : pageT("status.disabled", "Disabled")}</Badge>
            {selectedRecord.isDefault ? <Badge tone="info">{pageT("status.default", "Default")}</Badge> : null}
            <Badge tone="outline">{selectedRecord.modelCode}</Badge>
          </div>

          {editing ? (
            <div className="space-y-5">
              <ModelFormFields
                state={form}
                onChange={setForm}
                apiKeyInputRef={apiKeyInputRef}
                apiKeyFieldHint={pageT("form.keepApiKeyHint", "Leave blank to keep the current key.")}
                onApiKeyDirtyChange={setApiKeyDirty}
                displayNameError={duplicateDisplayName ? pageT("errors.duplicateDisplayName", "displayName already exists in workspace {{workspace}}", { workspace: activeWorkspaceLabel }) : undefined}
              />
              <ToggleSwitch
                checked={allowInheritance}
                label={pageT("form.allowInheritance", "Allow Inheritance")}
                onCheckedChange={setAllowInheritance}
              />
            </div>
          ) : (
            <div className="grid gap-x-8 gap-y-5 md:grid-cols-2">
              <Field label={pageT("form.displayName", "Display Name")}>
                <ReadValue>{selectedRecord.displayName}</ReadValue>
              </Field>
              <Field label={pageT("form.vendorModelId", "Vendor Model ID")}>
                <ReadValue>{selectedRecord.modelName}</ReadValue>
              </Field>
              <Field label={pageT("form.vendor", "Vendor")}>
                <ReadValue>{selectedRecord.vendor}</ReadValue>
              </Field>
              <Field label={pageT("form.protocol", "Protocol")}>
                <ReadValue>{pageT(`protocol.${selectedRecord.protocol === "OPENAI_COMPATIBLE" ? `openaiCompatible.${selectedRecord.modelType === "EMBEDDING" ? "embedding" : "chat"}` : selectedRecord.protocol === "RESPONSES" ? "responses" : "rerank"}`, formatModelProtocol(selectedRecord.protocol, selectedRecord.modelType))}</ReadValue>
              </Field>
              <Field label={pageT("form.modelType", "Model Type")}>
                <ReadValue>{normalizeModelCapabilities(selectedRecord.modelCapabilities, selectedRecord.modelType).map((capability) => pageT(`capabilities.${capability}`, formatModelCapability(capability))).join(", ")}</ReadValue>
              </Field>
              <Field label={pageT("form.baseUrl", "Base URL")}>
                <ReadValue>{selectedRecord.baseUrl}</ReadValue>
              </Field>
              <Field label={pageT("form.apiPath", "API Path")}>
                <ReadValue>{formatOptionalText(selectedRecord.apiPath)}</ReadValue>
              </Field>
              <Field label={pageT("form.apiKey", "API Key")}>
                <ReadValue>{maskApiKeyValue(selectedRecord.apiKeyRef)}</ReadValue>
              </Field>
              <div className="md:col-span-2">
                <Field label={pageT("form.description", "Description")}>
                  <ReadValue>{formatOptionalText(selectedRecord.description)}</ReadValue>
                </Field>
              </div>
              <Field label={pageT("form.temperature", "Temperature")}>
                <ReadValue>{formatOptionalNumber(selectedRecord.temperature)}</ReadValue>
              </Field>
              <Field label={pageT("form.maxTokens", "Max Tokens")}>
                <ReadValue>{selectedRecord.maxTokens === null ? "" : String(selectedRecord.maxTokens)}</ReadValue>
              </Field>
              <Field label={pageT("form.contextWindow", "Context Window")}>
                <ReadValue>{selectedRecord.contextWindowTokens === null ? "" : String(selectedRecord.contextWindowTokens)}</ReadValue>
              </Field>
              <Field label={pageT("form.topP", "Top P")}>
                <ReadValue>{formatOptionalNumber(selectedRecord.topP)}</ReadValue>
              </Field>
              <Field label={pageT("form.maxRetries", "Max Retries")}>
                <ReadValue>{selectedRecord.maxRetries === null ? "" : String(selectedRecord.maxRetries)}</ReadValue>
              </Field>
              <Field label={pageT("form.thinkingEnabled", "Thinking Enabled")}>
                <ReadValue>{selectedRecord.thinkingEnabled ? pageT("status.enabled", "Enabled") : pageT("status.disabled", "Disabled")}</ReadValue>
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
              <Field label={pageT("form.structuredOutputEnabled", "Structured Output Enabled")}>
                <ReadValue>{selectedRecord.structuredOutputEnabled ? pageT("status.enabled", "Enabled") : pageT("status.disabled", "Disabled")}</ReadValue>
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
              {selectedRecord.sourceKind === "LOCAL" ? (
                <Field label={pageT("form.allowInheritance", "Allow Inheritance")}>
                  <ReadValue>{selectedRecord.allowInheritance ? pageT("status.enabled", "Enabled") : pageT("status.disabled", "Disabled")}</ReadValue>
                </Field>
              ) : null}
            </div>
          )}
        </SurfaceCard>
      </div>
    </>
  );
}

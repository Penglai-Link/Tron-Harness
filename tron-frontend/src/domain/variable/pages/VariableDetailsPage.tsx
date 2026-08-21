import {
  useEffect,
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
  Trash2,
  Undo2,
  X,
} from "lucide-react";

import {
  createVariable,
  deleteVariable,
  revealWorkspaceVariable,
  updateVariable,
  verifyStepUpPassword,
} from "@/domain/variable/api";
import {
  createVariableRecord,
  deleteVariableRecord,
  revealWorkspaceVariableRecord,
  updateVariableRecord,
  verifyStepUpPasswordRecord,
} from "@/domain/variable/mock/data";
import type { VariableRecord } from "@/domain/variable/model";
import { getVariableSourceTone } from "@/domain/variable/constants";
import { useVariableCatalog } from "@/domain/variable/hooks";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Field,
  PageHeader,
  SurfaceCard,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  ReadValue,
  SensitiveDataBadge,
  SensitiveRevealIconButton,
  SensitiveValueCode,
  StepUpRevealDialog,
  getSensitiveDisplayValue,
  getSensitiveState,
  isSensitiveRecord,
} from "./platformVariablePageHelpers";
import {
  WorkspaceVariableFormFields,
  emptyWorkspaceVariableForm,
  formatWorkspaceVariableTimestamp,
  getWorkspaceVariableInheritedFrom,
  isWorkspaceVariableEditable,
  isWorkspaceVariableRemovable,
  normalizeWorkspaceVariableDescription,
  workspaceVariableFormFromOverride,
  workspaceVariableFormFromRecord,
  type WorkspaceVariableFormState,
} from "./workspaceVariablePageHelpers";

type WorkspaceVariablePageMode = "view" | "edit" | "override";
type StepUpIntent = "edit" | "override" | "reveal";

function sourceLabel(record: VariableRecord) {
  if (record.sourceKind === "Inherited") {
    return record.sourceLabel?.trim() || record.sourceWorkspaceName?.trim() || record.sourceWorkspaceCode;
  }
  if (record.sourceKind === "Override") {
    return "Current workspace override";
  }
  return "Current workspace";
}

export function VariableDetailsPage() {
  const { t } = useTranslation(["pages"]);
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const variableKey = searchParams.get("variableKey") ?? "";
  const requestedMode = searchParams.get("mode");
  const { loading, refresh, variables } = useVariableCatalog(activeWorkspaceCode);
  const selectedRecord = variables.find((record) => record.key === variableKey) ?? null;
  const [mode, setMode] = useState<WorkspaceVariablePageMode>("view");
  const [form, setForm] = useState<WorkspaceVariableFormState>(emptyWorkspaceVariableForm());
  const [submitting, setSubmitting] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [revealedValue, setRevealedValue] = useState("");
  const [stepUpIntent, setStepUpIntent] = useState<StepUpIntent | null>(null);
  const [revealPending, setRevealPending] = useState(false);
  const [revealError, setRevealError] = useState("");

  const editing = mode === "edit" || mode === "override";
  const inheritedWorkspace = selectedRecord ? getWorkspaceVariableInheritedFrom(selectedRecord) : null;

  useEffect(() => {
    if (!selectedRecord) {
      setMode("view");
      setForm(emptyWorkspaceVariableForm());
      return;
    }

    if (requestedMode === "override" && selectedRecord.sourceKind === "Inherited") {
      if (isSensitiveRecord(selectedRecord)) {
        setMode("view");
        setForm(workspaceVariableFormFromOverride(selectedRecord));
        setStepUpIntent("override");
        return;
      }
      setMode("override");
      setForm(workspaceVariableFormFromOverride(selectedRecord));
    } else if (requestedMode === "edit" && isWorkspaceVariableEditable(selectedRecord)) {
      if (isSensitiveRecord(selectedRecord)) {
        setMode("view");
        setForm(workspaceVariableFormFromRecord(selectedRecord));
        setStepUpIntent("edit");
        return;
      }
      setMode("edit");
      setForm(workspaceVariableFormFromRecord(selectedRecord));
    } else {
      setMode("view");
      setForm(workspaceVariableFormFromRecord(selectedRecord));
    }
  }, [requestedMode, selectedRecord]);

  useEffect(() => {
    setRevealedValue("");
    setStepUpIntent(null);
    setRevealError("");
  }, [activeWorkspaceCode, variableKey]);

  if (!variableKey) {
    return <Navigate replace to="/variables" />;
  }

  function detailHref(nextMode?: "edit" | "override") {
    const params = new URLSearchParams({ variableKey });
    if (nextMode) params.set("mode", nextMode);
    return `/variables/detail?${params.toString()}`;
  }

  function beginEdit() {
    if (!selectedRecord || !isWorkspaceVariableEditable(selectedRecord)) return;
    if (isSensitiveRecord(selectedRecord)) {
      setRevealError("");
      setStepUpIntent("edit");
      navigate(detailHref("edit"), { replace: true });
      return;
    }

    setMode("edit");
    setForm(workspaceVariableFormFromRecord(selectedRecord));
    navigate(detailHref("edit"), { replace: true });
  }

  function beginOverride() {
    if (!selectedRecord || selectedRecord.sourceKind !== "Inherited") return;
    if (isSensitiveRecord(selectedRecord)) {
      setRevealError("");
      setStepUpIntent("override");
      navigate(detailHref("override"), { replace: true });
      return;
    }

    setMode("override");
    setForm(workspaceVariableFormFromOverride(selectedRecord));
    navigate(detailHref("override"), { replace: true });
  }

  function exitEditing() {
    setMode("view");
    if (selectedRecord) setForm(workspaceVariableFormFromRecord(selectedRecord));
    setStepUpIntent(null);
    navigate(detailHref(), { replace: true });
  }

  function cancelStepUp() {
    const currentIntent = stepUpIntent;
    setStepUpIntent(null);
    setRevealError("");
    if (currentIntent === "edit" || currentIntent === "override" || requestedMode === "edit" || requestedMode === "override") {
      navigate(detailHref(), { replace: true });
    }
  }

  async function saveVariable() {
    if (!activeWorkspaceCode || !selectedRecord || !editing) return;

    const trimmedKey = mode === "override" ? selectedRecord.key : form.key.trim();
    if (!trimmedKey) {
      toast.error(t("pages:variables.errors.keyRequired", "Variable key is required"));
      return;
    }
    const selectedIsSensitive = isSensitiveRecord(selectedRecord);
    if (selectedIsSensitive && !form.isSensitive && mode === "edit" && !form.value.trim()) {
      toast.error(t("pages:variables.errors.newValueRequired", "New value is required before turning off sensitivity"));
      return;
    }
    if (!selectedIsSensitive && form.isSensitive && mode === "edit" && !form.value.trim()) {
      toast.error(t("pages:variables.errors.sensitiveValueRequired", "Sensitive value is required before turning on sensitivity"));
      return;
    }
    if (selectedIsSensitive && mode === "override" && !form.isSensitive) {
      toast.error(t("pages:variables.errors.inheritedSensitive", "Inherited sensitive variables must stay sensitive when overridden"));
      return;
    }
    if (form.isSensitive && !form.value.trim()) {
      toast.error(t("pages:variables.errors.valueRequired", "Variable value is required"));
      return;
    }

    setSubmitting(true);
    try {
      let saved: VariableRecord;
      if (mode === "override") {
        const payload = {
          key: selectedRecord.key,
          value: form.value,
          description: normalizeWorkspaceVariableDescription(form.description),
          isSensitive: form.isSensitive,
          valueMode: "REPLACE" as const,
        };
        saved = dataSource === "api"
          ? await createVariable(activeWorkspaceCode, payload)
          : createVariableRecord(activeWorkspaceCode, payload);
        toast.success(t("pages:variables.messages.overridden", "{{key}} overridden in {{workspace}}", { key: saved.key, workspace: activeWorkspaceCode }));
      } else {
        const payload = {
          description: normalizeWorkspaceVariableDescription(form.description),
          isSensitive: form.isSensitive,
          value: form.value,
          valueMode: "REPLACE" as const,
        };
        saved = dataSource === "api"
          ? await updateVariable(activeWorkspaceCode, selectedRecord.key, payload)
          : updateVariableRecord(activeWorkspaceCode, selectedRecord.key, payload);
        toast.success(t("pages:variables.messages.saved", "{{key}} saved", { key: saved.key }));
      }

      setRevealedValue("");
      refresh();
      setMode("view");
      navigate(`/variables/detail?variableKey=${encodeURIComponent(saved.key)}`, { replace: true });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:variables.errors.save", "Variable could not be saved"));
    } finally {
      setSubmitting(false);
    }
  }

  async function removeVariable() {
    if (!activeWorkspaceCode || !selectedRecord || !isWorkspaceVariableRemovable(selectedRecord)) return;
    setDeleting(true);
    try {
      if (dataSource === "api") {
        await deleteVariable(activeWorkspaceCode, selectedRecord.key);
      } else {
        deleteVariableRecord(activeWorkspaceCode, selectedRecord.key);
      }
      setRevealedValue("");
      refresh();
      if (selectedRecord.sourceKind === "Override") {
        toast.success(t("pages:variables.messages.reverted", "{{key}} reverted to inherited value", { key: selectedRecord.key }));
        navigate(detailHref(), { replace: true });
      } else {
        toast.success(t("pages:variables.messages.deleted", "{{key}} deleted", { key: selectedRecord.key }));
        navigate("/variables");
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:variables.errors.remove", "Variable could not be removed"));
    } finally {
      setDeleting(false);
    }
  }

  async function revealSensitiveValue(password: string) {
    if (!selectedRecord || !activeWorkspaceCode) return;
    const intent = stepUpIntent ?? "reveal";
    setRevealPending(true);
    setRevealError("");
    try {
      const stepUp = dataSource === "api"
        ? await verifyStepUpPassword({
          password,
          target: {
            scope: "workspace",
            workspaceCode: activeWorkspaceCode,
            variableKey: selectedRecord.key,
          },
        })
        : verifyStepUpPasswordRecord({
          password,
          target: {
            scope: "workspace",
            workspaceCode: activeWorkspaceCode,
            variableKey: selectedRecord.key,
          },
        });
      const revealed = dataSource === "api"
        ? await revealWorkspaceVariable({
          workspaceCode: activeWorkspaceCode,
          variableKey: selectedRecord.key,
          stepUpToken: stepUp.stepUpToken,
        })
        : revealWorkspaceVariableRecord(activeWorkspaceCode, selectedRecord.key, stepUp.stepUpToken);
      if (intent === "edit") {
        setMode("edit");
        setForm({
          ...workspaceVariableFormFromRecord(selectedRecord),
          value: revealed.value,
          valueMode: "REPLACE",
        });
        setRevealedValue("");
      } else if (intent === "override") {
        setMode("override");
        setForm({
          ...workspaceVariableFormFromOverride(selectedRecord),
          value: revealed.value,
          valueMode: "REPLACE",
        });
        setRevealedValue("");
      } else {
        setRevealedValue(revealed.value);
      }
      setStepUpIntent(null);
      setRevealError("");
    } catch (error) {
      setRevealError(error instanceof Error ? error.message : t("pages:variables.errors.reveal", "Sensitive value could not be revealed"));
    } finally {
      setRevealPending(false);
    }
  }

  if (!activeWorkspaceCode) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:variables.title", "Variables"), href: "/variables" }, { label: t("pages:variables.workspace", "Workspace") }]}
          title={t("pages:variables.detailTitle", "Variable")}
        />
        <SurfaceCard>{t("pages:variables.empty.workspaceDetail", "Select a workspace in the shell to load this variable.")}</SurfaceCard>
      </>
    );
  }

  if (!selectedRecord && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:variables.title", "Variables"), href: "/variables" }, { label: t("pages:variables.loading", "Loading") }]}
          title={t("pages:variables.detailTitle", "Variable")}
        />
        <SurfaceCard>{t("pages:variables.loadingValue", "Loading variable...")}</SurfaceCard>
      </>
    );
  }

  if (!selectedRecord) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: t("pages:variables.title", "Variables"), href: "/variables" }, { label: t("pages:variables.notFound", "Not Found") }]}
          title={t("pages:variables.detailTitle", "Variable")}
        />
        <SurfaceCard>{t("pages:variables.empty.notFound", "Variable not found in the current workspace.")}</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: t("pages:variables.title", "Variables"), href: "/variables" }, { label: selectedRecord.key }]}
        title={mode === "override" ? t("pages:variables.overrideTitle", "Override {{key}}", { key: selectedRecord.key }) : selectedRecord.key}
        actions={editing ? (
          <>
            <Button variant="secondary" disabled={submitting} onClick={exitEditing}>
              <X size={16} aria-hidden="true" />
              {t("pages:variables.actions.cancel", "Cancel")}
            </Button>
            <Button disabled={submitting || !form.key.trim()} aria-busy={submitting} onClick={() => void saveVariable()}>
              <Save size={16} aria-hidden="true" />
              {submitting ? t("pages:variables.actions.saving", "Saving...") : t("pages:variables.actions.save", "Save")}
            </Button>
          </>
        ) : (
          <>
            {selectedRecord.sourceKind === "Inherited" ? (
              <Button variant="secondary" onClick={beginOverride}>
                <Undo2 size={16} aria-hidden="true" />
                {t("pages:variables.actions.override", "Override")}
              </Button>
            ) : null}
            {isWorkspaceVariableEditable(selectedRecord) ? (
              <Button variant="secondary" onClick={beginEdit}>
                <Edit3 size={16} aria-hidden="true" />
                {t("pages:variables.actions.edit", "Edit")}
              </Button>
            ) : null}
            {isWorkspaceVariableRemovable(selectedRecord) ? (
              <Button variant="ghost" disabled={deleting} aria-busy={deleting} onClick={() => void removeVariable()}>
                <Trash2 size={16} aria-hidden="true" />
                {deleting ? t("pages:variables.actions.deleting", "Deleting...") : (selectedRecord.sourceKind === "Override" ? t("pages:variables.actions.revert", "Revert") : t("pages:variables.actions.delete", "Delete"))}
              </Button>
            ) : null}
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        <SurfaceCard className="space-y-6">
          {editing ? (
            <WorkspaceVariableFormFields
              disableSensitiveDataToggle={mode === "override" && isSensitiveRecord(selectedRecord)}
              form={form}
              keyEditable={false}
              mode={mode === "override" ? "override" : "edit"}
              onChange={setForm}
            />
          ) : (
            <div className="grid gap-x-8 gap-y-6 md:grid-cols-2">
              <Field label={t("pages:variables.form.key", "Variable Key")}>
                <ReadValue>
                  <code>{selectedRecord.key}</code>
                </ReadValue>
              </Field>
              <Field label={t("pages:variables.form.sensitive", "Sensitive Data")}>
                <SensitiveDataBadge sensitive={isSensitiveRecord(selectedRecord)} />
              </Field>
              <Field label={t("pages:variables.detail.source", "Source")}>
                <div className="flex min-h-6 flex-wrap items-center gap-2">
                  <Badge tone={getVariableSourceTone(selectedRecord.sourceKind)}>{selectedRecord.sourceKind}</Badge>
                  <span className="text-sm text-[var(--color-on-surface)]">{sourceLabel(selectedRecord)}</span>
                </div>
              </Field>
              {inheritedWorkspace ? (
                <Field label={selectedRecord.sourceKind === "Override" ? t("pages:variables.detail.overrides", "Overrides") : t("pages:variables.detail.inheritedFrom", "Inherited From")}>
                  <ReadValue>
                    {inheritedWorkspace.label}
                    {inheritedWorkspace.code ? ` (${inheritedWorkspace.code})` : ""}
                  </ReadValue>
                </Field>
              ) : null}
              <Field label={t("pages:variables.detail.created", "Created")}>
                <ReadValue>{formatWorkspaceVariableTimestamp(selectedRecord.createdAt)}</ReadValue>
              </Field>
              <Field label={t("pages:variables.detail.updated", "Updated")}>
                <ReadValue>{formatWorkspaceVariableTimestamp(selectedRecord.updatedAt)}</ReadValue>
              </Field>
              <div className="md:col-span-2">
                <Field className="w-full" label={t("pages:variables.form.value", "Variable Value")}>
                  <div className="min-h-32 w-full rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <SensitiveValueCode
                        className="min-w-0 flex-1"
                        state={getSensitiveState(isSensitiveRecord(selectedRecord), revealedValue)}
                        testId={`workspace-sensitive-value-${selectedRecord.key}`}
                      >
                        {getSensitiveDisplayValue(selectedRecord, revealedValue)}
                      </SensitiveValueCode>
                      {isSensitiveRecord(selectedRecord) ? (
                        <SensitiveRevealIconButton
                          revealed={Boolean(revealedValue)}
                          variableKey={selectedRecord.key}
                          onClick={() => {
                            if (revealedValue) {
                              setRevealedValue("");
                            } else {
                              setRevealError("");
                              setStepUpIntent("reveal");
                            }
                          }}
                        />
                      ) : null}
                    </div>
                  </div>
                </Field>
              </div>
              <div className="md:col-span-2">
                <Field className="w-full" label={t("pages:variables.form.description", "Description")}>
                  <div className="min-h-20 w-full whitespace-pre-wrap break-words rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4 text-sm leading-6 text-[var(--color-on-surface)]">
                    {selectedRecord.description ?? ""}
                  </div>
                </Field>
              </div>
            </div>
          )}
        </SurfaceCard>
      </div>

      <StepUpRevealDialog
        error={revealError}
        onCancel={cancelStepUp}
        onSubmit={(password) => void revealSensitiveValue(password)}
        open={stepUpIntent !== null}
        pending={revealPending}
        variableKey={selectedRecord.key}
      />
    </>
  );
}

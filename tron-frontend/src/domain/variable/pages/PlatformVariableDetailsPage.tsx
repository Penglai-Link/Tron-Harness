import {
  useEffect,
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
  Trash2,
  X,
} from "lucide-react";

import {
  deletePlatformVariable,
  fetchPlatformVariables,
  revealPlatformVariable,
  updatePlatformVariable,
  verifyStepUpPassword,
} from "@/domain/variable/api";
import {
  deletePlatformVariableRecord,
  listPlatformVariableRecords,
  revealPlatformVariableRecord,
  updatePlatformVariableRecord,
  verifyStepUpPasswordRecord,
} from "@/domain/variable/mock/data";
import type { PlatformVariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { Button } from "@/shared/ui/Button";
import {
  Field,
  PageHeader,
  SurfaceCard,
  useToast,
} from "@/shared";

import {
  PlatformVariableFormFields,
  ReadValue,
  SensitiveDataBadge,
  SensitiveRevealIconButton,
  SensitiveValueCode,
  StepUpRevealDialog,
  emptyPlatformVariableForm,
  findPlatformVariable,
  formatPlatformVariableTimestamp,
  getSensitiveDisplayValue,
  getSensitiveState,
  isSensitiveRecord,
  normalizePlatformVariableDescription,
  platformVariableFormFromRecord,
} from "./platformVariablePageHelpers";

type StepUpIntent = "edit" | "reveal";

export function PlatformVariableDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const variableKey = searchParams.get("variableKey") ?? "";
  const startsInEditMode = searchParams.get("mode") === "edit";
  const [record, setRecord] = useState<PlatformVariableRecord | null>(null);
  const [form, setForm] = useState(emptyPlatformVariableForm());
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [editing, setEditing] = useState(false);
  const [revealedValue, setRevealedValue] = useState("");
  const [stepUpIntent, setStepUpIntent] = useState<StepUpIntent | null>(null);
  const [revealPending, setRevealPending] = useState(false);
  const [revealError, setRevealError] = useState("");

  useEffect(() => {
    if (!variableKey) return;

    let active = true;

    async function loadVariable() {
      setLoading(true);
      try {
        const records = dataSource === "api"
          ? await fetchPlatformVariables()
          : listPlatformVariableRecords();
        if (active) {
          setRecord(findPlatformVariable(records, variableKey));
          setRevealedValue("");
          setStepUpIntent(null);
          setRevealError("");
        }
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : "Platform variable could not be loaded");
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadVariable();

    return () => {
      active = false;
    };
  }, [dataSource, toast, variableKey]);

  useEffect(() => {
    if (!record) return;
    setForm(platformVariableFormFromRecord(record));
  }, [record]);

  useEffect(() => {
    if (!record) return;
    if (startsInEditMode && isSensitiveRecord(record)) {
      setEditing(false);
      setStepUpIntent("edit");
      setRevealError("");
      return;
    }

    setEditing(startsInEditMode);
    if (!startsInEditMode) setStepUpIntent(null);
  }, [record, startsInEditMode]);

  if (!variableKey) {
    return <Navigate replace to="/settings/variables" />;
  }

  function detailPath(key = variableKey) {
    return `/settings/variables/detail?variableKey=${encodeURIComponent(key)}`;
  }

  function enterEditing() {
    if (!record) return;
    if (isSensitiveRecord(record)) {
      setEditing(false);
      setRevealedValue("");
      setRevealError("");
      setStepUpIntent("edit");
      navigate(`${detailPath(record.key)}&mode=edit`, { replace: true });
      return;
    }

    setEditing(true);
    setRevealedValue("");
    setStepUpIntent(null);
    setRevealError("");
    navigate(`${detailPath(record.key)}&mode=edit`, { replace: true });
  }

  function exitEditing(key = variableKey) {
    setEditing(false);
    setStepUpIntent(null);
    navigate(detailPath(key), { replace: true });
  }

  function cancelStepUp() {
    setStepUpIntent(null);
    setRevealError("");
    if (startsInEditMode) navigate(detailPath(), { replace: true });
  }

  function resetDraft() {
    if (!record) return;
    setForm(platformVariableFormFromRecord(record));
    exitEditing(record.key);
  }

  async function saveVariable() {
    if (!record) return;
    const wasSensitive = isSensitiveRecord(record);
    if (wasSensitive && !form.isSensitive && !form.value.trim()) {
      toast.error("New value is required before turning off sensitivity");
      return;
    }
    if (form.isSensitive && !form.value.trim()) {
      toast.error("Variable value is required");
      return;
    }
    if (!wasSensitive && form.isSensitive && !form.value.trim()) {
      toast.error("Sensitive value is required before turning on sensitivity");
      return;
    }

    setSubmitting(true);
    try {
      const payload = {
        description: normalizePlatformVariableDescription(form.description),
        isSensitive: form.isSensitive,
        value: form.value,
        valueMode: "REPLACE" as const,
      };
      const saved = dataSource === "api"
        ? await updatePlatformVariable(record.key, payload)
        : updatePlatformVariableRecord(record.key, payload);
      toast.success(`${saved.key} saved`);
      setRecord(saved);
      setRevealedValue("");
      setStepUpIntent(null);
      setRevealError("");
      exitEditing(saved.key);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform variable could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  async function removeVariable() {
    if (!record) return;
    setDeleting(true);
    try {
      if (dataSource === "api") {
        await deletePlatformVariable(record.key);
      } else {
        deletePlatformVariableRecord(record.key);
      }
      toast.success(`${record.key} deleted`);
      navigate("/settings/variables");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Platform variable could not be deleted");
    } finally {
      setDeleting(false);
    }
  }

  async function revealSensitiveValue(password: string) {
    if (!record) return;
    const intent = stepUpIntent ?? "reveal";
    setRevealPending(true);
    setRevealError("");
    try {
      const stepUp = dataSource === "api"
        ? await verifyStepUpPassword({
          password,
          target: {
            scope: "platform",
            variableKey: record.key,
          },
        })
        : verifyStepUpPasswordRecord({
          password,
          target: {
            scope: "platform",
            variableKey: record.key,
          },
        });
      const revealed = dataSource === "api"
        ? await revealPlatformVariable({
          variableKey: record.key,
          stepUpToken: stepUp.stepUpToken,
        })
        : revealPlatformVariableRecord(record.key, stepUp.stepUpToken);
      if (intent === "edit") {
        setForm({
          ...platformVariableFormFromRecord(record),
          value: revealed.value,
          valueMode: "REPLACE",
        });
        setEditing(true);
        setRevealedValue("");
      } else {
        setRevealedValue(revealed.value);
      }
      setStepUpIntent(null);
      setRevealError("");
    } catch (error) {
      setRevealError(error instanceof Error ? error.message : "Sensitive value could not be revealed");
    } finally {
      setRevealPending(false);
    }
  }

  if (!record && loading) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: "Loading" }]}
          title="Platform Variable"
        />
        <SurfaceCard>Loading platform variable...</SurfaceCard>
      </>
    );
  }

  if (!record) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: "Not Found" }]}
          title="Platform Variable"
        />
        <SurfaceCard>Platform variable not found.</SurfaceCard>
      </>
    );
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Variables", href: "/settings/variables" }, { label: record.key }]}
        title={record.key}
        actions={editing ? (
          <>
            <Button variant="secondary" disabled={submitting} onClick={resetDraft}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
            <Button disabled={submitting} aria-busy={submitting} onClick={() => void saveVariable()}>
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        ) : (
          <>
            <Button onClick={enterEditing}>
              <Edit3 size={16} aria-hidden="true" />
              Edit
            </Button>
            <Button variant="ghost" disabled={deleting || loading} aria-busy={deleting} onClick={() => void removeVariable()}>
              <Trash2 size={16} aria-hidden="true" />
              {deleting ? "Deleting..." : "Delete"}
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        <SurfaceCard className="space-y-6">
          {editing ? (
            <PlatformVariableFormFields form={form} keyEditable={false} onChange={setForm} />
          ) : (
            <div className="grid gap-x-8 gap-y-6 md:grid-cols-2">
              <Field label="Variable Key">
                <ReadValue>
                  <code>{record.key}</code>
                </ReadValue>
              </Field>
              <Field label="Sensitive Data">
                <SensitiveDataBadge sensitive={isSensitiveRecord(record)} />
              </Field>
              <Field label="Scope">
                <ReadValue>Platform Configuration</ReadValue>
              </Field>
              <Field label="Created">
                <ReadValue>{formatPlatformVariableTimestamp(record.createdAt)}</ReadValue>
              </Field>
              <Field label="Updated">
                <ReadValue>{formatPlatformVariableTimestamp(record.updatedAt)}</ReadValue>
              </Field>
              <Field label="Identifier">
                <ReadValue>{record.id}</ReadValue>
              </Field>
              <div className="md:col-span-2">
                <Field label="Variable Value">
                  <div className="min-h-32 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-4">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <SensitiveValueCode
                        className="min-w-0 flex-1"
                        state={getSensitiveState(isSensitiveRecord(record), revealedValue)}
                        testId={`platform-sensitive-value-${record.key}`}
                      >
                        {getSensitiveDisplayValue(record, revealedValue)}
                      </SensitiveValueCode>
                      {isSensitiveRecord(record) ? (
                        <SensitiveRevealIconButton
                          revealed={Boolean(revealedValue)}
                          variableKey={record.key}
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
                <Field label="Description">
                  <div className="min-h-20 whitespace-pre-wrap break-words rounded-lg border border-[var(--color-outline-variant)] p-4 text-sm leading-6 text-[var(--color-on-surface)]">
                    {record.description ?? ""}
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
        variableKey={record.key}
      />
    </>
  );
}

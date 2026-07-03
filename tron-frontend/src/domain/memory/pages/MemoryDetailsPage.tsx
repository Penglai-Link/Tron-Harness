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
  Archive,
  Ban,
  Check,
  Copy,
  Edit3,
  Save,
  X,
} from "lucide-react";

import {
  MemoryFormFields,
  memoryFormFromRecord,
  memoryFormToInput,
  validateMemoryForm,
  type MemoryFormState,
} from "@/domain/memory/components";
import {
  archiveMemoryRecord,
  disableMemoryRecord,
  updateMemoryRecord,
  useMemoryRecord,
  useMemoryTargets,
} from "@/domain/memory";
import type {
  MemoryRecord,
  MemorySearchTarget,
} from "@/domain/memory/model";
import {
  archiveMemoryFragment,
  disableMemoryFragment,
  updateMemoryFragment,
} from "@/domain/memory/mock";
import {
  Badge,
  Button,
  ConfirmDialog,
  Field,
  PageHeader,
  SurfaceCard,
  useConsoleShellConfig,
  useToast,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";

type ConfirmAction = "archive" | "disable" | null;

function getTypeTone(memory: MemoryRecord) {
  if (memory.memoryStatus === "DISABLED") return "error";
  if (memory.scope === "space") return "success";
  if (memory.scope === "domain") return "warning";
  if (memory.scope === "user") return "info";
  return "outline";
}

function formatReviewStatus(reviewStatus: MemoryRecord["reviewStatus"]) {
  return reviewStatus.replace("_", " ");
}

function getUseLabel(memory: Pick<MemoryRecord, "injectable">) {
  return memory.injectable ? "Auto context" : "Search only";
}

function ReadValue({ children }: { children: string }) {
  return (
    <div className="min-h-6 break-words text-sm leading-6 text-[var(--color-on-surface)]">
      {children || "None"}
    </div>
  );
}

function CodeValue({ children }: { children: string }) {
  return (
    <code className="block min-h-6 break-words rounded-md bg-[var(--color-surface-container-low)] px-2 py-1 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">
      {children || "None"}
    </code>
  );
}

function targetFromMemory(memory: MemoryRecord): MemorySearchTarget {
  return {
    code: memory.targetCode,
    description: memory.scope === "domain" ? "Work domain" : `Current ${memory.scope}`,
    id: memory.targetId,
    label: memory.targetLabel,
    resultCount: 0,
    scope: memory.scope,
  };
}

export function MemoryDetailsPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const toast = useToast();
  const dataSource = getServiceDataSource("memory");
  const {
    selectedPrimarySpaceId,
    selectedSecondarySpaceId,
  } = useWorkspaceSpaces();
  const workspaceCode = selectedSecondarySpaceId || selectedPrimarySpaceId;
  const memoryId = searchParams.get("memoryId") ?? "";
  const editing = searchParams.get("mode") === "edit";
  const [refreshKey, setRefreshKey] = useState(0);
  const memory = useMemoryRecord(workspaceCode, memoryId, refreshKey);
  const targets = useMemoryTargets(workspaceCode, refreshKey);
  const safeTargets = Array.isArray(targets) ? targets : [];
  const [form, setForm] = useState<MemoryFormState | null>(null);
  const [copied, setCopied] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction>(null);

  useConsoleShellConfig({ searchPlaceholder: "Search memories..." });

  useEffect(() => {
    if (!memory) return;
    setForm(memoryFormFromRecord(memory));
  }, [memory]);

  if (!memoryId) {
    return <Navigate replace to="/memory" />;
  }

  if (!memory) {
    return (
      <>
        <PageHeader
          breadcrumbs={[{ label: "Memory", href: "/memory" }, { label: "Not Found" }]}
          title="Memory"
        />
        <SurfaceCard>Memory not found.</SurfaceCard>
      </>
    );
  }

  const selectedMemory = memory;
  const lockedTarget = targetFromMemory(selectedMemory);
  const targetOptions = [lockedTarget, ...safeTargets.filter((target) => target.id !== lockedTarget.id)];
  const domainTargets = targetOptions.filter((target) => target.scope === "domain");

  function detailHref(nextMode?: "edit") {
    const params = new URLSearchParams({ memoryId });
    if (nextMode) params.set("mode", nextMode);
    return `/memory/details?${params.toString()}`;
  }

  async function copyMemoryCode() {
    await navigator.clipboard?.writeText(selectedMemory.memoryCode);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  }

  function cancelEdit() {
    setForm(memoryFormFromRecord(selectedMemory));
    navigate(detailHref(), { replace: true });
  }

  function refreshMemory() {
    setRefreshKey((value) => value + 1);
  }

  async function saveMemory() {
    if (!form) return;
    const validationError = validateMemoryForm(form, lockedTarget);
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setSubmitting(true);
    try {
      const payload = memoryFormToInput(form, lockedTarget);
      const saved = dataSource === "api" && workspaceCode
        ? await updateMemoryRecord(workspaceCode, selectedMemory.memoryCode, payload)
        : updateMemoryFragment(selectedMemory.id, payload);
      toast.success(`${saved.title} saved`);
      refreshMemory();
      navigate(detailHref(), { replace: true });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Memory could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  async function confirmMutation() {
    if (!confirmAction) return;
    try {
      const next = dataSource === "api" && workspaceCode
        ? confirmAction === "archive"
          ? await archiveMemoryRecord(workspaceCode, selectedMemory.memoryCode)
          : await disableMemoryRecord(workspaceCode, selectedMemory.memoryCode)
        : confirmAction === "archive"
          ? archiveMemoryFragment(selectedMemory.id)
          : disableMemoryFragment(selectedMemory.id);
      toast.success(`${next.title} ${confirmAction === "archive" ? "archived" : "disabled"}`);
      setConfirmAction(null);
      refreshMemory();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Memory status could not be updated");
    }
  }

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: "Memory", href: "/memory" }, { label: memory.title }]}
        title={memory.title}
        actions={editing ? (
          <>
            <Button variant="secondary" disabled={submitting} onClick={cancelEdit}>
              <X size={16} aria-hidden="true" />
              Cancel
            </Button>
            <Button disabled={submitting || !form?.title.trim()} aria-busy={submitting} onClick={saveMemory}>
              <Save size={16} aria-hidden="true" />
              {submitting ? "Saving..." : "Save"}
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" onClick={() => navigate(detailHref("edit"))}>
              <Edit3 size={16} aria-hidden="true" />
              Edit
            </Button>
            <Button variant="secondary" onClick={() => void copyMemoryCode()}>
              {copied ? <Check size={16} aria-hidden="true" /> : <Copy size={16} aria-hidden="true" />}
              {copied ? "Copied" : "Copy code"}
            </Button>
            <Button variant="ghost" onClick={() => setConfirmAction("archive")}>
              <Archive size={16} aria-hidden="true" />
              Archive
            </Button>
            <Button variant="ghost" onClick={() => setConfirmAction("disable")}>
              <Ban size={16} aria-hidden="true" />
              Disable
            </Button>
          </>
        )}
      />

      <div className="max-w-5xl space-y-5">
        {editing && form ? (
          <>
            <SurfaceCard>
              <MemoryFormFields
                domainTargets={domainTargets}
                lockedTarget={lockedTarget}
                mode="edit"
                state={form}
                targetOptions={targetOptions}
                onChange={setForm}
              />
            </SurfaceCard>
            <SurfaceCard className="space-y-5">
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Locked identity</h2>
              <div className="grid gap-x-8 gap-y-5 md:grid-cols-2">
                <Field label="Memory code">
                  <CodeValue>{memory.memoryCode}</CodeValue>
                </Field>
                <Field label="Target code">
                  <CodeValue>{memory.targetCode}</CodeValue>
                </Field>
                <Field label="Source code">
                  <CodeValue>{memory.sourceCode}</CodeValue>
                </Field>
                <Field label="Score">
                  <ReadValue>{memory.score.toFixed(2)}</ReadValue>
                </Field>
                <Field label="Updated">
                  <ReadValue>{memory.updatedAt}</ReadValue>
                </Field>
              </div>
            </SurfaceCard>
          </>
        ) : (
          <>
            <SurfaceCard className="space-y-5">
              <div className="flex flex-wrap items-center gap-2">
                <Badge className="font-bold" tone={getTypeTone(memory)}>{memory.memoryType}</Badge>
                <Badge tone={memory.memoryStatus === "ACTIVE" ? "success" : "info"}>{memory.memoryStatus}</Badge>
                <Badge tone={memory.reviewStatus === "APPROVED" ? "success" : "info"}>{formatReviewStatus(memory.reviewStatus)}</Badge>
                <Badge tone={memory.injectable ? "success" : "info"}>{getUseLabel(memory)}</Badge>
                <Badge tone="info">{memory.targetLabel}</Badge>
              </div>
              <div>
                <h2 className="text-xl font-semibold leading-7 tracking-normal text-[var(--color-on-surface)]">
                  {memory.title}
                </h2>
                <p className="mt-3 whitespace-pre-wrap text-sm leading-7 text-[var(--color-on-surface)]">
                  {memory.text}
                </p>
              </div>
              <CodeValue>{memory.memoryCode}</CodeValue>
            </SurfaceCard>

            <SurfaceCard className="space-y-5">
              <div>
                <h2 className="text-base font-semibold text-[var(--color-on-surface)]">Metadata</h2>
                <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                  Audit and identity fields for this memory.
                </p>
              </div>
              <div className="grid gap-x-8 gap-y-5 md:grid-cols-2">
                <Field label="Scope">
                  <ReadValue>{memory.scope}</ReadValue>
                </Field>
                <Field label="Target">
                  <ReadValue>{memory.targetLabel}</ReadValue>
                </Field>
                <Field label="Target code">
                  <CodeValue>{memory.targetCode}</CodeValue>
                </Field>
                <Field label="Semantic key">
                  <CodeValue>{memory.semanticKey}</CodeValue>
                </Field>
                <Field label="Source label">
                  <ReadValue>{memory.sourceLabel}</ReadValue>
                </Field>
                <Field label="Source code">
                  <CodeValue>{memory.sourceCode}</CodeValue>
                </Field>
                <Field label="Score">
                  <ReadValue>{memory.score.toFixed(2)}</ReadValue>
                </Field>
                <Field label="Updated">
                  <ReadValue>{memory.updatedAt}</ReadValue>
                </Field>
                <div className="md:col-span-2">
                  <Field label="Tags">
                    <ReadValue>{memory.tags.join(", ")}</ReadValue>
                  </Field>
                </div>
                {memory.metadata.map((item) => (
                  <Field key={`${item.label}:${item.value}`} label={item.label}>
                    <ReadValue>{item.value}</ReadValue>
                  </Field>
                ))}
              </div>
            </SurfaceCard>
          </>
        )}
      </div>

      <ConfirmDialog
        open={confirmAction !== null}
        title={confirmAction === "disable" ? "Disable memory?" : "Archive memory?"}
        description={confirmAction === "disable"
          ? "This mock memory will be disabled and removed from automatic context use."
          : "This mock memory will be archived and kept for later review."}
        confirmText={confirmAction === "disable" ? "Disable" : "Archive"}
        onCancel={() => setConfirmAction(null)}
        onConfirm={confirmMutation}
      />
    </>
  );
}

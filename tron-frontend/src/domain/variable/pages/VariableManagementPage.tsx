import {
  useEffect,
  useState,
} from "react";
import {
  ChevronRight,
  Plus,
  RefreshCcw,
  Save,
  Search,
  Trash2,
  Undo2,
  X,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  DataTable,
  Field,
  PageHeader,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  useTablePagination,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";

import {
  createVariable,
  deleteVariable,
  updateVariable,
} from "@/domain/variable/api";
import { getVariableSourceTone } from "@/domain/variable/constants";
import { useVariableCatalog } from "@/domain/variable/hooks";
import {
  createVariableRecord,
  deleteVariableRecord,
  updateVariableRecord,
} from "@/domain/variable/mock/data";
import type { VariableRecord } from "@/domain/variable/model";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

type PanelMode = "create" | "override" | "selected";
type RouteVariant = "details" | "standard";

interface VariableFormState {
  description: string;
  key: string;
  value: string;
}

function emptyForm(): VariableFormState {
  return {
    description: "",
    key: "",
    value: "",
  };
}

function SearchField({
  onChange,
  placeholder,
  value,
}: {
  onChange: (value: string) => void;
  placeholder: string;
  value: string;
}) {
  return (
    <div className="relative">
      <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]" size={15} aria-hidden="true" />
      <TextInput
        aria-label="Search variables"
        className={cn("h-9 w-full pl-9", value ? "pr-9" : undefined)}
        placeholder={placeholder}
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      {value ? (
        <Button
          aria-label="Clear variable search"
          className="absolute right-1 top-1/2 size-7 -translate-y-1/2"
          size="icon"
          type="button"
          variant="ghost"
          onClick={() => onChange("")}
        >
          <X size={14} aria-hidden="true" />
        </Button>
      ) : null}
    </div>
  );
}

function DetailField({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">{label}</p>
      <p className="mt-1 break-words text-sm font-medium text-[var(--color-on-surface)]">{value}</p>
    </div>
  );
}

function formatTimestamp(value: string) {
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function normalizeDescription(value: string) {
  return value.trim() ? value.trim() : null;
}

function matchesVariableSearch(record: VariableRecord, search: string) {
  if (!search.trim()) return true;
  const value = [
    record.key,
    record.value,
    record.description ?? "",
    record.sourceLabel,
    record.sourceWorkspaceCode,
    record.shadowedWorkspaceName ?? "",
    record.shadowedWorkspaceCode ?? "",
    record.shadowedLabel ?? "",
  ].join(" ").toLowerCase();
  return value.includes(search.trim().toLowerCase());
}

function formFromVariable(record: VariableRecord): VariableFormState {
  return {
    description: record.description ?? "",
    key: record.key,
    value: record.value,
  };
}

function getInheritedWorkspace(record: VariableRecord) {
  if (record.sourceKind === "Local") {
    return null;
  }

  if (record.sourceKind === "Override") {
    if (record.shadowedLabel || record.shadowedWorkspaceName || record.shadowedWorkspaceCode) {
      return {
        code: record.shadowedWorkspaceCode,
        label: record.shadowedLabel?.trim() || record.shadowedWorkspaceName?.trim() || record.shadowedWorkspaceCode || "Workspace",
      };
    }

    return null;
  }

  return {
    code: record.sourceWorkspaceCode,
    label: record.sourceLabel?.trim() || record.sourceWorkspaceName?.trim() || record.sourceWorkspaceCode || "Workspace",
  };
}

export function VariableManagementPage({ routeVariant }: { routeVariant: RouteVariant }) {
  const toast = useToast();
  const dataSource = getServiceDataSource("variable");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const activeWorkspaceCode = activeWorkspace?.workspaceCode ?? "";
  const activeWorkspaceLabel = selectedSecondarySpace?.label ?? selectedPrimarySpace?.name ?? "No workspace selected";
  const activeWorkspaceDescription = activeWorkspace?.description ?? "Use the shell workspace switcher to choose a variable scope.";
  const { loading, refresh, variables } = useVariableCatalog(activeWorkspaceCode);
  const [search, setSearch] = useState("");
  const [selectedKey, setSelectedKey] = useState("");
  const [panelMode, setPanelMode] = useState<PanelMode>("selected");
  const [form, setForm] = useState<VariableFormState>(emptyForm());
  const [submitting, setSubmitting] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (!variables.length) {
      setSelectedKey("");
      if (panelMode === "selected") setForm(emptyForm());
      return;
    }
    if (!variables.some((record) => record.key === selectedKey)) {
      setSelectedKey(variables[0].key);
    }
  }, [panelMode, selectedKey, variables]);

  const selectedVariable = variables.find((record) => record.key === selectedKey);

  useEffect(() => {
    if (panelMode !== "selected") return;
    if (!selectedVariable) {
      setForm(emptyForm());
      return;
    }
    setForm(formFromVariable(selectedVariable));
  }, [panelMode, selectedVariable]);

  const filteredVariables = variables.filter((record) => matchesVariableSearch(record, search));
  const pagination = useTablePagination(filteredVariables);

  function resetToSelected() {
    setPanelMode("selected");
    if (selectedVariable) {
      setForm(formFromVariable(selectedVariable));
      return;
    }
    setForm(emptyForm());
  }

  function beginCreate() {
    setPanelMode("create");
    setForm(emptyForm());
  }

  function beginOverride() {
    if (!selectedVariable) return;
    setPanelMode("override");
    setForm(formFromVariable(selectedVariable));
  }

  async function saveVariable() {
    if (!activeWorkspaceCode) return;
    const trimmedKey = form.key.trim();
    if (!trimmedKey) {
      toast.error("Variable key is required");
      return;
    }

    setSubmitting(true);
    try {
      let saved: VariableRecord;
      if (panelMode === "create" || panelMode === "override") {
        saved = dataSource === "api"
          ? await createVariable(activeWorkspaceCode, {
            key: trimmedKey,
            value: form.value,
            description: normalizeDescription(form.description),
          })
          : createVariableRecord(activeWorkspaceCode, {
            key: trimmedKey,
            value: form.value,
            description: normalizeDescription(form.description),
          });
        toast.success(panelMode === "override" ? `${saved.key} overridden in ${activeWorkspaceCode}` : `${saved.key} created`);
      } else if (selectedVariable) {
        saved = dataSource === "api"
          ? await updateVariable(activeWorkspaceCode, selectedVariable.key, {
            value: form.value,
            description: normalizeDescription(form.description),
          })
          : updateVariableRecord(activeWorkspaceCode, selectedVariable.key, {
            value: form.value,
            description: normalizeDescription(form.description),
          });
        toast.success(`${saved.key} saved`);
      } else {
        setSubmitting(false);
        return;
      }

      setSelectedKey(saved.key);
      setPanelMode("selected");
      refresh();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Variable could not be saved";
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  async function removeVariable() {
    if (!selectedVariable || !activeWorkspaceCode) return;
    setDeleting(true);
    try {
      if (dataSource === "api") {
        await deleteVariable(activeWorkspaceCode, selectedVariable.key);
      } else {
        deleteVariableRecord(activeWorkspaceCode, selectedVariable.key);
      }
      toast.success(selectedVariable.sourceKind === "Override" ? `${selectedVariable.key} reverted to inherited value` : `${selectedVariable.key} deleted`);
      setPanelMode("selected");
      refresh();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Variable could not be removed";
      toast.error(message);
    } finally {
      setDeleting(false);
    }
  }

  const inspectorTitle = panelMode === "create"
    ? "Create Variable"
    : panelMode === "override"
      ? `Override ${selectedVariable?.key ?? "Variable"}`
      : selectedVariable?.key ?? "Select a Variable";

  return (
    <>
      <PageHeader
        title="Variable Management"
        actions={(
          <>
            <Button variant="secondary" disabled={loading} aria-busy={loading} onClick={refresh}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? "Refreshing..." : "Refresh"}
            </Button>
            <Button variant="secondary" onClick={beginCreate}>
              <Plus size={16} aria-hidden="true" />
              New
            </Button>
          </>
        )}
      />

      <div className="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1fr)_380px]">
        <SurfaceCard className="p-0">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] p-4">
            <div className="min-w-[16rem] flex-1">
              <SearchField placeholder="Filter effective variables..." value={search} onChange={setSearch} />
            </div>
            {loading ? <Badge tone="accent">Loading</Badge> : null}
          </div>
          <DataTable<VariableRecord>
            rows={pagination.visibleItems}
            rowKey={(row) => row.key}
            pagination={<TablePagination itemLabel="variables" {...pagination.paginationProps} />}
            emptyState={activeWorkspaceCode ? "No variables match the current filters." : "Select a workspace in the shell to load variables."}
            columns={[
              {
                id: "key",
                header: "Key",
                render: (row) => (
                  <button
                    className={cn(
                      "rounded-md px-1 py-0.5 font-mono text-[13px] font-medium text-left",
                      row.key === selectedKey ? "text-[var(--color-primary)]" : "text-[var(--color-on-surface)]",
                    )}
                    type="button"
                    onClick={() => {
                      setSelectedKey(row.key);
                      setPanelMode("selected");
                    }}
                  >
                    {row.key}
                  </button>
                ),
              },
              {
                id: "value",
                header: "Value",
                render: (row) => <span className="block max-w-[20rem] truncate font-mono text-[13px]">{row.value}</span>,
              },
              {
                id: "sourceKind",
                header: "Source",
                render: (row) => (
                  <Badge tone={getVariableSourceTone(row.sourceKind)}>{row.sourceKind}</Badge>
                ),
              },
              {
                id: "inheritedFrom",
                header: "Inherited From",
                render: (row) => {
                  const inheritedWorkspace = getInheritedWorkspace(row);
                  if (!inheritedWorkspace) {
                    return (
                      <span className="text-[13px] text-[var(--color-on-surface-variant)]">-</span>
                    );
                  }

                  return (
                    <div className="min-w-0">
                      <p className="truncate font-medium text-[var(--color-on-surface)]">
                        {inheritedWorkspace.label}
                      </p>
                      {inheritedWorkspace.code ? (
                        <p className="mt-1 truncate text-[13px] text-[var(--color-on-surface-variant)]">
                          {inheritedWorkspace.code}
                        </p>
                      ) : null}
                    </div>
                  );
                },
              },
              {
                id: "updatedAt",
                header: "Updated",
                render: (row) => formatTimestamp(row.updatedAt),
              },
            ]}
          />
        </SurfaceCard>

        <SurfaceCard className="space-y-5">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">Variable Studio</p>
              <h2 className="mt-2 text-2xl font-semibold tracking-tight text-[var(--color-on-surface)]">{inspectorTitle}</h2>
              <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
                {panelMode === "create"
                  ? `Create a local variable directly under ${activeWorkspaceLabel}.`
                  : panelMode === "override"
                    ? `Create a local copy in ${activeWorkspaceLabel} without mutating the ancestor workspace.`
                    : activeWorkspaceDescription}
              </p>
            </div>
            {selectedVariable && panelMode === "selected" ? <Badge tone={getVariableSourceTone(selectedVariable.sourceKind)}>{selectedVariable.sourceKind}</Badge> : null}
          </div>

          {!activeWorkspaceCode ? (
            <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
              Select a workspace from the shell switcher to load its effective variable catalog.
            </div>
          ) : panelMode === "create" || panelMode === "override" ? (
            <>
              <div className="grid gap-4">
                <Field label="Workspace">
                  <div className="min-h-10 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm text-[var(--color-on-surface)]">
                    {activeWorkspaceLabel}
                  </div>
                </Field>
                <Field label="Variable Key">
                  <TextInput
                    aria-label="Variable key"
                    disabled={panelMode === "override"}
                    value={form.key}
                    onChange={(event) => setForm((current) => ({ ...current, key: event.target.value }))}
                  />
                </Field>
                <Field
                  label="Variable Value"
                  hint={panelMode === "override" ? "Start from the inherited value, then save a local override." : "Value is stored exactly as entered; no data typing is applied."}
                >
                  <TextArea
                    aria-label="Variable value"
                    className="min-h-32 font-mono"
                    value={form.value}
                    onChange={(event) => setForm((current) => ({ ...current, value: event.target.value }))}
                  />
                </Field>
                <Field label="Description" hint="Optional operator-facing note about how this variable is used.">
                  <TextArea
                    aria-label="Variable description"
                    className="min-h-24"
                    value={form.description}
                    onChange={(event) => setForm((current) => ({ ...current, description: event.target.value }))}
                  />
                </Field>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button disabled={submitting} aria-busy={submitting} onClick={() => void saveVariable()}>
                  <Save size={16} aria-hidden="true" />
                  {submitting ? "Saving..." : "Save"}
                </Button>
                <Button variant="secondary" onClick={resetToSelected}>
                  <X size={16} aria-hidden="true" />
                  Cancel
                </Button>
              </div>
            </>
          ) : !selectedVariable ? (
            <div className="rounded-2xl border border-dashed border-[var(--color-outline-variant)] p-6 text-sm text-[var(--color-on-surface-variant)]">
              Choose a variable from the table or create a new one in the current workspace.
            </div>
          ) : selectedVariable.sourceKind === "Inherited" ? (
            <>
              <div className="grid gap-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                <DetailField label="Current Workspace" value={activeWorkspaceLabel} />
                <DetailField label="Inherited From" value={`${selectedVariable.sourceLabel} (${selectedVariable.sourceWorkspaceCode})`} />
                <DetailField label="Updated" value={formatTimestamp(selectedVariable.updatedAt)} />
                <DetailField label="Value" value={selectedVariable.value} />
                <DetailField label="Description" value={selectedVariable.description ?? "No description"} />
              </div>
              <div className="rounded-2xl bg-[var(--color-surface-container-low)] p-4 text-sm text-[var(--color-on-surface-variant)]">
                This value is inherited from an ancestor workspace. Creating an override keeps the parent value intact and writes a local copy into {activeWorkspaceLabel}.
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button onClick={beginOverride}>
                  <ChevronRight size={16} aria-hidden="true" />
                  Override
                </Button>
              </div>
            </>
          ) : (
            <>
              <div className="grid gap-4">
                <Field label="Workspace">
                  <div className="min-h-10 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] px-3 py-2 text-sm text-[var(--color-on-surface)]">
                    {activeWorkspaceLabel}
                  </div>
                </Field>
                <Field label="Variable Key" hint="Renaming is intentionally disabled so overrides remain stable by key.">
                  <TextInput aria-label="Variable key" disabled value={form.key} />
                </Field>
                <Field
                  label="Variable Value"
                  hint={selectedVariable.sourceKind === "Override" && selectedVariable.shadowedLabel
                    ? `This local value overrides ${selectedVariable.shadowedLabel}.`
                    : "Value is scoped to the current workspace only."}
                >
                  <TextArea
                    aria-label="Variable value"
                    className="min-h-32 font-mono"
                    value={form.value}
                    onChange={(event) => setForm((current) => ({ ...current, value: event.target.value }))}
                  />
                </Field>
                <Field label="Description" hint="Optional operator-facing note about how this variable is used.">
                  <TextArea
                    aria-label="Variable description"
                    className="min-h-24"
                    value={form.description}
                    onChange={(event) => setForm((current) => ({ ...current, description: event.target.value }))}
                  />
                </Field>
              </div>
              <div className="grid gap-4 rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-4">
                <DetailField label="Source Workspace" value={`${selectedVariable.sourceLabel} (${selectedVariable.sourceWorkspaceCode})`} />
                <DetailField label="Updated" value={formatTimestamp(selectedVariable.updatedAt)} />
                {selectedVariable.shadowedLabel ? <DetailField label="Shadowed Ancestor" value={`${selectedVariable.shadowedLabel} (${selectedVariable.shadowedWorkspaceCode})`} /> : null}
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button disabled={submitting} aria-busy={submitting} onClick={() => void saveVariable()}>
                  <Save size={16} aria-hidden="true" />
                  {submitting ? "Saving..." : "Save"}
                </Button>
                <Button variant="secondary" disabled={deleting} aria-busy={deleting} onClick={() => void removeVariable()}>
                  {selectedVariable.sourceKind === "Override" ? <Undo2 size={16} aria-hidden="true" /> : <Trash2 size={16} aria-hidden="true" />}
                  {deleting ? "Working..." : selectedVariable.sourceKind === "Override" ? "Revert" : "Delete"}
                </Button>
                <Button variant="secondary" onClick={beginCreate}>
                  <Plus size={16} aria-hidden="true" />
                  New
                </Button>
              </div>
            </>
          )}
        </SurfaceCard>
      </div>
    </>
  );
}

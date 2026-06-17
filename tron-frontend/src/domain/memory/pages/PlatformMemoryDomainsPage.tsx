import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  Boxes,
  MoreHorizontal,
  Plus,
  RefreshCcw,
  Save,
  X,
} from "lucide-react";

import {
  createMemoryDomain,
  disableMemoryDomain,
  enableMemoryDomain,
  fetchMemoryDomains,
  type MemoryWorkDomainRecord,
  updateMemoryDomain,
} from "@/domain/memory/api";
import {
  createMemoryWorkDomainRecord,
  disableMemoryWorkDomainRecord,
  enableMemoryWorkDomainRecord,
  listMemoryWorkDomainsMock,
  updateMemoryWorkDomainRecord,
} from "@/domain/memory/mock";
import {
  Button,
  ConfirmDialog,
  DataTable,
  DetailField,
  Field,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  useDismissibleLayer,
  useTablePagination,
  useToast,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

interface RowMenuState {
  domainCode: string;
  left: number;
  top: number;
  triggerElement: HTMLButtonElement;
}

interface MemoryDomainFormState {
  description: string;
  domainCode: string;
  name: string;
  status: MemoryWorkDomainRecord["status"];
}

interface EditorState {
  mode: "create" | "edit" | "preview";
  record: MemoryWorkDomainRecord | null;
}

type DomainStatusFilter = "active" | "all" | "disabled";
type DomainSortMode = "updated-desc" | "name-asc" | "code-asc";

const domainStatusFilterOptions = [
  { label: "Active", value: "active" },
  { label: "All statuses", value: "all" },
  { label: "Disabled", value: "disabled" },
] as const satisfies ReadonlyArray<{ label: string; value: DomainStatusFilter }>;

const domainSortOptions = [
  { label: "Updated first", value: "updated-desc" },
  { label: "Name A-Z", value: "name-asc" },
  { label: "Code A-Z", value: "code-asc" },
] as const satisfies ReadonlyArray<{ label: string; value: DomainSortMode }>;

function emptyDomainForm(): MemoryDomainFormState {
  return {
    description: "",
    domainCode: "",
    name: "",
    status: "ACTIVE",
  };
}

function domainFormFromRecord(record: MemoryWorkDomainRecord): MemoryDomainFormState {
  return {
    description: record.description ?? "",
    domainCode: record.domainCode,
    name: record.name,
    status: record.status,
  };
}

function normalizeTextValue(value: string) {
  return value.trim();
}

function validateDomainForm(form: MemoryDomainFormState, mode: EditorState["mode"]) {
  if (!normalizeTextValue(form.domainCode)) {
    return "Domain code is required";
  }
  if (!normalizeTextValue(form.name)) {
    return "Domain name is required";
  }
  if (mode === "create" && /\s/.test(form.domainCode.trim())) {
    return "Domain code cannot contain spaces";
  }
  return "";
}

function filterMatchesRecord(record: MemoryWorkDomainRecord, query: string, statusFilter: DomainStatusFilter) {
  if (statusFilter === "active" && record.status !== "ACTIVE") return false;
  if (statusFilter === "disabled" && record.status !== "DISABLED") return false;

  const normalizedQuery = normalizeTextValue(query).toLowerCase();
  if (!normalizedQuery) return true;

  return [
    record.domainCode,
    record.name,
    record.description ?? "",
    record.status,
  ].some((value) => value.toLowerCase().includes(normalizedQuery));
}

function compareDomainRecords(left: MemoryWorkDomainRecord, right: MemoryWorkDomainRecord, sortMode: DomainSortMode) {
  if (sortMode === "name-asc") {
    return left.name.localeCompare(right.name, undefined, { sensitivity: "base" });
  }
  if (sortMode === "code-asc") {
    return left.domainCode.localeCompare(right.domainCode, undefined, { sensitivity: "base" });
  }

  const leftTime = Date.parse(left.updatedAt);
  const rightTime = Date.parse(right.updatedAt);
  if (!Number.isNaN(leftTime) && !Number.isNaN(rightTime) && leftTime !== rightTime) {
    return rightTime - leftTime;
  }
  return right.updatedAt.localeCompare(left.updatedAt);
}

function formatUpdatedAt(value: string) {
  const parsed = new Date(value);
  if (!Number.isNaN(parsed.getTime())) {
    return new Intl.DateTimeFormat("en-US", {
      day: "2-digit",
      month: "short",
      year: "numeric",
    }).format(parsed);
  }

  if (value.includes("T")) return value.split("T")[0];
  if (value.includes(" ")) return value.split(" ")[0];
  return value;
}

function DomainStatusBadge({ status }: { status: MemoryWorkDomainRecord["status"] }) {
  const className = status === "ACTIVE"
    ? "bg-[rgba(34,197,94,0.14)] text-[rgb(21,128,61)]"
    : "bg-[rgba(245,158,11,0.16)] text-[rgb(180,83,9)]";

  return (
    <span className={`inline-flex min-h-6 items-center rounded-full px-2.5 text-[12px] font-semibold ${className}`}>
      {status}
    </span>
  );
}

function DomainEditorDialog({
  form,
  mode,
  record,
  onChange,
  onClose,
  onSubmit,
  onSwitchMode,
  submitting,
}: {
  form: MemoryDomainFormState;
  mode: EditorState["mode"];
  record: MemoryWorkDomainRecord | null;
  onChange: (nextForm: MemoryDomainFormState) => void;
  onClose: () => void;
  onSubmit: () => void;
  onSwitchMode: (nextMode: EditorState["mode"]) => void;
  submitting: boolean;
}) {
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useDismissibleLayer({
    dismissOnViewportChange: false,
    onDismiss: onClose,
    open: true,
    popupRef: dialogRef,
  });

  const title = mode === "create"
    ? "New Memory Domain"
    : mode === "preview"
      ? "Memory Domain"
      : "Edit Memory Domain";
  const readOnly = mode === "preview";

  return createPortal(
    <div className="fixed inset-0 z-[90] bg-[rgba(15,23,42,0.42)] px-4 py-8">
      <div className="flex min-h-full items-center justify-center">
        <div
          ref={dialogRef}
          aria-modal="true"
          className="w-full max-w-2xl rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] shadow-[0_28px_70px_rgba(15,23,42,0.24)]"
          role="dialog"
        >
          <div className="flex items-start justify-between gap-4 border-b border-[var(--color-outline-variant)] px-6 py-5">
            <div className="min-w-0">
              <h2 className="text-lg font-semibold text-[var(--color-on-surface)]">{title}</h2>
              <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">
                {readOnly
                  ? "Review the selected domain before switching to edit mode."
                  : "Domain-scoped memory targets use this catalog."}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {record ? (
                <div className="inline-flex rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-1">
                  <button
                    className={`rounded-md px-3 py-1.5 text-sm ${mode === "preview" ? "bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)] shadow-sm" : "text-[var(--color-on-surface-variant)]"}`}
                    type="button"
                    onClick={() => onSwitchMode("preview")}
                  >
                    Preview
                  </button>
                  <button
                    className={`rounded-md px-3 py-1.5 text-sm ${mode === "edit" ? "bg-[var(--color-elevated-surface)] text-[var(--color-on-surface)] shadow-sm" : "text-[var(--color-on-surface-variant)]"}`}
                    type="button"
                    onClick={() => onSwitchMode("edit")}
                  >
                    Edit
                  </button>
                </div>
              ) : null}
              <Button size="icon" variant="ghost" aria-label="Close memory domain dialog" onClick={onClose}>
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
          </div>

          <div className="grid gap-5 px-6 py-6">
            {readOnly ? (
              <>
                <div className="grid gap-5 md:grid-cols-2">
                  <DetailField
                    label="Domain Code"
                    value={(
                      <code className="inline-flex rounded-md bg-[var(--color-surface-container-low)] px-2 py-1 font-mono text-[13px] text-[var(--color-on-surface-variant)]">
                        {form.domainCode}
                      </code>
                    )}
                  />
                  <DetailField
                    label="Status"
                    value={<DomainStatusBadge status={form.status} />}
                    valueClassName="mt-2"
                  />
                </div>

                <DetailField
                  label="Name"
                  value={(
                    <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3">
                      {form.name || "None"}
                    </div>
                  )}
                  valueClassName="mt-2 font-normal"
                />

                <DetailField
                  label="Description"
                  value={(
                    <div className="min-h-28 whitespace-pre-wrap rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 leading-6 text-[var(--color-on-surface)]">
                      {form.description || "No description"}
                    </div>
                  )}
                  valueClassName="mt-2 font-normal"
                />
              </>
            ) : (
              <>
                <div className="grid gap-5 md:grid-cols-2">
                  <Field label="Domain Code">
                    <TextInput
                      readOnly={mode !== "create"}
                      value={form.domainCode}
                      placeholder="frontend"
                      onChange={(event) => onChange({ ...form, domainCode: event.target.value })}
                    />
                  </Field>
                  <Field label="Status">
                    <SelectInput
                      value={form.status}
                      onChange={(event) => onChange({ ...form, status: event.target.value as MemoryWorkDomainRecord["status"] })}
                    >
                      <option value="ACTIVE">ACTIVE</option>
                      <option value="DISABLED">DISABLED</option>
                    </SelectInput>
                  </Field>
                </div>

                <Field label="Name">
                  <TextInput
                    value={form.name}
                    placeholder="Frontend Engineering"
                    onChange={(event) => onChange({ ...form, name: event.target.value })}
                  />
                </Field>

                <Field label="Description">
                  <TextArea
                    value={form.description}
                    placeholder="Explain what this domain is used for."
                    onChange={(event) => onChange({ ...form, description: event.target.value })}
                  />
                </Field>
              </>
            )}
          </div>

          <div className="flex justify-end gap-2 border-t border-[var(--color-outline-variant)] px-6 py-4">
            <Button variant="secondary" onClick={onClose}>Cancel</Button>
            {readOnly ? (
              <Button onClick={() => onSwitchMode("edit")}>
                Edit
              </Button>
            ) : (
              <Button disabled={submitting} aria-busy={submitting} onClick={onSubmit}>
                <Save size={16} aria-hidden="true" />
                {submitting ? "Saving..." : "Save"}
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}

export function PlatformMemoryDomainsPage() {
  const toast = useToast();
  const dataSource = getServiceDataSource("memory");
  const [records, setRecords] = useState<MemoryWorkDomainRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [rowBusyCode, setRowBusyCode] = useState("");
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<DomainStatusFilter>("active");
  const [sortMode, setSortMode] = useState<DomainSortMode>("updated-desc");
  const [refreshKey, setRefreshKey] = useState(0);
  const [openRowMenu, setOpenRowMenu] = useState<RowMenuState | null>(null);
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [editorForm, setEditorForm] = useState<MemoryDomainFormState>(emptyDomainForm());
  const [disableTarget, setDisableTarget] = useState<MemoryWorkDomainRecord | null>(null);
  const rowMenuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedQuery(query.trim());
    }, 260);

    return () => window.clearTimeout(timer);
  }, [query]);

  useDismissibleLayer({
    onDismiss: () => setOpenRowMenu(null),
    open: openRowMenu !== null,
    popupRef: rowMenuRef,
    triggerElement: openRowMenu?.triggerElement,
  });

  useEffect(() => {
    let active = true;

    async function loadDomains() {
      setLoading(true);
      try {
        const includeDisabled = statusFilter !== "active";
        const nextRecords = dataSource === "api"
          ? await fetchMemoryDomains({ includeDisabled, search: debouncedQuery })
          : listMemoryWorkDomainsMock({ includeDisabled, search: debouncedQuery });
        if (active) setRecords(nextRecords);
      } catch (error) {
        if (active) toast.error(error instanceof Error ? error.message : "Memory domains could not be loaded");
      } finally {
        if (active) setLoading(false);
      }
    }

    void loadDomains();

    return () => {
      active = false;
    };
  }, [dataSource, debouncedQuery, refreshKey, statusFilter, toast]);

  const visibleRecords = useMemo(() => (
    [...records]
      .filter((record) => filterMatchesRecord(record, debouncedQuery, statusFilter))
      .sort((left, right) => compareDomainRecords(left, right, sortMode))
  ), [debouncedQuery, records, sortMode, statusFilter]);

  const pagination = useTablePagination(visibleRecords);
  const openMenuRow = openRowMenu
    ? records.find((record) => record.domainCode === openRowMenu.domainCode) ?? null
    : null;

  function refreshDomains() {
    setRefreshKey((value) => value + 1);
  }

  function openCreateDialog() {
    setEditor({ mode: "create", record: null });
    setEditorForm(emptyDomainForm());
  }

  function openEditDialog(record: MemoryWorkDomainRecord) {
    setEditor({ mode: "edit", record });
    setEditorForm(domainFormFromRecord(record));
  }

  function openPreviewDialog(record: MemoryWorkDomainRecord) {
    setEditor({ mode: "preview", record });
    setEditorForm(domainFormFromRecord(record));
  }

  function closeEditor() {
    if (submitting) return;
    setEditor(null);
    setEditorForm(emptyDomainForm());
  }

  function switchEditorMode(nextMode: EditorState["mode"]) {
    setEditor((current) => {
      if (!current) return current;
      if (!current.record && nextMode !== "create") return current;
      return {
        ...current,
        mode: nextMode,
      };
    });
  }

  function toggleRowMenu(record: MemoryWorkDomainRecord, trigger: HTMLButtonElement) {
    if (rowBusyCode === record.domainCode) return;
    if (openRowMenu?.domainCode === record.domainCode) {
      setOpenRowMenu(null);
      return;
    }

    const menuWidth = 168;
    const estimatedMenuHeight = 108;
    const rect = trigger.getBoundingClientRect();
    const left = Math.min(
      Math.max(12, rect.right - menuWidth),
      window.innerWidth - menuWidth - 12,
    );
    const top = Math.min(
      Math.max(12, rect.bottom + 8),
      window.innerHeight - estimatedMenuHeight - 12,
    );

    setOpenRowMenu({
      domainCode: record.domainCode,
      left,
      top,
      triggerElement: trigger,
    });
  }

  async function saveDomain() {
    if (!editor) return;

    const validationError = validateDomainForm(editorForm, editor.mode);
    if (validationError) {
      toast.error(validationError);
      return;
    }

    setSubmitting(true);
    try {
      const description = normalizeTextValue(editorForm.description);
      if (editor.mode === "create") {
        const created = dataSource === "api"
          ? await createMemoryDomain({
            description: description || null,
            domainCode: normalizeTextValue(editorForm.domainCode),
            name: normalizeTextValue(editorForm.name),
          })
          : createMemoryWorkDomainRecord({
            description: description || null,
            domainCode: normalizeTextValue(editorForm.domainCode),
            name: normalizeTextValue(editorForm.name),
          });

        if (editorForm.status === "DISABLED") {
          if (dataSource === "api") {
            await disableMemoryDomain(created.domainCode);
          } else {
            disableMemoryWorkDomainRecord(created.domainCode);
          }
        }

        toast.success(`${created.name} created`);
      } else if (editor.record) {
        const saved = dataSource === "api"
          ? await updateMemoryDomain(editor.record.domainCode, {
            description: description || null,
            name: normalizeTextValue(editorForm.name),
            status: editorForm.status,
          })
          : updateMemoryWorkDomainRecord(editor.record.domainCode, {
            description: description || null,
            name: normalizeTextValue(editorForm.name),
            status: editorForm.status,
          });
        toast.success(`${saved.name} saved`);
      }

      closeEditor();
      refreshDomains();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Memory domain could not be saved");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleEnable(record: MemoryWorkDomainRecord) {
    if (rowBusyCode === record.domainCode) return;
    setRowBusyCode(record.domainCode);
    setOpenRowMenu(null);

    try {
      const updated = dataSource === "api"
        ? await enableMemoryDomain(record.domainCode)
        : enableMemoryWorkDomainRecord(record.domainCode);
      toast.success(`${updated.name} enabled`);
      refreshDomains();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Memory domain could not be enabled");
    } finally {
      setRowBusyCode("");
    }
  }

  async function confirmDisable() {
    if (!disableTarget) return;
    setRowBusyCode(disableTarget.domainCode);

    try {
      const updated = dataSource === "api"
        ? await disableMemoryDomain(disableTarget.domainCode)
        : disableMemoryWorkDomainRecord(disableTarget.domainCode);
      toast.success(`${updated.name} disabled`);
      refreshDomains();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Memory domain could not be disabled");
    } finally {
      setDisableTarget(null);
      setRowBusyCode("");
    }
  }

  return (
    <>
      <PageHeader
        title="Memory Domains"
        actions={(
          <>
            <Button variant="secondary" disabled={loading} aria-busy={loading} onClick={refreshDomains}>
              <RefreshCcw size={16} aria-hidden="true" />
              {loading ? "Refreshing..." : "Refresh"}
            </Button>
            <Button onClick={openCreateDialog}>
              <Plus size={16} aria-hidden="true" />
              New Domain
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <SurfaceCard className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            <Field label="Search">
              <SearchField
                ariaLabel="Search memory domains"
                className="w-full"
                disabled={loading}
                placeholder="Search memory domains..."
                value={query}
                onChange={setQuery}
              />
            </Field>
            <Field label="Status">
              <SelectInput
                aria-label="Filter memory domains by status"
                className="w-full"
                disabled={loading}
                value={statusFilter}
                onChange={(event) => setStatusFilter(event.target.value as DomainStatusFilter)}
              >
                {domainStatusFilterOptions.map((option) => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </SelectInput>
            </Field>
            <Field label="Sort">
              <SelectInput
                aria-label="Sort memory domains"
                className="w-full"
                disabled={loading}
                value={sortMode}
                onChange={(event) => setSortMode(event.target.value as DomainSortMode)}
              >
                {domainSortOptions.map((option) => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </SelectInput>
            </Field>
          </div>
        </SurfaceCard>

        <DataTable<MemoryWorkDomainRecord>
          onRowClick={(row) => openPreviewDialog(row)}
          rowKey={(row) => row.domainCode}
          rows={pagination.visibleItems}
          tableClassName="table-fixed min-w-[900px]"
          pagination={<TablePagination itemLabel="memory domains" {...pagination.paginationProps} />}
          title="Domains"
          emptyState="No memory domains match the current filters."
          columns={[
            {
              id: "domain",
              header: "Domain",
              width: "290px",
              cellClassName: "min-w-0",
              render: (row) => (
                <div className="flex min-w-0 items-center gap-3">
                  <span className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-surface-container)] text-[var(--color-primary)]">
                    <Boxes size={17} aria-hidden="true" />
                  </span>
                  <div className="min-w-0">
                    <p className="truncate font-medium text-[var(--color-on-surface)]" title={row.name}>{row.name}</p>
                    <p className="truncate font-mono text-[12px] text-[var(--color-on-surface-variant)]" title={row.domainCode}>{row.domainCode}</p>
                  </div>
                </div>
              ),
            },
            {
              id: "description",
              header: "Description",
              width: "340px",
              cellClassName: "min-w-0",
              render: (row) => (
                <p className="line-clamp-2 text-[13px] text-[var(--color-on-surface-variant)]">
                  {row.description || "No description"}
                </p>
              ),
            },
            {
              id: "status",
              header: "Status",
              width: "140px",
              render: (row) => <DomainStatusBadge status={row.status} />,
            },
            {
              id: "updatedAt",
              header: "Updated",
              width: "150px",
              render: (row) => (
                <span className="whitespace-nowrap text-[13px] text-[var(--color-on-surface)]">
                  {formatUpdatedAt(row.updatedAt)}
                </span>
              ),
            },
            {
              align: "right",
              cellClassName: "sticky right-0 z-10 w-24 border-l border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[-8px_0_12px_rgba(0,0,0,0.04)] group-hover:bg-[var(--color-hover-surface)]",
              header: "Actions",
              headerClassName: "sticky right-0 z-20 w-24 border-l border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] shadow-[-8px_0_12px_rgba(0,0,0,0.04)]",
              id: "actions",
              width: "100px",
              render: (row) => (
                <div className="flex justify-end">
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-expanded={openRowMenu?.domainCode === row.domainCode}
                    aria-label={`Open actions for ${row.domainCode}`}
                    disabled={rowBusyCode === row.domainCode}
                    onClick={(event) => {
                      event.stopPropagation();
                      toggleRowMenu(row, event.currentTarget);
                    }}
                  >
                    <MoreHorizontal size={16} aria-hidden="true" />
                  </Button>
                </div>
              ),
            },
          ]}
        />
      </div>

      {openRowMenu && openMenuRow ? createPortal(
        <div
          ref={rowMenuRef}
          className="fixed z-[80] w-40 rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-2 text-left text-sm shadow-[0_12px_28px_rgba(0,0,0,0.22)]"
          role="menu"
          aria-label={`Actions for ${openMenuRow.domainCode}`}
          style={{ left: openRowMenu.left, top: openRowMenu.top }}
        >
          <button
            className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
            role="menuitem"
            type="button"
            onClick={() => {
              setOpenRowMenu(null);
              openPreviewDialog(openMenuRow);
            }}
          >
            Preview
          </button>
          <button
            className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
            role="menuitem"
            type="button"
            onClick={() => {
              setOpenRowMenu(null);
              openEditDialog(openMenuRow);
            }}
          >
            Edit
          </button>
          {openMenuRow.status === "ACTIVE" ? (
            <button
              className="block w-full rounded-lg px-3 py-2 text-left text-[var(--color-error)] hover:bg-[var(--color-hover-surface)]"
              role="menuitem"
              type="button"
              onClick={() => {
                setOpenRowMenu(null);
                setDisableTarget(openMenuRow);
              }}
            >
              Disable
            </button>
          ) : (
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              role="menuitem"
              type="button"
              onClick={() => void handleEnable(openMenuRow)}
            >
              Enable
            </button>
          )}
        </div>,
        document.body,
      ) : null}

      {editor ? (
        <DomainEditorDialog
          form={editorForm}
          mode={editor.mode}
          record={editor.record}
          submitting={submitting}
          onChange={setEditorForm}
          onClose={closeEditor}
          onSubmit={() => void saveDomain()}
          onSwitchMode={switchEditorMode}
        />
      ) : null}

      <ConfirmDialog
        cancelText="Cancel"
        confirmText={rowBusyCode === disableTarget?.domainCode ? "Disabling..." : "Disable"}
        description={disableTarget ? `Disable ${disableTarget.name}? It will be removed from active domain targets.` : ""}
        open={Boolean(disableTarget)}
        pending={rowBusyCode === disableTarget?.domainCode}
        title="Disable memory domain"
        tone="danger"
        onCancel={() => {
          if (rowBusyCode === disableTarget?.domainCode) return;
          setDisableTarget(null);
        }}
        onConfirm={() => void confirmDisable()}
      />
    </>
  );
}

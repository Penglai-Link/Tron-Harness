import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import {
  Boxes,
  MoreHorizontal,
  Save,
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
  SelectInput,
  ConfirmDialog,
  DataTable,
  Field,
  PageHeader,
  SearchField,
  TablePagination,
  TextArea,
  TextInput,
  useDismissibleLayer,
  useTablePagination,
  useToast,
} from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { SelectItem } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

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
  onChange,
  onClose,
  onSubmit,
  submitting,
}: {
  form: MemoryDomainFormState;
  mode: EditorState["mode"];
  onChange: (nextForm: MemoryDomainFormState) => void;
  onClose: () => void;
  onSubmit: () => void;
  submitting: boolean;
}) {
  const { t } = useTranslation(["pages"]);
  const title = mode === "create"
    ? t("pages:memoryDomains.createTitle", "New Memory Domain")
    : mode === "preview"
      ? t("pages:memoryDomains.detailTitle", "Memory Domain")
      : t("pages:memoryDomains.editTitle", "Edit Memory Domain");
  const readOnly = mode === "preview";

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        showCloseButton
      >
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        <div className="grid gap-5">

          <div className="grid gap-5 md:grid-cols-2">
            <Field label={t("pages:memoryDomains.fields.code", "Domain Code")}>
              <TextInput
                readOnly={readOnly}
                value={form.domainCode}
                placeholder="frontend"
                onChange={(event) => onChange({ ...form, domainCode: event.target.value })}
              />
            </Field>
            <Field label={t("pages:memoryDomains.fields.status", "Status")}>
              <SelectInput
                readOnly={readOnly}
                value={form.status}
                onValueChange={(event) => onChange({ ...form, status: event as MemoryWorkDomainRecord["status"] })}
              >
                <SelectItem value="ACTIVE">ACTIVE</SelectItem>
                <SelectItem value="DISABLED">DISABLED</SelectItem>
              </SelectInput>
            </Field>
          </div>

          <Field label={t("pages:memoryDomains.fields.name", "Name")}>
            <TextInput
              readOnly={readOnly}
              value={form.name}
              placeholder="Frontend Engineering"
              onChange={(event) => onChange({ ...form, name: event.target.value })}
            />
          </Field>

          <Field label={t("pages:memoryDomains.fields.description", "Description")}>
            <TextArea
              readOnly={readOnly}
              value={form.description}
              placeholder="Explain what this domain is used for."
              onChange={(event) => onChange({ ...form, description: event.target.value })}
            />
          </Field>
        </div>

        {!readOnly && (
          <DialogFooter className="m-0 shrink-0 flex-row justify-end gap-[12px] rounded-none border-0 bg-transparent p-0">
            <Button type="button" size="lg" variant="outline" onClick={onClose}>{t("pages:memoryDomains.actions.cancel", "Cancel")}</Button>
            <Button type="button" size="lg" disabled={submitting} aria-busy={submitting} onClick={onSubmit}>
              <Save size={16} aria-hidden="true" />
              {submitting ? t("pages:memoryDomains.actions.saving", "Saving...") : t("pages:memoryDomains.actions.save", "Save")}
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}

export function PlatformMemoryDomainsPage() {
  const { t } = useTranslation(["pages"]);
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
        if (active) toast.error(error instanceof Error ? error.message : t("pages:memoryDomains.errors.load", "Memory domains could not be loaded"));
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

        toast.success(t("pages:memoryDomains.messages.created", "{{name}} created", { name: created.name }));
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
        toast.success(t("pages:memoryDomains.messages.saved", "{{name}} saved", { name: saved.name }));
      }

      closeEditor();
      refreshDomains();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:memoryDomains.errors.save", "Memory domain could not be saved"));
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
      toast.success(t("pages:memoryDomains.messages.enabled", "{{name}} enabled", { name: updated.name }));
      refreshDomains();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:memoryDomains.errors.enable", "Memory domain could not be enabled"));
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
      toast.success(t("pages:memoryDomains.messages.disabled", "{{name}} disabled", { name: updated.name }));
      refreshDomains();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("pages:memoryDomains.errors.disable", "Memory domain could not be disabled"));
    } finally {
      setDisableTarget(null);
      setRowBusyCode("");
    }
  }

  return (
    <>
      <PageHeader
        title={t("pages:memoryDomains.title", "Memory Domains")}
        actions={(
          <>
            <Button size="lg" variant="outline" disabled={loading} aria-busy={loading} onClick={refreshDomains}>
              {loading ? t("pages:memoryDomains.actions.refreshing", "Refreshing...") : t("pages:memoryDomains.actions.refresh", "Refresh")}
            </Button>
            <Button size="lg" onClick={openCreateDialog}>
              {t("pages:memoryDomains.actions.new", "New Domain")}
            </Button>
          </>
        )}
      />

      <div className="space-y-5">
        <DataTable<MemoryWorkDomainRecord>
          onRowClick={(row) => openPreviewDialog(row)}
          rowKey={(row) => row.domainCode}
          rows={pagination.visibleItems}
          tableClassName="table-fixed min-w-[900px]"
          pagination={<TablePagination itemLabel={t("pages:memoryDomains.pagination.itemLabel", "memory domains")} {...pagination.paginationProps} />}
          title={t("pages:memoryDomains.table.title", "Domains")}
          toolbar={(
            <>
              <SearchField
                ariaLabel={t("pages:memoryDomains.aria.search", "Search memory domains")}
                className="min-w-0 grow basis-full sm:basis-0 sm:max-w-[360px]"
                disabled={loading}
                placeholder={t("pages:memoryDomains.placeholders.search", "Search domains")}
                value={query}
                onChange={setQuery}
              />
              <SelectInput
                aria-label={t("pages:memoryDomains.aria.status", "Filter memory domains by status")}
                className="w-full sm:w-auto"
                disabled={loading}
                value={statusFilter}
                onValueChange={(event) => setStatusFilter(event as DomainStatusFilter)}
              >
                {domainStatusFilterOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>{t(`pages:memoryDomains.status.${option.value}`, option.label)}</SelectItem>
                ))}
              </SelectInput>
              <SelectInput
                aria-label={t("pages:memoryDomains.aria.sort", "Sort memory domains")}
                className="w-full sm:w-auto"
                disabled={loading}
                value={sortMode}
                onValueChange={(event) => setSortMode(event as DomainSortMode)}
              >
                {domainSortOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>{t(`pages:memoryDomains.sort.${option.value}`, option.label)}</SelectItem>
                ))}
              </SelectInput>
            </>
          )}
          emptyState={t("pages:memoryDomains.table.empty", "No domains match the current filters.")}
          columns={[
            {
              id: "domain",
              header: t("pages:memoryDomains.columns.domain", "Domain"),
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
              header: t("pages:memoryDomains.columns.description", "Description"),
              width: "340px",
              cellClassName: "min-w-0",
              render: (row) => (
                <p className="line-clamp-2 text-[13px] text-[var(--color-on-surface-variant)]">
                  {row.description || t("pages:memoryDomains.table.noDescription", "No description")}
                </p>
              ),
            },
            {
              id: "status",
              header: t("pages:memoryDomains.columns.status", "Status"),
              width: "140px",
              render: (row) => <Badge variant={row.status === 'ACTIVE' ? 'success' : 'error'} >{row.status}</Badge>,
            },
            {
              id: "updatedAt",
              header: t("pages:memoryDomains.columns.updated", "Updated"),
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
              header: t("pages:memoryDomains.columns.actions", "Actions"),
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
            {t("pages:memoryDomains.actions.preview", "Preview")}
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
            {t("pages:memoryDomains.actions.edit", "Edit")}
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
              {t("pages:memoryDomains.actions.disable", "Disable")}
            </button>
          ) : (
            <button
              className="block w-full rounded-lg px-3 py-2 text-left hover:bg-[var(--color-hover-surface)]"
              role="menuitem"
              type="button"
              onClick={() => void handleEnable(openMenuRow)}
            >
              {t("pages:memoryDomains.actions.enable", "Enable")}
            </button>
          )}
        </div>,
        document.body,
      ) : null}

      {editor ? (
        <DomainEditorDialog
          form={editorForm}
          mode={editor.mode}
          submitting={submitting}
          onChange={setEditorForm}
          onClose={closeEditor}
          onSubmit={() => void saveDomain()}
        />
      ) : null}

      <ConfirmDialog
        cancelText={t("pages:memoryDomains.actions.cancel", "Cancel")}
        confirmText={rowBusyCode === disableTarget?.domainCode ? t("pages:memoryDomains.actions.disabling", "Disabling...") : t("pages:memoryDomains.actions.disable", "Disable")}
        description={disableTarget ? t("pages:memoryDomains.confirm.description", "{{name}} will be removed from active memory targets.", { name: disableTarget.name }) : ""}
        open={Boolean(disableTarget)}
        pending={rowBusyCode === disableTarget?.domainCode}
        title={t("pages:memoryDomains.confirm.title", "Disable domain?")}
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

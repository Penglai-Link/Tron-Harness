import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  Copy,
  Filter,
  MoreHorizontal,
  Plus,
  RefreshCcw,
  RotateCcw,
  Trash2,
  X,
} from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SelectItem } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import {
  ConfirmDialog,
  DataTable,
  Field,
  InspectorPanel,
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TablePagination,
  TextArea,
  TextInput,
  useConsoleShellConfig,
  useLocaleFormatters,
  useLocaleText,
  useOpenRightSidebar,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  deletePlatformPrompt,
  deleteWorkspacePrompt,
  duplicatePlatformPrompt,
  duplicateWorkspacePrompt,
  exportWorkspacePrompts,
  revertWorkspacePrompt,
  renderPromptTemplateApi,
  setPlatformPromptEnabled,
  setWorkspacePromptEnabled,
} from "@/domain/prompt/api";
import {
  promptCategoryLabel,
  promptCategoryOptions,
  promptSourceOptions,
} from "@/domain/prompt/constants";
import { usePromptPage } from "@/domain/prompt/hooks";
import {
  deletePlatformPromptMock,
  deleteWorkspacePromptMock,
  duplicatePlatformPromptMock,
  duplicateWorkspacePromptMock,
  exportWorkspacePromptsMock,
  renderPromptTemplateMock,
  revertWorkspacePromptMock,
  setPlatformPromptEnabledMock,
  setWorkspacePromptEnabledMock,
} from "@/domain/prompt/mock";
import {
  getWorkspacePromptActions,
  type PromptCategory,
  type PromptPageQuery,
  type PromptRenderResult,
  type PromptRecord,
  type PromptScopeKind,
  type PromptSourceKind,
} from "@/domain/prompt/model";
import { cn } from "@/shared/lib/utils";

const workspaceCategoryTabs: Array<{ label: string; value: PromptCategory | "" }> = [
  { label: "All Prompts", value: "" },
  { label: "System", value: "SYSTEM" },
  { label: "Persona", value: "PERSONA" },
  { label: "Tools", value: "TOOL" },
  { label: "Templates", value: "TEMPLATE" },
];

function pageBounds(current: number, size: number, total: number, visible: number) {
  if (!total || !visible) return { endItem: 0, startItem: 0 };
  const startItem = (current - 1) * size + 1;
  return { endItem: startItem + visible - 1, startItem };
}

function sourceTone(sourceKind: PromptSourceKind) {
  if (sourceKind === "LOCAL" || sourceKind === "PLATFORM") return "success" as const;
  if (sourceKind === "OVERRIDE") return "warning" as const;
  return "info" as const;
}

function promptVariableValues(prompt: PromptRecord) {
  return Object.fromEntries(prompt.variables.map((variable) => [variable.name, ""]));
}

function PromptInspector({
  onClose,
  prompt,
  useApi,
}: {
  onClose: () => void;
  prompt: PromptRecord;
  useApi: boolean;
}) {
  const pt = useLocaleText();
  const toast = useToast();
  const [testing, setTesting] = useState(false);
  const [preview, setPreview] = useState<PromptRenderResult | null>(null);
  const [variables, setVariables] = useState<Record<string, string>>(() => promptVariableValues(prompt));

  useEffect(() => {
    setPreview(null);
    setVariables(promptVariableValues(prompt));
  }, [prompt]);

  async function copyPrompt() {
    await navigator.clipboard?.writeText(prompt.promptBody);
    toast.success(pt("Prompt body copied"));
  }

  async function runPreview() {
    if (testing) return;
    setTesting(true);
    try {
      const payload = {
        promptBody: prompt.promptBody,
        variables,
      };
      const result = useApi ? await renderPromptTemplateApi(payload) : renderPromptTemplateMock(payload);
      setPreview(result);
      toast.success(pt("Preview completed"));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("Preview failed"));
    } finally {
      setTesting(false);
    }
  }

  return (
    <InspectorPanel title={pt("Prompt Details")}>
      <div className="flex items-start justify-between">
        <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{prompt.displayName}</p>
        <Button size="icon" variant="ghost" aria-label={pt("Close prompt details")} onClick={onClose}><X size={16} aria-hidden="true" /></Button>
      </div>
      <div className="mt-5 rounded-lg bg-[var(--color-surface-container-low)] p-4">
        <pre className="whitespace-pre-wrap text-[13px] leading-6">{prompt.promptBody}</pre>
      </div>
      <div className="mt-5 flex justify-end">
        <Button size="sm" variant="secondary" onClick={() => void copyPrompt()}><Copy size={14} aria-hidden="true" />{pt("Copy")}</Button>
      </div>
      <div className="mt-6 grid gap-3">
        <h3 className="text-sm font-semibold">{pt("Test Variables")}</h3>
        {prompt.variables.map((variable) => (
          <Field label={variable.name} key={variable.name}>
            {variable.name === "user_query" ? (
              <TextArea value={variables[variable.name] ?? ""} onChange={(event) => setVariables((current) => ({ ...current, [variable.name]: event.target.value }))} />
            ) : (
              <TextInput value={variables[variable.name] ?? ""} onChange={(event) => setVariables((current) => ({ ...current, [variable.name]: event.target.value }))} />
            )}
          </Field>
        ))}
        <Button disabled={testing} aria-busy={testing} onClick={() => void runPreview()}>{pt("Render Preview")}</Button>
        {preview ? <pre className="max-h-56 overflow-auto whitespace-pre-wrap rounded-lg bg-[var(--color-surface-container-low)] p-3 text-xs leading-5">{preview.renderedPrompt}</pre> : null}
      </div>
    </InspectorPanel>
  );
}

function DuplicatePromptDialog({
  category,
  name,
  onCancel,
  onConfirm,
  pending,
}: {
  category: PromptCategory;
  name: string;
  onCancel: () => void;
  onConfirm: (targetCategory: PromptCategory) => void;
  pending: boolean;
}) {
  const pt = useLocaleText();
  const [targetCategory, setTargetCategory] = useState(category);

  return createPortal(
    <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/45 p-4">
      <div className="w-[min(92vw,28rem)] rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-xl" role="dialog" aria-modal="true" aria-label={pt("Duplicate prompt")}>
        <div className="border-b border-[var(--color-outline-variant)] px-5 py-4">
          <h2 className="text-base font-semibold">{pt("Duplicate")}</h2>
          <p className="mt-1 truncate text-sm text-[var(--color-on-surface-variant)]">{name}</p>
        </div>
        <div className="px-5 py-5">
          <label className="grid gap-2 text-sm font-medium">
            {pt("Target category")}
            <SelectInput value={targetCategory} onValueChange={(value) => setTargetCategory(value as PromptCategory)}>
              {promptCategoryOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(option.label)}</SelectItem>)}
            </SelectInput>
          </label>
        </div>
        <div className="flex justify-end gap-2 border-t border-[var(--color-outline-variant)] px-5 py-4">
          <Button variant="ghost" disabled={pending} onClick={onCancel}>{pt("Cancel")}</Button>
          <Button disabled={pending} aria-busy={pending} onClick={() => onConfirm(targetCategory)}>
            <Copy size={15} aria-hidden="true" />
            {pt("Duplicate")}
          </Button>
        </div>
      </div>
    </div>,
    document.body,
  );
}

export function PromptManagementPage({ scopeKind }: { scopeKind: PromptScopeKind }) {
  const navigate = useNavigate();
  const openRightSidebar = useOpenRightSidebar();
  const pt = useLocaleText();
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const dataSource = getServiceDataSource("prompt");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const workspaceCode = scopeKind === "WORKSPACE" ? activeWorkspace?.workspaceCode ?? "" : "";
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<PromptCategory | "">("");
  const [sourceKind, setSourceKind] = useState<PromptSourceKind | "">("");
  const [enabled, setEnabled] = useState<"" | "true" | "false">("");
  const [busyCode, setBusyCode] = useState("");
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [promptPanelOpen, setPromptPanelOpen] = useState(true);
  const [selectedCode, setSelectedCode] = useState("");
  const [duplicateTarget, setDuplicateTarget] = useState<PromptRecord | null>(null);
  const [confirmTarget, setConfirmTarget] = useState<{ action: "DELETE" | "REVERT"; record: PromptRecord } | null>(null);

  const query = useMemo<PromptPageQuery>(() => ({
    category: category || undefined,
    current,
    enabled: enabled === "" ? undefined : enabled === "true",
    search: search.trim() || undefined,
    size,
    sourceKind: sourceKind || undefined,
    workspaceCode: workspaceCode || undefined,
  }), [category, current, enabled, search, size, sourceKind, workspaceCode]);
  const { error, loading, page, refresh } = usePromptPage(scopeKind, query);
  const pageCount = Math.max(1, Math.ceil(page.total / Math.max(1, page.size)));
  const bounds = pageBounds(page.current, page.size, page.total, page.records.length);
  const selectedPrompt = page.records.find((record) => record.promptCode === selectedCode) ?? page.records[0];
  const rightPanel = useMemo(() => (
    scopeKind === "WORKSPACE" && selectedPrompt && promptPanelOpen ? (
      <PromptInspector
        key={selectedPrompt.promptCode}
        onClose={() => setPromptPanelOpen(false)}
        prompt={selectedPrompt}
        useApi={dataSource === "api"}
      />
    ) : undefined
  ), [dataSource, promptPanelOpen, scopeKind, selectedPrompt]);

  useConsoleShellConfig({ rightPanel });

  useEffect(() => {
    setCurrent(1);
    setDuplicateTarget(null);
    setConfirmTarget(null);
  }, [category, enabled, size, sourceKind, workspaceCode]);

  function selectPrompt(record: PromptRecord) {
    setSelectedCode(record.promptCode);
    setPromptPanelOpen(true);
    openRightSidebar();
  }

  function detailHref(record: PromptRecord, mode?: "override") {
    const params = new URLSearchParams({ promptCode: record.promptCode });
    if (mode) params.set("mode", mode);
    return scopeKind === "WORKSPACE"
      ? `/prompts/detail?${params.toString()}`
      : `/settings/prompts/detail?${params.toString()}`;
  }

  async function toggleEnabled(record: PromptRecord) {
    if (busyCode) return;
    setBusyCode(record.promptCode);
    try {
      if (scopeKind === "WORKSPACE") {
        const payload = { enabled: !record.enabled, expectedVersion: record.version, promptCode: record.promptCode, workspaceCode };
        await (dataSource === "api" ? setWorkspacePromptEnabled(payload) : setWorkspacePromptEnabledMock(payload));
      } else {
        const payload = { enabled: !record.enabled, expectedVersion: record.version, promptCode: record.promptCode };
        await (dataSource === "api" ? setPlatformPromptEnabled(payload) : setPlatformPromptEnabledMock(payload));
      }
      toast.success(record.enabled ? pt("Prompt disabled") : pt("Prompt enabled"));
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("Status update failed"));
    } finally {
      setBusyCode("");
    }
  }

  async function confirmDestructiveAction() {
    if (!confirmTarget || busyCode) return;
    const { action, record } = confirmTarget;
    setBusyCode(record.promptCode);
    try {
      if (action === "REVERT") {
        const payload = { promptCode: record.promptCode, workspaceCode };
        await (dataSource === "api" ? revertWorkspacePrompt(payload) : revertWorkspacePromptMock(payload));
        toast.success(pt("Override reverted"));
      } else if (scopeKind === "WORKSPACE") {
        const payload = { promptCode: record.promptCode, workspaceCode };
        await (dataSource === "api" ? deleteWorkspacePrompt(payload) : deleteWorkspacePromptMock(payload));
        toast.success(pt("Prompt deleted"));
      } else {
        const payload = { promptCode: record.promptCode };
        await (dataSource === "api" ? deletePlatformPrompt(payload) : deletePlatformPromptMock(payload));
        toast.success(pt("Prompt deleted"));
      }
      setConfirmTarget(null);
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("Action failed"));
    } finally {
      setBusyCode("");
    }
  }

  async function duplicatePrompt(targetCategory: PromptCategory) {
    if (!duplicateTarget || busyCode) return;
    setBusyCode(duplicateTarget.promptCode);
    try {
      if (scopeKind === "WORKSPACE") {
        const payload = { promptCode: duplicateTarget.promptCode, targetCategory, workspaceCode };
        await (dataSource === "api" ? duplicateWorkspacePrompt(payload) : duplicateWorkspacePromptMock(payload));
      } else {
        const payload = { promptCode: duplicateTarget.promptCode, targetCategory };
        await (dataSource === "api" ? duplicatePlatformPrompt(payload) : duplicatePlatformPromptMock(payload));
      }
      setDuplicateTarget(null);
      toast.success(pt("Prompt duplicated"));
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("Duplicate failed"));
    } finally {
      setBusyCode("");
    }
  }

  async function exportPrompts() {
    if (!workspaceCode) return;
    try {
      const payload = { ...query, workspaceCode };
      const result = dataSource === "api"
        ? await exportWorkspacePrompts(payload)
        : exportWorkspacePromptsMock(payload);
      const url = URL.createObjectURL(new Blob([result.content], { type: "application/json" }));
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = result.fileName;
      anchor.click();
      URL.revokeObjectURL(url);
      toast.success(pt("Prompts exported"));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("Export failed"));
    }
  }

  function actionItems(record: PromptRecord) {
    const workspaceActions = scopeKind === "WORKSPACE"
      ? getWorkspacePromptActions(record.sourceKind)
      : (["EDIT", "DUPLICATE", "SET_ENABLED", "DELETE"] as const);
    return workspaceActions.map((action) => {
      if (action === "EDIT" || action === "OVERRIDE") {
        return <DropdownMenuItem key={action} onClick={() => navigate(detailHref(record, action === "OVERRIDE" ? "override" : undefined))}>{pt(action === "OVERRIDE" ? "Create override" : "Edit")}</DropdownMenuItem>;
      }
      if (action === "DUPLICATE") {
        return <DropdownMenuItem key={action} onClick={() => setDuplicateTarget(record)}><Copy aria-hidden="true" />{pt("Duplicate")}</DropdownMenuItem>;
      }
      if (action === "SET_ENABLED") {
        return <DropdownMenuItem key={action} onClick={() => void toggleEnabled(record)}>{pt(record.enabled ? "Disable" : "Enable")}</DropdownMenuItem>;
      }
      if (action === "REVERT") {
        return <DropdownMenuItem key={action} onClick={() => setConfirmTarget({ action, record })}><RotateCcw aria-hidden="true" />{pt("Revert")}</DropdownMenuItem>;
      }
      return <DropdownMenuItem key={action} variant="destructive" onClick={() => setConfirmTarget({ action: "DELETE", record })}><Trash2 aria-hidden="true" />{pt("Delete")}</DropdownMenuItem>;
    });
  }

  return (
    <>
      {scopeKind === "WORKSPACE" ? (
        <>
          <PageHeader
            title={pt("Prompt Management")}
            actions={(
              <>
                <Button size="lg" variant="outline" disabled={!workspaceCode} onClick={() => void exportPrompts()}>{pt("Bulk Export")}</Button>
                <Button size="lg" disabled={!workspaceCode} onClick={() => navigate("/prompts/new")}>{pt("New")}</Button>
              </>
            )}
          />
          <div className="mt-5">
            <SurfaceCard className="p-0">
              <div className="border-b border-[var(--color-outline-variant)] px-5 pt-4">
                <div className="flex flex-wrap gap-1">
                  {workspaceCategoryTabs.map((item) => (
                    <button
                      className={cn("border-b-2 px-4 py-2 text-sm", category === item.value ? "border-[var(--color-primary)] font-medium" : "border-transparent text-[var(--color-on-surface-variant)]")}
                      type="button"
                      key={item.value || "all"}
                      onClick={() => { setCategory(item.value); setCurrent(1); }}
                    >
                      {pt(item.label)}
                    </button>
                  ))}
                </div>
                <div className="flex flex-wrap items-center gap-2 py-4">
                  <SearchField ariaLabel={pt("Search prompts")} placeholder={pt("Search prompts...")} value={search} onChange={(value) => { setSearch(value); setCurrent(1); }} />
                  <Button size="lg" variant="ghost" onClick={() => setFiltersOpen((open) => !open)}><Filter size={14} aria-hidden="true" />{pt("Filter")}</Button>
                  {loading ? <span className="text-sm text-[var(--color-on-surface-variant)]">{pt("Loading...")}</span> : null}
                </div>
                {filtersOpen ? (
                  <div className="flex flex-wrap items-center gap-3 pb-4">
                    <SelectInput aria-label={pt("Source")} value={sourceKind} onValueChange={(value) => setSourceKind((value ?? "") as PromptSourceKind | "")}>
                      <SelectItem value="">{pt("All sources")}</SelectItem>
                      {promptSourceOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(option.label)}</SelectItem>)}
                    </SelectInput>
                    <SelectInput aria-label={pt("Status")} value={enabled} onValueChange={(value) => setEnabled((value ?? "") as typeof enabled)}>
                      <SelectItem value="">{pt("All statuses")}</SelectItem>
                      <SelectItem value="true">{pt("Enabled")}</SelectItem>
                      <SelectItem value="false">{pt("Disabled")}</SelectItem>
                    </SelectInput>
                  </div>
                ) : null}
              </div>
              {error ? <div className="border-b border-[var(--color-outline-variant)] px-5 py-3 text-sm text-[var(--color-error)]">{error.message}</div> : null}
              {!workspaceCode ? <div className="px-5 py-10 text-center text-sm text-[var(--color-on-surface-variant)]">{pt("Select a workspace to manage prompts.")}</div> : (
                <DataTable
                  variant="embedded"
                  rows={page.records}
                  rowKey={(record) => record.promptCode}
                  pagination={<TablePagination {...bounds} itemLabel={pt("prompts")} page={page.current} pageCount={pageCount} pageSize={page.size} totalItems={page.total} onPageChange={setCurrent} onPageSizeChange={(nextSize) => { setSize(nextSize); setCurrent(1); }} />}
                  columns={[
                    { id: "name", header: pt("Name"), render: (record) => <button className="font-medium text-[var(--color-primary)]" type="button" onClick={() => selectPrompt(record)}>{record.displayName}</button> },
                    { id: "category", header: pt("Category"), render: (record) => <Badge>{pt(promptCategoryLabel(record.category))}</Badge> },
                    { id: "inheritance", header: pt("Inheritance"), render: (record) => record.sourceLabel },
                    { id: "updated", header: pt("Last Updated"), render: (record) => formatDateTime(record.updatedAt, { dateStyle: "medium", timeStyle: "short" }) },
                    { id: "actions", header: pt("Actions"), align: "right", render: (record) => <DropdownMenu><DropdownMenuTrigger render={<Button size="icon" variant="ghost" aria-label={pt("Actions for {{name}}", { name: record.displayName })} disabled={busyCode === record.promptCode}><MoreHorizontal size={16} aria-hidden="true" /></Button>} /><DropdownMenuContent align="end">{actionItems(record)}</DropdownMenuContent></DropdownMenu> },
                  ]}
                />
              )}
            </SurfaceCard>
          </div>
        </>
      ) : (
        <>
          <PageHeader
            title={pt("Platform Prompts")}
            actions={(
              <>
                <Button variant="outline" aria-label={pt("Refresh")} onClick={refresh}><RefreshCcw size={15} aria-hidden="true" /></Button>
                <Button onClick={() => navigate("/settings/prompts/new")}><Plus size={15} aria-hidden="true" />{pt("New")}</Button>
              </>
            )}
          />
          <SurfaceCard className="mt-5 p-0">
            <div className="flex flex-wrap items-center gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <SearchField ariaLabel={pt("Search prompts")} placeholder={pt("Search prompts...")} value={search} onChange={(value) => { setSearch(value); setCurrent(1); }} />
              <SelectInput aria-label={pt("Category")} value={category} onValueChange={(value) => setCategory((value ?? "") as PromptCategory | "")}>
                <SelectItem value="">{pt("All categories")}</SelectItem>
                {promptCategoryOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(option.label)}</SelectItem>)}
              </SelectInput>
              <SelectInput aria-label={pt("Status")} value={enabled} onValueChange={(value) => setEnabled((value ?? "") as typeof enabled)}>
                <SelectItem value="">{pt("All statuses")}</SelectItem>
                <SelectItem value="true">{pt("Enabled")}</SelectItem>
                <SelectItem value="false">{pt("Disabled")}</SelectItem>
              </SelectInput>
              {loading ? <span className="text-sm text-[var(--color-on-surface-variant)]">{pt("Loading...")}</span> : null}
            </div>
            {error ? <div className="border-b border-[var(--color-outline-variant)] px-5 py-3 text-sm text-[var(--color-error)]">{error.message}</div> : null}
            <DataTable
              variant="embedded"
              rows={page.records}
              rowKey={(record) => record.promptCode}
              pagination={<TablePagination {...bounds} itemLabel={pt("prompts")} page={page.current} pageCount={pageCount} pageSize={page.size} totalItems={page.total} onPageChange={setCurrent} onPageSizeChange={(nextSize) => { setSize(nextSize); setCurrent(1); }} />}
              columns={[
                { id: "name", header: pt("Name"), render: (record) => <button className="text-left font-medium text-[var(--color-primary)]" type="button" onClick={() => navigate(detailHref(record))}>{record.displayName}<span className="mt-1 block font-mono text-[11px] font-normal text-[var(--color-on-surface-variant)]">{record.promptCode}</span></button> },
                { id: "category", header: pt("Category"), render: (record) => <Badge>{pt(promptCategoryLabel(record.category))}</Badge> },
                { id: "source", header: pt("Source"), render: (record) => <div><Badge tone={sourceTone(record.sourceKind)}>{pt(record.sourceKind)}</Badge><span className="mt-1 block text-xs text-[var(--color-on-surface-variant)]">{record.sourceLabel}</span></div> },
                { id: "variables", header: pt("Variables"), render: (record) => <Badge tone="info">{record.variables.length}</Badge> },
                { id: "status", header: pt("Status"), render: (record) => <Badge tone={record.enabled ? "success" : "outline"}>{pt(record.enabled ? "Enabled" : "Disabled")}</Badge> },
                { id: "updated", header: pt("Updated"), render: (record) => formatDateTime(record.updatedAt, { dateStyle: "medium", timeStyle: "short" }) },
                { id: "actions", header: pt("Actions"), align: "right", render: (record) => <DropdownMenu><DropdownMenuTrigger render={<Button size="icon" variant="ghost" aria-label={pt("Actions for {{name}}", { name: record.displayName })} disabled={busyCode === record.promptCode}><MoreHorizontal size={16} aria-hidden="true" /></Button>} /><DropdownMenuContent align="end">{actionItems(record)}</DropdownMenuContent></DropdownMenu> },
              ]}
            />
          </SurfaceCard>
        </>
      )}
      {duplicateTarget ? <DuplicatePromptDialog category={duplicateTarget.category} name={duplicateTarget.displayName} pending={busyCode === duplicateTarget.promptCode} onCancel={() => setDuplicateTarget(null)} onConfirm={(targetCategory) => void duplicatePrompt(targetCategory)} /> : null}
      <ConfirmDialog
        open={confirmTarget !== null}
        pending={Boolean(confirmTarget && busyCode === confirmTarget.record.promptCode)}
        title={pt(confirmTarget?.action === "REVERT" ? "Revert override?" : "Delete prompt?")}
        description={confirmTarget?.record.displayName}
        confirmText={pt(confirmTarget?.action === "REVERT" ? "Revert" : "Delete")}
        onCancel={() => setConfirmTarget(null)}
        onConfirm={() => void confirmDestructiveAction()}
      />
    </>
  );
}

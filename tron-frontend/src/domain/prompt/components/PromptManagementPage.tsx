import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { useNavigate } from "react-router";
import {
  Filter,
  MoreHorizontal,
  Plus,
  RefreshCcw,
  RotateCcw,
  Trash2,
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
  PageHeader,
  SearchField,
  SelectInput,
  SurfaceCard,
  TablePagination,
  useConsoleShellConfig,
  useLocaleFormatters,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  deletePlatformPrompt,
  deleteWorkspacePrompt,
  exportWorkspacePrompts,
  revertWorkspacePrompt,
  setPlatformPromptEnabled,
  setWorkspacePromptEnabled,
} from "@/domain/prompt/api";
import {
  promptCategoryLabel,
  promptCategoryOptions,
  promptSourceOptions,
} from "@/domain/prompt/constants";
import {
  usePromptPage,
  usePromptTranslation,
} from "@/domain/prompt/hooks";
import {
  deletePlatformPromptMock,
  deleteWorkspacePromptMock,
  exportWorkspacePromptsMock,
  revertWorkspacePromptMock,
  setPlatformPromptEnabledMock,
  setWorkspacePromptEnabledMock,
} from "@/domain/prompt/mock";
import {
  getWorkspacePromptActions,
  type PromptCategory,
  type PromptPageQuery,
  type PromptRecord,
  type PromptScopeKind,
  type PromptSourceKind,
} from "@/domain/prompt/model";
import { cn } from "@/shared/lib/utils";

const workspaceCategoryTabs: Array<{ label: string; translationKey: string; value: PromptCategory | "" }> = [
  { label: "All Prompts", translationKey: "tabs.all", value: "" },
  { label: "System", translationKey: "tabs.system", value: "SYSTEM" },
  { label: "Persona", translationKey: "tabs.persona", value: "PERSONA" },
  { label: "Tools", translationKey: "tabs.tools", value: "TOOL" },
  { label: "Templates", translationKey: "tabs.templates", value: "TEMPLATE" },
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

export function PromptManagementPage({ scopeKind }: { scopeKind: PromptScopeKind }) {
  const navigate = useNavigate();
  const pt = usePromptTranslation();
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

  useConsoleShellConfig({
    searchPlaceholder: pt("shell.searchPlaceholder", "Search prompts..."),
  });

  useEffect(() => {
    setCurrent(1);
    setConfirmTarget(null);
  }, [category, enabled, size, sourceKind, workspaceCode]);

  function detailHref(record: PromptRecord, mode?: "override") {
    const params = new URLSearchParams({ promptCode: record.promptCode });
    if (mode) params.set("mode", mode);
    return scopeKind === "WORKSPACE"
      ? `/prompts/detail?${params.toString()}`
      : `/settings/prompts/detail?${params.toString()}`;
  }

  function editorHref(record: PromptRecord) {
    return detailHref(record, scopeKind === "WORKSPACE" && record.sourceKind === "INHERITED" ? "override" : undefined);
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
      toast.success(record.enabled
        ? pt("messages.disabled", "Prompt disabled")
        : pt("messages.enabled", "Prompt enabled"));
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.statusUpdate", "Status update failed"));
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
        toast.success(pt("messages.overrideReverted", "Override reverted"));
      } else if (scopeKind === "WORKSPACE") {
        const payload = { promptCode: record.promptCode, workspaceCode };
        await (dataSource === "api" ? deleteWorkspacePrompt(payload) : deleteWorkspacePromptMock(payload));
        toast.success(pt("messages.deleted", "Prompt deleted"));
      } else {
        const payload = { promptCode: record.promptCode };
        await (dataSource === "api" ? deletePlatformPrompt(payload) : deletePlatformPromptMock(payload));
        toast.success(pt("messages.deleted", "Prompt deleted"));
      }
      setConfirmTarget(null);
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.action", "Action failed"));
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
      toast.success(pt("messages.exported", "Prompts exported"));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.export", "Export failed"));
    }
  }

  function actionItems(record: PromptRecord) {
    const workspaceActions = scopeKind === "WORKSPACE"
      ? getWorkspacePromptActions(record.sourceKind)
      : (["EDIT", "SET_ENABLED", "DELETE"] as const);
    return workspaceActions.map((action) => {
      if (action === "EDIT" || action === "OVERRIDE") {
        return <DropdownMenuItem key={action} onClick={(event) => { event.stopPropagation(); navigate(detailHref(record, action === "OVERRIDE" ? "override" : undefined)); }}>{action === "OVERRIDE" ? pt("actions.createOverride", "Create override") : pt("actions.edit", "Edit")}</DropdownMenuItem>;
      }
      if (action === "SET_ENABLED") {
        return <DropdownMenuItem key={action} onClick={(event) => { event.stopPropagation(); void toggleEnabled(record); }}>{record.enabled ? pt("actions.disable", "Disable") : pt("actions.enable", "Enable")}</DropdownMenuItem>;
      }
      if (action === "REVERT") {
        return <DropdownMenuItem key={action} onClick={(event) => { event.stopPropagation(); setConfirmTarget({ action, record }); }}><RotateCcw aria-hidden="true" />{pt("actions.revert", "Revert")}</DropdownMenuItem>;
      }
      return <DropdownMenuItem key={action} variant="destructive" onClick={(event) => { event.stopPropagation(); setConfirmTarget({ action: "DELETE", record }); }}><Trash2 aria-hidden="true" />{pt("actions.delete", "Delete")}</DropdownMenuItem>;
    });
  }

  return (
    <>
      {scopeKind === "WORKSPACE" ? (
        <>
          <PageHeader
            title={pt("titles.management", "Prompt Management")}
            actions={(
              <>
                <Button size="lg" variant="outline" disabled={!workspaceCode} onClick={() => void exportPrompts()}>{pt("actions.bulkExport", "Bulk Export")}</Button>
                <Button size="lg" disabled={!workspaceCode} onClick={() => navigate("/prompts/new")}>{pt("actions.new", "New")}</Button>
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
                      {pt(item.translationKey, item.label)}
                    </button>
                  ))}
                </div>
                <div className="flex flex-wrap items-center gap-2 py-4">
                  <SearchField ariaLabel={pt("aria.search", "Search prompts")} placeholder={pt("filters.searchPlaceholder", "Search prompts...")} value={search} onChange={(value) => { setSearch(value); setCurrent(1); }} />
                  <Button size="lg" variant="ghost" onClick={() => setFiltersOpen((open) => !open)}><Filter size={14} aria-hidden="true" />{pt("actions.filter", "Filter")}</Button>
                  {loading ? <span className="text-sm text-[var(--color-on-surface-variant)]">{pt("status.loading", "Loading...")}</span> : null}
                </div>
                {filtersOpen ? (
                  <div className="flex flex-wrap items-center gap-3 pb-4">
                    <SelectInput aria-label={pt("fields.source", "Source")} value={sourceKind} onValueChange={(value) => setSourceKind((value ?? "") as PromptSourceKind | "")}>
                      <SelectItem value="">{pt("filters.allSources", "All sources")}</SelectItem>
                      {promptSourceOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(`sources.${option.value}`, option.label)}</SelectItem>)}
                    </SelectInput>
                    <SelectInput aria-label={pt("fields.status", "Status")} value={enabled} onValueChange={(value) => setEnabled((value ?? "") as typeof enabled)}>
                      <SelectItem value="">{pt("filters.allStatuses", "All statuses")}</SelectItem>
                      <SelectItem value="true">{pt("status.enabled", "Enabled")}</SelectItem>
                      <SelectItem value="false">{pt("status.disabled", "Disabled")}</SelectItem>
                    </SelectInput>
                  </div>
                ) : null}
              </div>
              {error ? <div className="border-b border-[var(--color-outline-variant)] px-5 py-3 text-sm text-[var(--color-error)]">{error.message}</div> : null}
              {!workspaceCode ? <div className="px-5 py-10 text-center text-sm text-[var(--color-on-surface-variant)]">{pt("empty.workspaceList", "Select a workspace to manage prompts.")}</div> : (
                <DataTable
                  variant="embedded"
                  rows={page.records}
                  rowKey={(record) => record.promptCode}
                  onRowClick={(record) => navigate(editorHref(record))}
                  pagination={<TablePagination {...bounds} itemLabel={pt("pagination.itemLabel", "prompts")} page={page.current} pageCount={pageCount} pageSize={page.size} totalItems={page.total} onPageChange={setCurrent} onPageSizeChange={(nextSize) => { setSize(nextSize); setCurrent(1); }} />}
                  columns={[
                    { id: "name", header: pt("fields.name", "Name"), render: (record) => <span className="font-medium text-[var(--color-primary)]">{record.displayName}</span> },
                    { id: "category", header: pt("fields.category", "Category"), render: (record) => <Badge>{pt(`categories.${record.category}`, promptCategoryLabel(record.category))}</Badge> },
                    { id: "inheritance", header: pt("fields.inheritance", "Inheritance"), render: (record) => record.sourceLabel },
                    { id: "status", header: pt("fields.status", "Status"), render: (record) => <Badge tone={record.enabled ? "success" : "outline"}>{record.enabled ? pt("status.enabled", "Enabled") : pt("status.disabled", "Disabled")}</Badge> },
                    { id: "updated", header: pt("fields.lastUpdated", "Last Updated"), render: (record) => formatDateTime(record.updatedAt, { dateStyle: "medium", timeStyle: "short" }) },
                    { id: "actions", header: pt("fields.actions", "Actions"), align: "right", render: (record) => <DropdownMenu><DropdownMenuTrigger render={<Button size="icon" variant="ghost" aria-label={pt("aria.actionsFor", "Actions for {{name}}", { name: record.displayName })} disabled={busyCode === record.promptCode} onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}><MoreHorizontal size={16} aria-hidden="true" /></Button>} /><DropdownMenuContent align="end">{actionItems(record)}</DropdownMenuContent></DropdownMenu> },
                  ]}
                />
              )}
            </SurfaceCard>
          </div>
        </>
      ) : (
        <>
          <PageHeader
            title={pt("titles.platform", "Platform Prompts")}
            actions={(
              <>
                <Button variant="outline" aria-label={pt("actions.refresh", "Refresh")} onClick={refresh}><RefreshCcw size={15} aria-hidden="true" /></Button>
                <Button onClick={() => navigate("/settings/prompts/new")}><Plus size={15} aria-hidden="true" />{pt("actions.new", "New")}</Button>
              </>
            )}
          />
          <SurfaceCard className="mt-5 p-0">
            <div className="flex flex-wrap items-center gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <SearchField ariaLabel={pt("aria.search", "Search prompts")} placeholder={pt("filters.searchPlaceholder", "Search prompts...")} value={search} onChange={(value) => { setSearch(value); setCurrent(1); }} />
              <SelectInput aria-label={pt("fields.category", "Category")} value={category} onValueChange={(value) => setCategory((value ?? "") as PromptCategory | "")}>
                <SelectItem value="">{pt("filters.allCategories", "All categories")}</SelectItem>
                {promptCategoryOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(`categories.${option.value}`, option.label)}</SelectItem>)}
              </SelectInput>
              <SelectInput aria-label={pt("fields.status", "Status")} value={enabled} onValueChange={(value) => setEnabled((value ?? "") as typeof enabled)}>
                <SelectItem value="">{pt("filters.allStatuses", "All statuses")}</SelectItem>
                <SelectItem value="true">{pt("status.enabled", "Enabled")}</SelectItem>
                <SelectItem value="false">{pt("status.disabled", "Disabled")}</SelectItem>
              </SelectInput>
              {loading ? <span className="text-sm text-[var(--color-on-surface-variant)]">{pt("status.loading", "Loading...")}</span> : null}
            </div>
            {error ? <div className="border-b border-[var(--color-outline-variant)] px-5 py-3 text-sm text-[var(--color-error)]">{error.message}</div> : null}
            <DataTable
              variant="embedded"
              rows={page.records}
              rowKey={(record) => record.promptCode}
              onRowClick={(record) => navigate(editorHref(record))}
              pagination={<TablePagination {...bounds} itemLabel={pt("pagination.itemLabel", "prompts")} page={page.current} pageCount={pageCount} pageSize={page.size} totalItems={page.total} onPageChange={setCurrent} onPageSizeChange={(nextSize) => { setSize(nextSize); setCurrent(1); }} />}
              columns={[
                { id: "name", header: pt("fields.name", "Name"), render: (record) => <div className="text-left font-medium text-[var(--color-primary)]">{record.displayName}<span className="mt-1 block font-mono text-[11px] font-normal text-[var(--color-on-surface-variant)]">{record.promptCode}</span></div> },
                { id: "category", header: pt("fields.category", "Category"), render: (record) => <Badge>{pt(`categories.${record.category}`, promptCategoryLabel(record.category))}</Badge> },
                { id: "source", header: pt("fields.source", "Source"), render: (record) => <div><Badge tone={sourceTone(record.sourceKind)}>{record.sourceKind}</Badge><span className="mt-1 block text-xs text-[var(--color-on-surface-variant)]">{record.sourceLabel}</span></div> },
                { id: "variables", header: pt("fields.variables", "Variables"), render: (record) => <Badge tone="info">{record.variables.length}</Badge> },
                { id: "status", header: pt("fields.status", "Status"), render: (record) => <Badge tone={record.enabled ? "success" : "outline"}>{record.enabled ? pt("status.enabled", "Enabled") : pt("status.disabled", "Disabled")}</Badge> },
                { id: "updated", header: pt("fields.updated", "Updated"), render: (record) => formatDateTime(record.updatedAt, { dateStyle: "medium", timeStyle: "short" }) },
                { id: "actions", header: pt("fields.actions", "Actions"), align: "right", render: (record) => <DropdownMenu><DropdownMenuTrigger render={<Button size="icon" variant="ghost" aria-label={pt("aria.actionsFor", "Actions for {{name}}", { name: record.displayName })} disabled={busyCode === record.promptCode} onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}><MoreHorizontal size={16} aria-hidden="true" /></Button>} /><DropdownMenuContent align="end">{actionItems(record)}</DropdownMenuContent></DropdownMenu> },
              ]}
            />
          </SurfaceCard>
        </>
      )}
      <ConfirmDialog
        open={confirmTarget !== null}
        pending={Boolean(confirmTarget && busyCode === confirmTarget.record.promptCode)}
        title={confirmTarget?.action === "REVERT"
          ? pt("confirm.revertTitle", "Revert override?")
          : pt("confirm.deleteTitle", "Delete prompt?")}
        description={confirmTarget?.record.displayName}
        confirmText={confirmTarget?.action === "REVERT"
          ? pt("actions.revert", "Revert")
          : pt("actions.delete", "Delete")}
        onCancel={() => setConfirmTarget(null)}
        onConfirm={() => void confirmDestructiveAction()}
      />
    </>
  );
}

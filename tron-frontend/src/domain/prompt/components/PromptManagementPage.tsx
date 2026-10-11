import { MoreHorizontal } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router";

import { DataTable } from "@/components/data-table";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { SearchField } from "@/components/search-field";
import { TablePagination } from "@/components/table-pagination";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { SelectItem } from "@/components/ui/select";
import { SelectInput } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { useToast } from "@/components/ui/toast";
import { deletePrompt, duplicatePrompt as duplicatePromptApi, setPromptEnabled } from "@/domain/prompt/api";
import { promptCategoryLabel, promptFilterCategoryOptions } from "@/domain/prompt/constants";
import { usePromptPage, usePromptTranslation } from "@/domain/prompt/hooks";
import { deletePlatformPromptMock, duplicatePlatformPromptMock, setPlatformPromptEnabledMock } from "@/domain/prompt/mock";
import type { PromptCategory, PromptPage, PromptRecord } from "@/domain/prompt/model";
import { PageHeader, SurfaceCard, useLocaleFormatters, useTabPanelConfig } from "@/shared";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

function pageBounds(current: number, size: number, total: number, visible: number) {
  if (!total || !visible) return { endItem: 0, startItem: 0 };
  const startItem = (current - 1) * size + 1;
  return { endItem: startItem + visible - 1, startItem };
}

function localizedPromptName(record: PromptRecord, locale: string) {
  const translation = record.translations?.find((item) => item.locale === locale);
  return translation?.displayName.trim() || record.displayName;
}

interface PromptManagementReturnState {
  current: number;
  size: number;
  search: string;
  category: PromptCategory | "";
  enabled: "" | "true" | "false";
  page: PromptPage<PromptRecord>;
}

interface PromptManagementLocationState {
  promptManagementReturnId?: string;
}

const promptManagementReturnCache = new Map<string, PromptManagementReturnState>();
let promptManagementReturnSequence = 0;

export function PromptManagementPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const pt = usePromptTranslation();
  const toast = useToast();
  const { currentLocale, formatDateTime } = useLocaleFormatters();
  const dataSource = getServiceDataSource("prompt");
  const routeState = location.state as PromptManagementLocationState | null;
  const returnId = routeState?.promptManagementReturnId;
  const [returnState] = useState<PromptManagementReturnState | null>(() => returnId ? promptManagementReturnCache.get(returnId) ?? null : null);
  const [current, setCurrent] = useState(() => returnState?.current ?? 1);
  const [size, setSize] = useState(() => returnState?.size ?? 15);
  const [search, setSearch] = useState(() => returnState?.search ?? "");
  const [category, setCategory] = useState<PromptCategory | "">(() => returnState?.category ?? "");
  const [enabled, setEnabled] = useState<"" | "true" | "false">(() => returnState?.enabled ?? "");
  const [busyCode, setBusyCode] = useState("");
  const [confirmTarget, setConfirmTarget] = useState<PromptRecord | null>(null);
  const filterKey = JSON.stringify([category, enabled, size]);
  const previousFilterKey = useRef(filterKey);
  const query = useMemo(() => ({
    category: category || undefined,
    current,
    enabled: enabled === "" ? undefined : enabled === "true",
    search: search.trim() || undefined,
    size,
  }), [category, current, enabled, search, size]);
  const { error, loading, page, refresh } = usePromptPage(query, returnState?.page);
  const pageCount = Math.max(1, Math.ceil(page.total / Math.max(1, page.size)));
  const bounds = pageBounds(page.current, page.size, page.total, page.records.length);

  useTabPanelConfig({ searchPlaceholder: pt("shell.searchPlaceholder", "Search prompts...") });

  useEffect(() => {
    if (returnId) promptManagementReturnCache.delete(returnId);
  }, [returnId]);

  function getReturnLocationState(): PromptManagementLocationState {
    const id = `prompt-list-${Date.now()}-${++promptManagementReturnSequence}`;
    promptManagementReturnCache.set(id, { current, size, search, category, enabled, page });
    return { promptManagementReturnId: id };
  }

  useEffect(() => {
    if (previousFilterKey.current !== filterKey) setCurrent(1);
    previousFilterKey.current = filterKey;
    setConfirmTarget(null);
  }, [filterKey]);

  function detailHref(record: PromptRecord) {
    return `/settings/prompts/detail?${new URLSearchParams({ promptCode: record.promptCode }).toString()}`;
  }

  function editHref(record: PromptRecord) {
    return `/settings/prompts/edit?${new URLSearchParams({ promptCode: record.promptCode }).toString()}`;
  }

  async function toggleEnabled(record: PromptRecord) {
    if (busyCode) return;
    setBusyCode(record.promptCode);
    try {
      const payload = { enabled: !record.enabled, expectedVersion: record.version, promptCode: record.promptCode };
      await (dataSource === "api" ? setPromptEnabled(payload) : setPlatformPromptEnabledMock(payload));
      toast.success(record.enabled ? pt("messages.disabled", "Prompt disabled") : pt("messages.enabled", "Prompt enabled"));
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.statusUpdate", "Status update failed"));
    } finally {
      setBusyCode("");
    }
  }

  async function deleteSelectedPrompt() {
    if (!confirmTarget || busyCode) return;
    setBusyCode(confirmTarget.promptCode);
    try {
      const payload = { promptCode: confirmTarget.promptCode };
      await (dataSource === "api" ? deletePrompt(payload) : deletePlatformPromptMock(payload));
      toast.success(pt("messages.deleted", "Prompt deleted"));
      setConfirmTarget(null);
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.action", "Action failed"));
    } finally {
      setBusyCode("");
    }
  }

  async function duplicatePrompt(record: PromptRecord) {
    if (busyCode) return;
    setBusyCode(record.promptCode);
    try {
      const payload = { promptCode: record.promptCode, targetCategory: record.category };
      if (dataSource === "api") await duplicatePromptApi(payload);
      else duplicatePlatformPromptMock(payload);
      toast.success(pt("messages.duplicated", "Prompt duplicated"));
      refresh();
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.action", "Action failed"));
    } finally {
      setBusyCode("");
    }
  }

  function actionItems(record: PromptRecord) {
    const actions = record.builtIn
      ? (record.runtimeRequired ? ["EDIT"] : ["EDIT", "SET_ENABLED"])
      : ["EDIT", "DUPLICATE", "SET_ENABLED", "DELETE"];
    return actions.map((action) => {
      if (action === "EDIT") return <DropdownMenuItem key={action} onClick={(event) => { event.stopPropagation(); navigate(editHref(record), { state: getReturnLocationState() }); }}>{pt("actions.edit", "Edit")}</DropdownMenuItem>;
      if (action === "DUPLICATE") return <DropdownMenuItem key={action} onClick={(event) => { event.stopPropagation(); void duplicatePrompt(record); }}>{pt("actions.duplicate", "Duplicate")}</DropdownMenuItem>;
      if (action === "SET_ENABLED") return <DropdownMenuItem key={action} onClick={(event) => { event.stopPropagation(); void toggleEnabled(record); }}>{record.enabled ? pt("actions.disable", "Disable") : pt("actions.enable", "Enable")}</DropdownMenuItem>;
      return <DropdownMenuItem key={action} variant="destructive" onClick={(event) => { event.stopPropagation(); setConfirmTarget(record); }}>{pt("actions.delete", "Delete")}</DropdownMenuItem>;
    });
  }

  return (
    <>
      <PageHeader
        title={pt("titles.platform", "Platform Prompts")}
        actions={(
          <>
            <Button variant="outline" disabled={loading} onClick={refresh}>

              {pt("actions.refresh", "Refresh")}
            </Button>
            <Button onClick={() => navigate("/settings/prompts/new", { state: getReturnLocationState() })}>{pt("actions.new", "New")}</Button>
          </>
        )}
      />
      <SurfaceCard className="mt-5 p-0">
        <div className="grid grid-cols-1 gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4 sm:grid-cols-3">
          <SearchField ariaLabel={pt("aria.search", "Search prompts")} className="w-full min-w-0" placeholder={pt("filters.searchPlaceholder", "Search prompts...")} value={search} onChange={(value) => { setSearch(value); setCurrent(1); }} />
          <SelectInput aria-label={pt("fields.category", "Category")} className="w-full min-w-0" value={category} onValueChange={(value) => setCategory((value ?? "") as PromptCategory | "")}>
            <SelectItem value="">{pt("filters.allCategories", "All categories")}</SelectItem>
            {promptFilterCategoryOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(`categories.${option.value}`, option.label)}</SelectItem>)}
          </SelectInput>
          <SelectInput aria-label={pt("fields.status", "Status")} className="w-full min-w-0" value={enabled} onValueChange={(value) => setEnabled((value ?? "") as typeof enabled)}>
            <SelectItem value="">{pt("filters.allStatuses", "All statuses")}</SelectItem>
            <SelectItem value="true">{pt("status.enabled", "Enabled")}</SelectItem>
            <SelectItem value="false">{pt("status.disabled", "Disabled")}</SelectItem>
          </SelectInput>
          {loading ? <span className="text-sm text-[var(--color-on-surface-variant)] sm:col-span-3">{pt("status.loading", "Loading...")}</span> : null}
        </div>
        {error ? <div className="border-b border-[var(--color-outline-variant)] px-5 py-3 text-sm text-[var(--color-error)]">{error.message}</div> : null}
        <DataTable
          variant="embedded"
          rows={page.records}
          rowKey={(record) => record.promptCode}
          onRowClick={(record) => navigate(detailHref(record), { state: getReturnLocationState() })}
          pagination={<TablePagination {...bounds} itemLabel={pt("pagination.itemLabel", "prompts")} page={page.current} pageCount={pageCount} pageSize={page.size} totalItems={page.total} onPageChange={setCurrent} onPageSizeChange={(nextSize) => { setSize(nextSize); setCurrent(1); }} />}
          columns={[
            { id: "name", header: pt("fields.name", "Name"), render: (record) => <div className="text-left font-medium text-[var(--color-primary)]">{localizedPromptName(record, currentLocale)}<span className="mt-1 block font-mono text-[11px] font-normal text-[var(--color-on-surface-variant)]">{record.promptCode}</span></div> },
            { id: "category", header: pt("fields.category", "Category"), render: (record) => <Badge>{pt(`categories.${record.category}`, promptCategoryLabel(record.category))}</Badge> },
            { id: "variables", header: pt("fields.variables", "Variables"), render: (record) => <Badge variant="info">{record.variables.length}</Badge> },
            { id: "status", header: pt("fields.status", "Status"), render: (record) => <Badge variant={record.enabled ? "success" : "outline"}>{record.enabled ? pt("status.enabled", "Enabled") : pt("status.disabled", "Disabled")}</Badge> },
            { id: "updated", header: pt("fields.updated", "Updated"), render: (record) => formatDateTime(record.updatedAt, { dateStyle: "medium", timeStyle: "short" }) },
            { id: "actions", header: pt("fields.actions", "Actions"), align: "right", render: (record) => <DropdownMenu><DropdownMenuTrigger render={<Button size="icon" variant="ghost" aria-label={pt("aria.actionsFor", "Actions for {{name}}", { name: localizedPromptName(record, currentLocale) })} disabled={busyCode === record.promptCode} onClick={(event) => event.stopPropagation()} onKeyDown={(event) => event.stopPropagation()}><MoreHorizontal size={16} aria-hidden="true" /></Button>} /><DropdownMenuContent align="end">{actionItems(record)}</DropdownMenuContent></DropdownMenu> },
          ]}
        />
      </SurfaceCard>
      <ConfirmDialog
        open={confirmTarget !== null}
        pending={Boolean(confirmTarget && busyCode === confirmTarget.promptCode)}
        title={pt("confirm.deleteTitle", "Delete prompt?")}
        description={confirmTarget ? localizedPromptName(confirmTarget, currentLocale) : undefined}
        confirmText={pt("actions.delete", "Delete")}
        onCancel={() => setConfirmTarget(null)}
        onConfirm={() => void deleteSelectedPrompt()}
      />
    </>
  );
}

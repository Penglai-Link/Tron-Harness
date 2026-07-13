import {
  useEffect,
  useMemo,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router";
import {
  Copy,
  Download,
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
  useLocaleFormatters,
  useLocaleText,
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

  useEffect(() => {
    setCurrent(1);
    setDuplicateTarget(null);
    setConfirmTarget(null);
  }, [category, enabled, size, sourceKind, workspaceCode]);

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
      <PageHeader
        title={pt(scopeKind === "WORKSPACE" ? "Prompt Management" : "Platform Prompts")}
        actions={(
          <>
            {scopeKind === "WORKSPACE" ? <Button variant="outline" disabled={!workspaceCode} onClick={() => void exportPrompts()}><Download size={15} aria-hidden="true" />{pt("Export")}</Button> : null}
            <Button variant="outline" aria-label={pt("Refresh")} onClick={refresh}><RefreshCcw size={15} aria-hidden="true" /></Button>
            <Button disabled={scopeKind === "WORKSPACE" && !workspaceCode} onClick={() => navigate(scopeKind === "WORKSPACE" ? "/prompts/new" : "/settings/prompts/new")}><Plus size={15} aria-hidden="true" />{pt("New")}</Button>
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
          {scopeKind === "WORKSPACE" ? (
            <SelectInput aria-label={pt("Source")} value={sourceKind} onValueChange={(value) => setSourceKind((value ?? "") as PromptSourceKind | "")}>
              <SelectItem value="">{pt("All sources")}</SelectItem>
              {promptSourceOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(option.label)}</SelectItem>)}
            </SelectInput>
          ) : null}
          <SelectInput aria-label={pt("Status")} value={enabled} onValueChange={(value) => setEnabled((value ?? "") as typeof enabled)}>
            <SelectItem value="">{pt("All statuses")}</SelectItem>
            <SelectItem value="true">{pt("Enabled")}</SelectItem>
            <SelectItem value="false">{pt("Disabled")}</SelectItem>
          </SelectInput>
          {loading ? <span className="text-sm text-[var(--color-on-surface-variant)]">{pt("Loading...")}</span> : null}
        </div>
        {error ? <div className="border-b border-[var(--color-outline-variant)] px-5 py-3 text-sm text-[var(--color-error)]">{error.message}</div> : null}
        {!workspaceCode && scopeKind === "WORKSPACE" ? <div className="px-5 py-10 text-center text-sm text-[var(--color-on-surface-variant)]">{pt("Select a workspace to manage prompts.")}</div> : (
          <DataTable
            variant="embedded"
            rows={page.records}
            rowKey={(record) => record.promptCode}
            pagination={<TablePagination {...bounds} itemLabel={pt("prompts")} page={page.current} pageCount={pageCount} pageSize={page.size} totalItems={page.total} onPageChange={setCurrent} onPageSizeChange={(nextSize) => { setSize(nextSize); setCurrent(1); }} />}
            columns={[
              { id: "name", header: pt("Name"), render: (record) => <button className="text-left font-medium text-[var(--color-primary)]" type="button" onClick={() => navigate(detailHref(record))}>{record.displayName}<span className="mt-1 block font-mono text-[11px] font-normal text-[var(--color-on-surface-variant)]">{record.promptCode}</span></button> },
              { id: "category", header: pt("Category"), render: (record) => <Badge>{pt(promptCategoryLabel(record.category))}</Badge> },
              { id: "source", header: pt("Source"), render: (record) => <div><Badge tone={sourceTone(record.sourceKind)}>{pt(record.sourceKind)}</Badge><span className="mt-1 block text-xs text-[var(--color-on-surface-variant)]">{record.sourceLabel}</span></div> },
              { id: "model", header: pt("Model"), render: (record) => <span className="font-mono text-xs">{record.modelCode ?? pt("Default")}</span> },
              { id: "status", header: pt("Status"), render: (record) => <Badge tone={record.enabled ? "success" : "outline"}>{pt(record.enabled ? "Enabled" : "Disabled")}</Badge> },
              { id: "updated", header: pt("Updated"), render: (record) => formatDateTime(record.updatedAt, { dateStyle: "medium", timeStyle: "short" }) },
              { id: "actions", header: pt("Actions"), align: "right", render: (record) => <DropdownMenu><DropdownMenuTrigger render={<Button size="icon" variant="ghost" aria-label={pt("Actions for {{name}}", { name: record.displayName })} disabled={busyCode === record.promptCode}><MoreHorizontal size={16} aria-hidden="true" /></Button>} /><DropdownMenuContent align="end">{actionItems(record)}</DropdownMenuContent></DropdownMenu> },
            ]}
          />
        )}
      </SurfaceCard>
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

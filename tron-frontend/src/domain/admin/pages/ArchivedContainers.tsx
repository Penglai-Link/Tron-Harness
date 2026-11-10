import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { RotateCcw } from "lucide-react";
import { DataTable } from "@/components/data-table";
import { Button } from "@/components/ui/button";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { TablePagination } from "@/components/table-pagination";
import { useToast } from "@/components/ui/toast";
import type { WorkspaceOption } from "@/shared/types/WorkspaceSpace";
import { fetchArchivedContainers, restoreArchivedContainer, type ArchivedContainer, type ArchivedContainerKind } from "../api/archivedChatsApi";

export function ArchivedContainers({ kind, workspaceCode, onWorkspaceChange, workspaces }: {
  kind: ArchivedContainerKind; workspaceCode: string; onWorkspaceChange: (code: string) => void; workspaces: WorkspaceOption[];
}) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [records, setRecords] = useState<ArchivedContainer[]>([]);
  const [total, setTotal] = useState(0);
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(25);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const load = useCallback(async (page: number, signal?: AbortSignal) => {
    const request = ++generation.current;
    setLoading(true);
    try {
      const query = { kind, workspaceCode: workspaceCode || null, current: page, size };
      let result = await fetchArchivedContainers(query, signal);
      const last = Math.max(1, Math.ceil(result.total / size));
      if (page > last) result = await fetchArchivedContainers({ ...query, current: last }, signal);
      if (request !== generation.current || signal?.aborted) return;
      setRecords(result.records); setTotal(result.total); setCurrent(result.current);
    } catch (error) {
      if (request === generation.current && !signal?.aborted) toast.error(error instanceof Error ? error.message : t("archivedChats.loadFailed"));
    } finally { if (request === generation.current && !signal?.aborted) setLoading(false); }
  }, [kind, workspaceCode, size, toast, t]);
  useEffect(() => {
    const controller = new AbortController();
    void load(1, controller.signal);
    return () => { ++generation.current; controller.abort(); };
  }, [load]);
  const restore = async (record: ArchivedContainer) => {
    setBusy(true);
    try {
      await restoreArchivedContainer(kind, record.code);
      toast.success(t("archivedChats.containerRestored"));
      await load(current);
    } catch (error) { toast.error(error instanceof Error ? error.message : t("archivedChats.restoreFailed")); }
    finally { setBusy(false); }
  };
  const pageCount = Math.max(1, Math.ceil(total / size));
  return <DataTable<ArchivedContainer> rows={loading ? [] : records} rowKey={record => record.code}
    emptyState={t(loading ? "archivedChats.loading" : "archivedChats.emptyContainers")}
    toolbar={<>
      <SelectInput aria-label={t("archivedChats.workspace")} className="w-full sm:w-64" value={workspaceCode || "__all__"} disabled={busy}
        onValueChange={value => onWorkspaceChange(value === "__all__" ? "" : String(value))}>
        <SelectItem value="__all__">{t("archivedChats.allWorkspaces")}</SelectItem>
        {workspaces.map(workspace => <SelectItem key={workspace.workspaceCode} value={workspace.workspaceCode}>{workspace.label || workspace.name}</SelectItem>)}
      </SelectInput>
      <Button className="ml-auto" variant="outline" size="sm" disabled={loading || busy} onClick={() => void load(current)}>{t("archivedChats.refresh")}</Button>
    </>}
    columns={[
      { id: "name", header: t(kind === "Topic" ? "archivedChats.topics" : "archivedChats.projects"), render: record => <div className="min-w-0"><p className="truncate font-medium" title={record.name}>{record.name}</p>{record.path && <p className="truncate text-xs text-muted-foreground" title={record.path}>{record.path}</p>}</div> },
      { id: "workspace", header: t("archivedChats.workspace"), render: record => workspaces.find(workspace => workspace.workspaceCode === record.workspaceCode)?.label || record.workspaceCode },
      { id: "chats", header: t("archivedChats.archivedChatCount"), render: record => record.chatCount },
      { id: "actions", header: t("archivedChats.actions"), width: "96px", align: "right", render: record => <Button variant="ghost" size="icon" aria-label={`${t("archivedChats.restore")} ${record.name}`} disabled={busy || loading} onClick={() => void restore(record)}><RotateCcw /></Button> },
    ]}
    pagination={<TablePagination itemLabel={t("archivedChats.containerItems")} totalItems={total} startItem={total ? (current - 1) * size + 1 : 0} endItem={Math.min(current * size, total)} page={current} pageCount={pageCount} pageSize={size}
      onPageChange={busy || loading ? undefined : page => void load(Math.max(1, Math.min(page, pageCount)))} onPageSizeChange={busy || loading ? undefined : setSize} />}
  />;
}

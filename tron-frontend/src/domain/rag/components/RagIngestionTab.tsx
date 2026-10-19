import { MoreHorizontal, FileText, Scissors, Layers3, CircleCheck } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ConfirmDialog } from "@/components/confirm-dialog/ConfirmDialog";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { Card, CardContent } from "@/components/ui/card";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Table, TableHeader, TableHead, TableBody, TableCell, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { MetricCard } from "@/shared";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { deleteRagTask, controlRagTask } from "../api/ragApi";
import { RagTaskChunksDialog } from "./RagTaskChunksDialog";
import { RagFlowSteps } from "./RagFlowSteps";
import { RagDocumentPreviewDialog, type RagDocumentPreview } from "./RagDocumentPreviewDialog";
import type { RagCorpus, RagTask } from "../model/RagModel";
import { useRagErrorToast } from "../hooks/useRagErrorToast";

const ingestionSteps = [
  { stage: "read", icon: FileText },
  { stage: "chunk", icon: Scissors },
  { stage: "index", icon: Layers3 },
  { stage: "publish", icon: CircleCheck },
];

const taskStates = ["RUNNING", "QUEUED", "WAITING", "COMPLETED", "FAILED", "CANCELLED"] as const;
const taskPriority = (task: RagTask) => task.state === "RUNNING" ? 0 : ["QUEUED", "WAITING"].includes(task.state) ? 1 : 2;
const compareTasks = (left: RagTask, right: RagTask) => {
  const priority = taskPriority(left) - taskPriority(right);
  if (priority || taskPriority(left) !== 1) return priority;
  return (left.fileSizeBytes ?? Number.MAX_SAFE_INTEGER) - (right.fileSizeBytes ?? Number.MAX_SAFE_INTEGER)
    || left.createdAt - right.createdAt;
};
function formatFileSize(bytes: number | null, locale: string) {
  if (bytes == null) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = bytes > 0 ? Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1) : 0;
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(bytes / 1024 ** exponent)} ${units[exponent]}`;
}
const isTerminal = (task: RagTask) => ["COMPLETED", "FAILED", "CANCELLED"].includes(task.state);

export function RagIngestionTab({ corpora, tasks, selected, onSelect, stateFilter, onSelectState, onChanged, onRefresh, refreshing, error }: { corpora: RagCorpus[]; tasks: RagTask[]; selected: string; onSelect: (id: string) => void; stateFilter: string; onSelectState: (state: string) => void; onChanged: () => void; onRefresh: () => void; refreshing?: boolean; error?: string }) {
  const { t, i18n } = useTranslation("pages");
  const toast = useToast();
  useRagErrorToast(error);
  const [pending, setPending] = useState("");
  const [history, setHistory] = useState(false);
  const [deleteTargets, setDeleteTargets] = useState<RagTask[]>([]);
  const [chunkTask, setChunkTask] = useState<RagTask | null>(null);
  const [preview, setPreview] = useState<RagDocumentPreview | null>(null);
  const [selectedTaskIds, setSelectedTaskIds] = useState<Set<string>>(new Set());
  const corpusTasks = tasks.filter((task) => (!selected || task.corpusId === selected) && (history ? task.isLatest === false : task.isLatest !== false)).sort(compareTasks);
  const statistics = [
    { key: "totalTasks", count: corpusTasks.length },
    { key: "processingTasks", count: corpusTasks.filter((task) => ["RUNNING", "QUEUED", "WAITING"].includes(task.state)).length },
    { key: "failedTasks", count: corpusTasks.filter((task) => task.state === "FAILED").length },
  ];
  const number = new Intl.NumberFormat(i18n.resolvedLanguage);
  const rows = corpusTasks.filter((task) => !stateFilter || task.state === stateFilter);
  const selectableTasks = rows.filter(isTerminal);
  const selectedTasks = selectableTasks.filter((task) => selectedTaskIds.has(task.id));
  const allSelected = selectableTasks.length > 0 && selectedTasks.length === selectableTasks.length;
  const toggleTask = (id: string, checked: boolean) => setSelectedTaskIds((current) => {
    const next = new Set(current);
    if (checked) next.add(id); else next.delete(id);
    return next;
  });
  const act = async (id: string, action: "cancel" | "retry") => {
    setPending(id);
    try { await controlRagTask(id, action); onChanged(); }
    catch (cause) { toast.error(cause instanceof Error ? cause.message : String(cause)); }
    finally { setPending(""); }
  };
  const deleteRecords = async () => {
    if (pending || !deleteTargets.length) return;
    setPending("__delete__");
    const deleted = new Set<string>();
    const failures: string[] = [];
    for (const task of deleteTargets) {
      try { await deleteRagTask(task.id); deleted.add(task.id); }
      catch (cause) { failures.push(`${task.title}: ${cause instanceof Error ? cause.message : String(cause)}`); }
    }
    setSelectedTaskIds((current) => new Set([...current].filter((id) => !deleted.has(id))));
    setDeleteTargets((current) => current.filter((task) => !deleted.has(task.id)));
    setPending(""); onChanged();
    if (failures.length) toast.error(failures.join("\n"));
    else toast.success(t("ragUnified.recordsDeleted", { count: deleted.size }));
  };
  const viewDocument = (task: RagTask, libraryId: string, view: RagDocumentPreview["view"]) => {
    if (task.documentId) setPreview({ libraryId, documentId: task.documentId, title: task.title, view });
  };
  return <div className="grid min-w-0 gap-4">
    <div className="hidden lg:block"><RagFlowSteps label={t("ragUnified.ingestionFlow")} steps={ingestionSteps} /></div>
    <div role="group" aria-label={t("ragUnified.taskStatistics")} className="grid gap-4 sm:grid-cols-3">
      {statistics.map(({ key, count }) => <MetricCard key={key} label={t(`ragUnified.${key}`)} value={number.format(count)} />)}
    </div>
    <div className="flex flex-wrap gap-3">
      <SelectInput aria-label={t("ragUnified.corpus")} className="w-72 max-w-full" disabled={!!pending} value={selected || "__all__"} onValueChange={(value) => { setSelectedTaskIds(new Set()); onSelect(value === "__all__" ? "" : String(value)); }}>
        <SelectItem value="__all__">{t("ragUnified.allCorpora")}</SelectItem>{corpora.map((item) => <SelectItem key={item.id} value={item.id}>{item.name}</SelectItem>)}
      </SelectInput>
      <SelectInput aria-label={t("ragUnified.state")} className="w-48 max-w-full" disabled={!!pending} value={stateFilter || "__all__"} onValueChange={(value) => { setSelectedTaskIds(new Set()); onSelectState(value === "__all__" ? "" : String(value)); }}>
        <SelectItem value="__all__">{t("ragUnified.allStates")}</SelectItem>{taskStates.map((state) => <SelectItem key={state} value={state}>{t(`ragUnified.states.${state}`)}</SelectItem>)}
      </SelectInput>
      <SelectInput aria-label={t("ragUnified.taskView")} className="w-48 max-w-full" disabled={!!pending} value={history ? "history" : "latest"} onValueChange={(value) => { setHistory(value === "history"); setSelectedTaskIds(new Set()); }}>
        <SelectItem value="latest">{t("ragUnified.latestTasks")}</SelectItem><SelectItem value="history">{t("ragUnified.historyTasks")}</SelectItem>
      </SelectInput>
      <Button variant="outline" disabled={selectedTasks.length === 0 || !!pending} onClick={() => setDeleteTargets(selectedTasks)}>{t("ragUnified.deleteRecords")}</Button>
      <Button variant="outline" disabled={!!pending || refreshing} onClick={onRefresh}>{t("ragUnified.refresh")}</Button>
    </div>
    <Card className="min-w-0"><CardContent className="min-w-0 pt-6"><Table className="min-w-[64rem] table-fixed">
      <colgroup>{["4%", "27%", "7%", "12%", "10%", "11%", "6%", "17%", "6%"].map((width, index) => <col key={index} style={{ width }} />)}</colgroup><TableHeader><TableRow><TableHead className="w-12"><input type="checkbox" className="size-4 accent-[var(--color-primary)]" aria-label={t("ragUnified.selectAllTasks")} checked={allSelected} ref={(node) => { if (node) node.indeterminate = selectedTasks.length > 0 && !allSelected; }} disabled={!!pending || selectableTasks.length === 0} onChange={(event) => setSelectedTaskIds(event.target.checked ? new Set(selectableTasks.map((task) => task.id)) : new Set())} /></TableHead>{["document", "fileSize", "corpus", "state", "stage", "chunks", "submittedAt", "actions"].map((key) => <TableHead key={key} className={key === "actions" ? "w-16 text-right" : undefined}>{t(`ragUnified.${key}`)}</TableHead>)}</TableRow></TableHeader><TableBody>
      {rows.map((task) => {
        const taskCorpus = corpora.find((item) => item.id === task.corpusId);
        const libraryId = taskCorpus?.knowledgeBaseId;
        const canCancel = task.isLatest !== false && ["RUNNING", "QUEUED"].includes(task.state);
        const canRetry = task.isLatest !== false && ["FAILED", "CANCELLED"].includes(task.state);
        const canPreview = !!task.documentId && !!libraryId;
        const canInspectBuiltin = ["skill", "memory-card"].includes(task.corpusId) && task.isLatest !== false && task.state === "COMPLETED";
        const canViewChunks = canPreview || canInspectBuiltin;
        const viewChunks = () => { if (canPreview) viewDocument(task, libraryId!, "chunks"); else setChunkTask(task); };
        const isSelected = isTerminal(task) && selectedTaskIds.has(task.id);
        return <TableRow key={task.id} data-state={isSelected ? "selected" : undefined}><TableCell><input type="checkbox" className="size-4 accent-[var(--color-primary)]" aria-label={t("ragUnified.selectTask", { name: task.title })} checked={isSelected} disabled={!!pending || !isTerminal(task)} onChange={(event) => toggleTask(task.id, event.target.checked)} /></TableCell><TableCell className="whitespace-normal [overflow-wrap:anywhere]"><div>{task.title}</div>{task.error ? <details className="mt-1 min-w-0 text-sm text-destructive"><summary className="cursor-pointer">{t("ragUnified.taskErrorDetails")}</summary><p className="mt-2 max-h-48 overflow-y-auto whitespace-pre-wrap [overflow-wrap:anywhere]">{task.error}</p></details> : null}</TableCell><TableCell className="whitespace-nowrap tabular-nums">{formatFileSize(task.fileSizeBytes, i18n.resolvedLanguage ?? i18n.language)}</TableCell><TableCell className="whitespace-normal [overflow-wrap:anywhere]">{taskCorpus?.name ?? task.corpusId}</TableCell><TableCell><Badge variant="secondary" className="max-w-full whitespace-normal text-left">{t(`ragUnified.states.${task.state}`, { defaultValue: task.state })}</Badge></TableCell><TableCell className="whitespace-normal [overflow-wrap:anywhere]">{t(`ragUnified.stages.${task.stage}`, { defaultValue: task.stage })}</TableCell><TableCell>{task.totalUnits > 0 ? canViewChunks ? <Button variant="link" className="h-auto p-0 tabular-nums" aria-label={`${t("ragUnified.viewChunks")} · ${task.title}`} onClick={viewChunks}>{number.format(task.totalUnits)}</Button> : number.format(task.totalUnits) : "—"}</TableCell><TableCell className="whitespace-nowrap tabular-nums">{new Intl.DateTimeFormat(i18n.resolvedLanguage ?? i18n.language, { dateStyle: "short", timeStyle: "medium" }).format(task.createdAt)}</TableCell><TableCell className="text-right">
          <DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon" disabled={!!pending || (!canCancel && !canRetry && !canPreview && !isTerminal(task))} aria-label={t("ragUnified.taskActions", { name: task.title })}><MoreHorizontal size={16} aria-hidden="true" /></Button>} />
            <DropdownMenuContent align="end">
              {canCancel ? <DropdownMenuItem disabled={!!pending} onClick={() => void act(task.id, "cancel")}>{t("ragUnified.cancel")}</DropdownMenuItem> : null}
              {canRetry ? <DropdownMenuItem disabled={!!pending} onClick={() => void act(task.id, "retry")}>{t("ragUnified.retry")}</DropdownMenuItem> : null}
              {canViewChunks ? <DropdownMenuItem onClick={viewChunks}>{t("ragUnified.viewChunks")}</DropdownMenuItem> : null}
              {canPreview ? <DropdownMenuItem onClick={() => viewDocument(task, libraryId!, "original")}>{t("ragUnified.viewOriginal")}</DropdownMenuItem> : null}
              {isTerminal(task) ? <DropdownMenuItem disabled={!!pending} onClick={() => setDeleteTargets([task])}>{t("ragUnified.deleteRecord")}</DropdownMenuItem> : null}
            </DropdownMenuContent>
          </DropdownMenu>
        </TableCell></TableRow>;
      })}
      {rows.length === 0 ? <TableRow><TableCell colSpan={9}>{t(stateFilter || selected ? "ragUnified.emptyFilteredTasks" : "ragUnified.emptyTasks")}</TableCell></TableRow> : null}
    </TableBody></Table></CardContent></Card>
    <ConfirmDialog open={deleteTargets.length > 0} title={t("ragUnified.deleteRecords")} description={t("ragUnified.deleteRecordsHelp", { count: deleteTargets.length })} cancelText={t("ragUnified.cancel")} confirmText={t("ragUnified.deleteRecords")} pending={pending === "__delete__"} onCancel={() => setDeleteTargets([])} onConfirm={() => void deleteRecords()} />
    {chunkTask ? <RagTaskChunksDialog key={chunkTask.id} task={chunkTask} onClose={() => setChunkTask(null)} /> : null}
    {preview ? <RagDocumentPreviewDialog key={`${preview.libraryId}:${preview.documentId}:${preview.view}`} preview={preview} onClose={() => setPreview(null)} /> : null}
  </div>;
}

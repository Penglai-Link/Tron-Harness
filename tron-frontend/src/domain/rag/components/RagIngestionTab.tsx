import { MoreHorizontal, FileText, Scissors, Layers3, CircleCheck } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { Card, CardContent } from "@/components/ui/card";
import { SelectInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Table, TableHeader, TableHead, TableBody, TableCell, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { MetricCard } from "@/shared";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { controlRagTask, rebuildRagRecordTask, submitRagTask } from "../api/ragApi";
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
const canRebuildTask = (task: RagTask) => !["RUNNING", "QUEUED", "WAITING"].includes(task.state);

export function RagIngestionTab({ corpora, tasks, selected, onSelect, stateFilter, onSelectState, onChanged, onRefresh, refreshing, error }: { corpora: RagCorpus[]; tasks: RagTask[]; selected: string; onSelect: (id: string) => void; stateFilter: string; onSelectState: (state: string) => void; onChanged: () => void; onRefresh: () => void; refreshing?: boolean; error?: string }) {
  const { t, i18n } = useTranslation("pages");
  const toast = useToast();
  useRagErrorToast(error);
  const [pending, setPending] = useState("");
  const [preview, setPreview] = useState<RagDocumentPreview | null>(null);
  const [selectedTaskIds, setSelectedTaskIds] = useState<Set<string>>(new Set());
  const corpusTasks = tasks.filter((task) => !selected || task.corpusId === selected).sort(compareTasks);
  const statistics = [
    { key: "totalTasks", count: corpusTasks.length },
    { key: "processingTasks", count: corpusTasks.filter((task) => ["RUNNING", "QUEUED", "WAITING"].includes(task.state)).length },
    { key: "failedTasks", count: corpusTasks.filter((task) => task.state === "FAILED").length },
  ];
  const number = new Intl.NumberFormat(i18n.resolvedLanguage);
  const rows = corpusTasks.filter((task) => !stateFilter || task.state === stateFilter);
  const selectableTasks = rows.filter(canRebuildTask);
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
  const rebuildSelected = async () => {
    if (pending || selectedTasks.length === 0) return;
    setPending("__batch__");
    const failures: string[] = [];
    const accepted = new Set<string>();
    const submittedDocuments = new Set<string>();
    for (const task of selectedTasks) {
      try {
        if (task.documentId) {
          const key = `${task.corpusId}:${task.documentId}`;
          if (!submittedDocuments.has(key)) {
            await submitRagTask(task.corpusId, task.documentId);
            submittedDocuments.add(key);
          }
        } else await rebuildRagRecordTask(task.id);
        accepted.add(task.id);
      } catch (cause) { failures.push(`${task.title}: ${cause instanceof Error ? cause.message : String(cause)}`); }
    }
    setSelectedTaskIds((current) => new Set([...current].filter((id) => !accepted.has(id))));
    if (failures.length) toast.error(failures.join("\n"));
    else toast.success(t("ragUnified.rebuildSubmitted", { count: accepted.size }));
    setPending(""); onChanged();
  };
  const viewDocument = (task: RagTask, libraryId: string, view: RagDocumentPreview["view"]) => {
    if (task.documentId) setPreview({ libraryId, documentId: task.documentId, title: task.title, view });
  };
  return <div className="grid gap-4">
    <RagFlowSteps label={t("ragUnified.ingestionFlow")} steps={ingestionSteps} />
    <div role="group" aria-label={t("ragUnified.taskStatistics")} className="grid gap-4 sm:grid-cols-3">
      {statistics.map(({ key, count }) => <MetricCard key={key} label={t(`ragUnified.${key}`)} value={number.format(count)} />)}
    </div>
    <div className="flex flex-wrap gap-3">
      <SelectInput aria-label={t("ragUnified.corpus")} className="w-72" disabled={!!pending} value={selected || "__all__"} onValueChange={(value) => { setSelectedTaskIds(new Set()); onSelect(value === "__all__" ? "" : String(value)); }}>
        <SelectItem value="__all__">{t("ragUnified.allCorpora")}</SelectItem>{corpora.map((item) => <SelectItem key={item.id} value={item.id}>{item.name}</SelectItem>)}
      </SelectInput>
      <SelectInput aria-label={t("ragUnified.state")} className="w-48" disabled={!!pending} value={stateFilter || "__all__"} onValueChange={(value) => { setSelectedTaskIds(new Set()); onSelectState(value === "__all__" ? "" : String(value)); }}>
        <SelectItem value="__all__">{t("ragUnified.allStates")}</SelectItem>{taskStates.map((state) => <SelectItem key={state} value={state}>{t(`ragUnified.states.${state}`)}</SelectItem>)}
      </SelectInput>
      <Button variant="outline" disabled={selectedTasks.length === 0 || !!pending} onClick={() => void rebuildSelected()}>{t(pending === "__batch__" ? "ragUnified.rebuilding" : "ragUnified.rebuild")}</Button>
      <Button variant="outline" disabled={!!pending || refreshing} onClick={onRefresh}>{t("ragUnified.refresh")}</Button>
    </div>
    <p className="text-sm text-muted-foreground">{t("ragUnified.smallFilesFirst")}</p>
    <Card><CardContent className="pt-6"><Table><TableHeader><TableRow><TableHead className="w-12"><input type="checkbox" className="size-4 accent-[var(--color-primary)]" aria-label={t("ragUnified.selectAllTasks")} checked={allSelected} ref={(node) => { if (node) node.indeterminate = selectedTasks.length > 0 && !allSelected; }} disabled={!!pending || selectableTasks.length === 0} onChange={(event) => setSelectedTaskIds(event.target.checked ? new Set(selectableTasks.map((task) => task.id)) : new Set())} /></TableHead>{["document", "fileSize", "corpus", "state", "stage", "chunks", "actions"].map((key) => <TableHead key={key} className={key === "actions" ? "w-16 text-right" : undefined}>{t(`ragUnified.${key}`)}</TableHead>)}</TableRow></TableHeader><TableBody>
      {rows.map((task) => {
        const taskCorpus = corpora.find((item) => item.id === task.corpusId);
        const libraryId = taskCorpus?.knowledgeBaseId;
        const canCancel = ["RUNNING", "QUEUED"].includes(task.state);
        const canRetry = ["FAILED", "CANCELLED"].includes(task.state);
        const canPreview = !!task.documentId && !!libraryId;
        const isSelected = canRebuildTask(task) && selectedTaskIds.has(task.id);
        return <TableRow key={task.id} data-state={isSelected ? "selected" : undefined}><TableCell><input type="checkbox" className="size-4 accent-[var(--color-primary)]" aria-label={t("ragUnified.selectTask", { name: task.title })} checked={isSelected} disabled={!!pending || !canRebuildTask(task)} onChange={(event) => toggleTask(task.id, event.target.checked)} /></TableCell><TableCell><div>{task.title}</div>{task.error ? <p className="mt-1 max-w-lg whitespace-pre-wrap text-sm text-destructive">{task.error}</p> : null}</TableCell><TableCell className="whitespace-nowrap tabular-nums">{formatFileSize(task.fileSizeBytes, i18n.resolvedLanguage ?? i18n.language)}</TableCell><TableCell>{taskCorpus?.name ?? task.corpusId}</TableCell><TableCell><Badge variant="secondary">{t(`ragUnified.states.${task.state}`, { defaultValue: task.state })}</Badge></TableCell><TableCell>{t(`ragUnified.stages.${task.stage}`, { defaultValue: task.stage })}</TableCell><TableCell>{task.totalUnits > 0 ? number.format(task.totalUnits) : "—"}</TableCell><TableCell className="text-right">
          <DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon" disabled={!canCancel && !canRetry && !canPreview} aria-label={t("ragUnified.taskActions", { name: task.title })}><MoreHorizontal size={16} aria-hidden="true" /></Button>} />
            <DropdownMenuContent align="end">
              {canCancel ? <DropdownMenuItem disabled={!!pending} onClick={() => void act(task.id, "cancel")}>{t("ragUnified.cancel")}</DropdownMenuItem> : null}
              {canRetry ? <DropdownMenuItem disabled={!!pending} onClick={() => void act(task.id, "retry")}>{t("ragUnified.retry")}</DropdownMenuItem> : null}
              {canPreview ? <DropdownMenuItem onClick={() => viewDocument(task, libraryId!, "chunks")}>{t("ragUnified.viewChunks")}</DropdownMenuItem> : null}
              {canPreview ? <DropdownMenuItem onClick={() => viewDocument(task, libraryId!, "original")}>{t("ragUnified.viewOriginal")}</DropdownMenuItem> : null}
            </DropdownMenuContent>
          </DropdownMenu>
        </TableCell></TableRow>;
      })}
      {rows.length === 0 ? <TableRow><TableCell colSpan={8}>{t(stateFilter || selected ? "ragUnified.emptyFilteredTasks" : "ragUnified.emptyTasks")}</TableCell></TableRow> : null}
    </TableBody></Table></CardContent></Card>
    {preview ? <RagDocumentPreviewDialog key={`${preview.libraryId}:${preview.documentId}:${preview.view}`} preview={preview} onClose={() => setPreview(null)} /> : null}
  </div>;
}

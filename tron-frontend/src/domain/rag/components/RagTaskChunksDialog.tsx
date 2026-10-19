import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { TablePagination } from "@/components/table-pagination";
import { useKnowledgeUser } from "@/domain/knowledge-base/hooks/useKnowledgeLibraries";
import { formatDocumentLocation } from "@/domain/knowledge-base/model/documentLocation";
import { getRagTaskChunks } from "../api/ragApi";
import type { RagTask } from "../model/RagModel";

export function RagTaskChunksDialog({ task, onClose }: { task: RagTask; onClose: () => void }) {
  const { t, i18n } = useTranslation("pages");
  const user = useKnowledgeUser();
  const [current, setCurrent] = useState(1);
  const [size, setSize] = useState(15);
  const query = useQuery({ queryKey: ["rag-task-chunks", user, task.id, task.updatedAt, current, size], queryFn: ({ signal }) => getRagTaskChunks(task.id, current, size, signal), retry: false });
  const total = query.data?.total ?? 0;
  const pages = Math.max(1, Math.ceil(total / size));
  return <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
    <DialogContent aria-describedby={undefined} closeLabel={t("ragUnified.close")} className="sm:max-w-4xl max-h-[90vh] overflow-y-auto">
      <DialogHeader><DialogTitle>{t("ragUnified.viewChunks")} · {task.title}</DialogTitle></DialogHeader>
      {query.isLoading ? <p role="status">{t("ragUnified.loading")}</p> : query.error ? <div className="grid gap-3"><p role="alert" className="text-destructive">{query.error.message}</p><Button variant="outline" onClick={() => void query.refetch()}>{t("ragUnified.retry")}</Button></div> : <>
        <div className="max-h-[60vh] overflow-auto"><Table><TableHeader><TableRow><TableHead className="w-36">{t("knowledgeLibrary.chunkPosition")}</TableHead><TableHead>{t("knowledgeLibrary.chunkContent")}</TableHead></TableRow></TableHeader><TableBody>
          {query.data?.records.map((chunk) => <TableRow key={chunk.textUnitCode}><TableCell className="align-top whitespace-normal"><p>{t("knowledgeLibrary.chunkNumber", { number: chunk.ordinal + 1 })}</p>{chunk.headingPath.length ? <p className="mt-1 text-xs text-muted-foreground">{chunk.headingPath.join(" / ")}</p> : null}{chunk.metadata.location ? <p className="mt-1 text-xs text-muted-foreground">{formatDocumentLocation(chunk.metadata.location, t, i18n.resolvedLanguage)}</p> : null}</TableCell><TableCell className="align-top"><pre className="whitespace-pre-wrap break-words font-sans text-sm">{chunk.text}</pre></TableCell></TableRow>)}
          {!total ? <TableRow><TableCell colSpan={2} className="py-8 text-center">{t("knowledgeLibrary.noPublishedChunks")}</TableCell></TableRow> : null}
        </TableBody></Table></div>
        <TablePagination itemLabel={t("knowledgeLibrary.chunkItems")} page={current} pageSize={size} pageCount={pages} totalItems={total} startItem={total ? (current - 1) * size + 1 : 0} endItem={Math.min(current * size, total)} onPageChange={(value) => setCurrent(Math.min(pages, Math.max(1, value)))} onPageSizeChange={(value) => { setSize(value); setCurrent(1); }} />
      </>}
    </DialogContent>
  </Dialog>;
}

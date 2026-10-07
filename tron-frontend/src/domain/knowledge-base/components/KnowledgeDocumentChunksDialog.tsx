import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { TablePagination } from "@/components/table-pagination";
import { getPublishedKnowledgeChunks } from "../api/knowledgeBaseApi";
import { formatDocumentLocation } from "../model/documentLocation";
import { useKnowledgeUser } from "../hooks/useKnowledgeLibraries";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";

export function KnowledgeDocumentChunksDialog({ workspace, libraryId, document, onClose }: {
  workspace: string; libraryId: string; document: KnowledgeDocument; onClose: () => void;
}) {
  const { t, i18n } = useTranslation("pages");
  const user = useKnowledgeUser();
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(15);
  const query = useQuery({
    queryKey: ["kb-published-chunks", user, workspace, libraryId, document.id, document.indexRevision, page, pageSize],
    queryFn: ({ signal }) => getPublishedKnowledgeChunks(workspace, libraryId, document.id, page, pageSize, signal), retry: false,
  });
  const total = query.data?.total ?? 0;
  const pages = Math.max(1, Math.ceil(total / pageSize));
  return <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
    <DialogContent aria-describedby={undefined} closeLabel={text("close")} className="sm:max-w-4xl max-h-[90vh] overflow-y-auto">
      <DialogHeader><DialogTitle>{text("viewChunks")} · {document.fileName}</DialogTitle></DialogHeader>
      {query.isLoading ? <p role="status">{text("loading")}</p> : query.error ? <p role="alert" className="text-[var(--color-error)]">{query.error.message}</p> : <>
        <div className="max-h-[60vh] overflow-auto"><Table><TableHeader><TableRow><TableHead className="w-36">{text("chunkPosition")}</TableHead><TableHead>{text("chunkContent")}</TableHead></TableRow></TableHeader><TableBody>
          {query.data?.items.map((chunk) => <TableRow key={chunk.textUnitCode}>
            <TableCell className="align-top whitespace-normal"><p>{t("knowledgeLibrary.chunkNumber", { number: chunk.ordinal + 1 })}</p>{chunk.metadata.location ? <p className="mt-1 text-xs text-[var(--color-on-surface-variant)]">{formatDocumentLocation(chunk.metadata.location, t, i18n.resolvedLanguage)}</p> : null}</TableCell>
            <TableCell className="align-top"><pre className="whitespace-pre-wrap break-words font-sans text-sm">{chunk.text}</pre></TableCell>
          </TableRow>)}
          {!total ? <TableRow><TableCell colSpan={2} className="py-8 text-center">{text("noPublishedChunks")}</TableCell></TableRow> : null}
        </TableBody></Table></div>
        <TablePagination itemLabel={text("chunkItems")} page={page} pageSize={pageSize} pageCount={pages} totalItems={total} startItem={total ? (page - 1) * pageSize + 1 : 0} endItem={Math.min(page * pageSize, total)} onPageChange={(value) => setPage(Math.min(pages, Math.max(1, value)))} onPageSizeChange={(value) => { setPageSize(value); setPage(1); }} />
      </>}
    </DialogContent>
  </Dialog>;
}

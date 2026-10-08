import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { getKnowledgeLibrary } from "@/domain/knowledge-base/api/knowledgeBaseApi";
import { KnowledgeDocumentChunksDialog } from "@/domain/knowledge-base/components/KnowledgeDocumentChunksDialog";
import { KnowledgeDocumentResourcesDialog } from "@/domain/knowledge-base/components/KnowledgeDocumentResourcesDialog";
import { useKnowledgeUser } from "@/domain/knowledge-base/hooks/useKnowledgeLibraries";
import { useRagErrorToast } from "../hooks/useRagErrorToast";

export interface RagDocumentPreview {
  libraryId: string;
  documentId: string;
  title: string;
  view: "chunks" | "original";
}

export function RagDocumentPreviewDialog({ preview, onClose }: {
  preview: RagDocumentPreview;
  onClose: () => void;
}) {
  const { t } = useTranslation("pages");
  const user = useKnowledgeUser();
  const query = useQuery({
    queryKey: ["knowledge-base", user, preview.libraryId],
    queryFn: ({ signal }) => getKnowledgeLibrary(preview.libraryId, signal),
    retry: false,
  });
  const document = query.data?.documents.find((item) => item.id === preview.documentId);
  useRagErrorToast(query.error?.message ?? (!query.isFetching && !document ? t("ragUnified.documentUnavailable") : undefined));

  if (!query.isFetching && !query.error && document) {
    return preview.view === "chunks"
      ? <KnowledgeDocumentChunksDialog libraryId={preview.libraryId} document={document} onClose={onClose} />
      : <KnowledgeDocumentResourcesDialog libraryId={preview.libraryId} document={document} onClose={onClose} />;
  }

  return <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
    <DialogContent aria-describedby={undefined} closeLabel={t("ragUnified.close")}>
      <DialogHeader><DialogTitle>{t(`ragUnified.${preview.view === "chunks" ? "viewChunks" : "viewOriginal"}`)} · {preview.title}</DialogTitle></DialogHeader>
      {query.isFetching ? <p role="status">{t("ragUnified.loading")}</p> : <>
        <Button variant="outline" onClick={() => void query.refetch()}>{t("ragUnified.retry")}</Button>
      </>}
    </DialogContent>
  </Dialog>;
}

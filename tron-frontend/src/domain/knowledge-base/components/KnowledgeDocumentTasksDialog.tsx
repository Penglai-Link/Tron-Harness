import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { getKnowledgeDocumentTasks } from "../api/knowledgeBaseApi";
import { useKnowledgeUser } from "../hooks/useKnowledgeLibraries";
import { formatIngestionWaitReason } from "../model/ingestionWaitReason";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";

export function KnowledgeDocumentTasksDialog({ libraryId, document, onClose }: {
  libraryId: string; document: KnowledgeDocument; onClose: () => void;
}) {
  const { t } = useTranslation("pages");
  const user = useKnowledgeUser();
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const query = useQuery({
    queryKey: ["kb-document-tasks", user, libraryId, document.id],
    queryFn: ({ signal }) => getKnowledgeDocumentTasks(libraryId, document.id, signal),
    refetchInterval: 5000,
    retry: false,
  });
  return <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
    <DialogContent aria-describedby={undefined} closeLabel={text("close")} className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
      <DialogHeader><DialogTitle>{text("taskHistory")} · {document.fileName}</DialogTitle></DialogHeader>
      {query.isLoading ? <p role="status">{text("loading")}</p> : query.error ? <p role="alert" className="text-[var(--color-error)]">{query.error.message}</p> :
        <div className="space-y-3">
          {query.data?.map((task) => <div key={task.id} className="rounded-lg border border-[var(--color-outline-variant)] p-3 text-sm">
            <div className="flex items-center justify-between gap-3">
              <strong>{text(task.kind === "PARSING" ? "parsingTask" : "indexingTask")}</strong>
              <span>{task.state}</span>
            </div>
            <p className="mt-1 text-[var(--color-on-surface-variant)]">{text("taskStage")}: {task.stage} · {text("taskAttempts")}: {task.attempt}</p>
            {task.error ? <p className="mt-2 text-[var(--color-error)]">{task.error}</p> : null}
            {task.waitReason ? <p className="mt-2">{formatIngestionWaitReason(task.waitReason, t)}</p> : null}
            {task.cancellationPending ? <p className="mt-2">{text("cancellationPending")}</p> : null}
          </div>)}
          {!query.data?.length ? <p>{text("noTasks")}</p> : null}
        </div>}
    </DialogContent>
  </Dialog>;
}

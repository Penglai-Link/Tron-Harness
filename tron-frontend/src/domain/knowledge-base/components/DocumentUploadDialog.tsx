import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { preflightKnowledgeDocument } from "../api/knowledgeBaseApi";
import { useKnowledgeUser } from "../hooks/useKnowledgeLibraries";
import { DocumentParsingPreferenceField } from "./DocumentParsingPreferenceField";
import type { DocumentParsePlan, DocumentParsePreference } from "../model/KnowledgeLibraryModel";

export interface PreparedDocumentUpload { file: File; plan: DocumentParsePlan }
interface InspectedDocumentUpload { file: File; plan: DocumentParsePlan | null; error: string | null }
export function DocumentUploadDialog({ libraryId, files, defaultPreference, pending, onUpload, onClose }: {
  libraryId: string; files: File[]; defaultPreference: DocumentParsePreference; pending: boolean;
  onUpload: (items: PreparedDocumentUpload[]) => void; onClose: () => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const user = useKnowledgeUser();
  const [preference, setPreference] = useState(defaultPreference);
  const [session] = useState(() => Array.from(crypto.getRandomValues(new Uint8Array(12))).join("-"));
  const inspection = useQuery({
    queryKey: ["knowledge-document-preflight", user, libraryId, session, preference],
    queryFn: async ({ signal }) => {
      const prepared: InspectedDocumentUpload[] = [];
      // Let a cancelled StrictMode mount release its query before sending a file.
      await Promise.resolve();
      // The server bounds concurrent inspection; a batch checks files sequentially.
      for (const file of files) {
        if (signal.aborted) throw new Error("Aborted");
        try {
          const result = await preflightKnowledgeDocument(libraryId, file, preference, signal);
          prepared.push({ file, plan: result.plan, error: null });
        } catch (error) {
          if (signal.aborted) throw error;
          prepared.push({ file, plan: null, error: error instanceof Error ? error.message : text("inspectionFailed") });
        }
      }
      return prepared;
    },
    retry: false, staleTime: Infinity, gcTime: 0, refetchOnWindowFocus: false,
  });
  const ready = inspection.data?.flatMap((item) => item.plan ? [{ file: item.file, plan: item.plan }] : []) ?? [];
  return <Dialog open onOpenChange={(open) => { if (!open && !pending) onClose(); }}>
    <DialogContent className="max-h-[90dvh] overflow-y-auto sm:max-w-xl" closeLabel={text("cancel")}>
      <DialogHeader><DialogTitle>{text("upload")}</DialogTitle></DialogHeader>
      <p className="text-sm">{text("uploadParsingHelp")}</p>
      <ul className="divide-y">
        {files.map((file, index) => {
          const item = inspection.data?.[index]; const plan = item?.plan;
          return <li key={`${file.name}:${index}`} className="grid min-w-0 gap-2 py-3 text-sm"><div className="flex min-w-0 items-start justify-between gap-4"><span className="min-w-0 break-all">{file.name}</span><span className="shrink-0 text-muted-foreground">{plan ? text(plan.features.localText ? "directRead" : `parsingPreferences.${plan.method}`) : text(item?.error || inspection.error ? "inspectionFailed" : "inspecting")}</span></div>{item?.error ? <p role="alert" className="text-[var(--color-error)]">{item.error}</p> : null}</li>;
        })}
      </ul>
      {inspection.error ? <div className="grid gap-2"><p role="alert" className="text-sm text-[var(--color-error)]">{inspection.error.message}</p><Button variant="outline" disabled={inspection.isFetching} onClick={() => void inspection.refetch()}>{text("refresh")}</Button></div> : null}
      {inspection.data?.some((item) => item.error) ? <Button variant="outline" disabled={pending || inspection.isFetching} onClick={() => void inspection.refetch()}>{text("refresh")}</Button> : null}
      <details className="grid gap-3"><summary className="cursor-pointer text-sm">{text("parsingAdvanced")}</summary><div className="pt-3"><DocumentParsingPreferenceField value={preference} onChange={setPreference} disabled={pending || inspection.isFetching} /></div></details>
      <DialogFooter><Button variant="outline" disabled={pending} onClick={onClose}>{text("cancel")}</Button><Button disabled={pending || inspection.isFetching || !ready.length} onClick={() => onUpload(ready)}>{text(pending ? "uploading" : "upload")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}

import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { KnowledgeDocumentChunksDialog } from "../components/KnowledgeDocumentChunksDialog";
import { MoreHorizontal } from "lucide-react";
import { PageHeader, SurfaceCard } from "@/shared";
import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useToast } from "@/components/ui/toast";
import { InfoTooltip } from "@/components/info-tooltip";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { downloadKnowledgeResource, cancelKnowledgeDocument, deleteKnowledgeDocument, getParsingCapabilities, indexKnowledgeDocument, uploadKnowledgeDocument } from "../api/knowledgeBaseApi";
import { useKnowledgeLibrary, useKnowledgeUser, useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";
import { formatIngestionWaitReason } from "../model/ingestionWaitReason";
import { formatKnowledgeDocumentError } from "../model/knowledgeDocumentError";
import { DocumentSettingsDialog } from "../components/DocumentSettingsDialog";
import { KnowledgeDocumentResourcesDialog } from "../components/KnowledgeDocumentResourcesDialog";
import { KnowledgeDocumentName } from "../components/KnowledgeDocumentName";
import { KnowledgeGrantsDialog } from "../components/KnowledgeGrantsDialog";

export function KnowledgeBaseDetailPage() {
  const user = useKnowledgeUser();
  const { knowledgeBaseId = "" } = useParams();
  return <KnowledgeDetail key={`${user}:${knowledgeBaseId}`} id={knowledgeBaseId} />;
}

function KnowledgeDetail({ id }: { id: string }) {
  const { t, i18n } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const toast = useToast();
  const active = useKnowledgeViewActive();
  const query = useKnowledgeLibrary(id);
  const cache = useQueryClient();
  const user = useKnowledgeUser();
  const refreshLibrary = async () => {
    await cache.invalidateQueries({ queryKey: ["platform-knowledge-bases", user] });
    await cache.invalidateQueries({ queryKey: ["knowledge-bases", user] });
    return query.refetch();
  };
  const library = query.data;
  const capabilities = useQuery({
    queryKey: ["kb-parsing-capabilities", user],
    queryFn: ({ signal }) => getParsingCapabilities( signal),
    enabled: Boolean(library?.documents.some((doc) => /\.pdf$/i.test(doc.fileName) && !doc.parsingTier && !doc.parsingArtifact)),
    retry: false,
  });
  const [selected, setSelected] = useState<KnowledgeDocument | null>(null);
  const [agentGrants, setAgentGrants] = useState(false);
  const [chunksDocument, setChunksDocument] = useState<KnowledgeDocument | null>(null);
  const [resources, setResources] = useState<KnowledgeDocument | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<KnowledgeDocument | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const upload = useMutation({
    mutationFn: async (files: File[]) => {
      let uploaded = 0;
      const failures: string[] = [];
      for (const file of files) {
        if (!active.current) break;
        try {
          if (!/\.(md|docx|xlsx|pdf)$/i.test(file.name)) throw new Error(text("unsupported"));
          if (file.size > 20 * 1024 * 1024) throw new Error(text("tooLarge"));
          const result = await uploadKnowledgeDocument( id, file);
          if (result?.status?.includes("FAILED") || result?.status === "REVIEW_REQUIRED") {
            throw new Error(result.waitReason ? formatIngestionWaitReason(result.waitReason, t) : formatKnowledgeDocumentError(result.error, t) || text("indexFailed"));
          }
          uploaded += 1;
        } catch (reason) {
          failures.push(`${file.name}: ${formatKnowledgeDocumentError(reason instanceof Error ? reason.message : text("uploadFailed"), t)}`);
        }
      }
      return { uploaded, failures };
    },
    onSuccess: async ({ uploaded, failures }) => {
      await cache.invalidateQueries({ queryKey: ["platform-knowledge-bases", user] });
    await cache.invalidateQueries({ queryKey: ["knowledge-bases", user] });
      if (!active.current) return;
      await query.refetch();
      if (!active.current) return;
      if (uploaded) toast.success(t("knowledgeLibrary.batchUploaded", { count: uploaded }));
      if (failures.length) toast.error(`${t("knowledgeLibrary.batchUploadFailed", { count: failures.length })}\n${failures.join("\n")}`);
    },
  });
  const deletion = useMutation({
    mutationFn: (doc: KnowledgeDocument) => deleteKnowledgeDocument( id, doc.id),
    onSuccess: async () => {
      if (!active.current) return;
      setDeleteTarget(null);
      toast.success(text("documentDeleted"));
      await refreshLibrary();
    },
    onError: async (error) => { if (!active.current) return; toast.error(formatKnowledgeDocumentError(error.message, t)); await query.refetch(); },
  });
  const operation = useMutation({
    mutationFn: async (action: { type: "retry" | "cancel" | "download"; document: KnowledgeDocument }) => {
      if (action.type === "download") {
        const response = await downloadKnowledgeResource(id, action.document.resourceCode);
        if (!active.current) return null;
        const url = URL.createObjectURL(response.blob);
        const anchor = document.createElement("a"); anchor.href = url; anchor.download = action.document.fileName;
        document.body.appendChild(anchor); anchor.click(); anchor.remove();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        return null;
      }
      if (action.type === "cancel") return cancelKnowledgeDocument( id, action.document.id);
      return indexKnowledgeDocument( id, action.document.id, true);
    },
    onSuccess: async (result, action) => {
      if (active.current && action.type !== "download") {
        await refreshLibrary();
        if (!active.current) return;
        if (result?.status?.includes("FAILED") || result?.status === "REVIEW_REQUIRED") toast.error(result.waitReason ? formatIngestionWaitReason(result.waitReason, t) : formatKnowledgeDocumentError(result.error, t) || text("indexFailed"));
        else if (result?.status === "CANCELLED") toast.success(text("cancelled"));
        else if (result && inProgress(result)) toast.success(text("processingStarted"));
        else toast.success(text("indexed"));
      }
    },
    onError: async (error) => { if (!active.current) return; toast.error(formatKnowledgeDocumentError(error.message, t)); await query.refetch(); },
  });
  const number = (value: number) => new Intl.NumberFormat(i18n.resolvedLanguage).format(value);
  const inProgress = (doc: KnowledgeDocument) => ["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(doc.status);
  const busy = upload.isPending || operation.isPending || deletion.isPending;
  const parsingQuality = (doc: KnowledgeDocument) => {
    if (/\.(md|docx|xlsx)$/i.test(doc.fileName)) return text("parsingTiers.flash");
    const tier = doc.parsingArtifact?.effectiveTier ?? doc.parsingTier ?? capabilities.data?.defaultTier;
    return tier ? text(`parsingTiers.${tier}`) : text("inheritShort");
  };
  const statusLabel = (doc: KnowledgeDocument) => {
    // A finished task keeps its last stage for diagnostics, not current progress.
    if (doc.status === "QUEUED" && doc.stage) {
      return t(`knowledgeLibrary.queuedStages.${doc.stage}`, { defaultValue: text("statuses.QUEUED") });
    }
    return t(`knowledgeLibrary.statuses.${doc.status.includes("FAILED") || doc.status === "REVIEW_REQUIRED" ? "FAILED" : doc.status}`, { defaultValue: doc.status });
  };

  return <>
    <PageHeader title={library?.name ?? text("title")} breadcrumbs={[{ label: text("title"), href: "/settings/knowledge-base" }, { label: library?.name ?? text("loading") }]} actions={<>
      <Button variant="outline" disabled={!library || busy} onClick={() => setAgentGrants(true)}>{text("grants")}</Button>
      <Button variant="outline" disabled={query.isFetching} onClick={() => void query.refetch()}>{text("refresh")}</Button>
      <Button disabled={!library || busy} onClick={() => input.current?.click()}>{text(upload.isPending ? "uploading" : "upload")}</Button>
    </>} />
    <Input ref={input} type="file" multiple className="sr-only" tabIndex={-1} aria-label={text("upload")} accept=".md,.docx,.xlsx,.pdf" disabled={!library || busy} onChange={(event) => {
      const files = Array.from(event.target.files ?? []); event.target.value = "";
      if (files.length) upload.mutate(files);
    }} />
    {query.isLoading ? <p role="status">{text("loading")}</p> : query.error ? <p role="alert" className="text-[var(--color-error)]">{query.error.message}</p> : library ? <SurfaceCard>
      {library.description ? <p className="mb-4 text-sm">{library.description}</p> : null}
      <Table><TableHeader><TableRow><TableHead><span className="inline-flex items-center gap-1">{text("document")}<InfoTooltip ariaLabel={text("documentHelp")}><div className="max-w-sm space-y-2 text-left"><p>{text("formats")}</p><p>{text("deletionHelp")}</p></div></InfoTooltip></span></TableHead><TableHead>{text("parsingQuality")}</TableHead><TableHead>{text("mode")}</TableHead><TableHead>{text("status")}</TableHead><TableHead>{text("chunks")}</TableHead><TableHead className="w-16 text-right">{text("actions")}</TableHead></TableRow></TableHeader><TableBody>
        {library.documents.map((doc) => <TableRow key={doc.id}>
          <TableCell className="max-w-xs"><KnowledgeDocumentName document={doc} /></TableCell>
          <TableCell>{parsingQuality(doc)}</TableCell>
          <TableCell>{text(`modes.${(doc.chunkConfig ?? library.defaultChunkConfig).mode}`)}</TableCell>
          <TableCell title={doc.waitReason ? formatIngestionWaitReason(doc.waitReason, t) : undefined}>
            <p className="whitespace-nowrap text-sm">{statusLabel(doc)}</p>
          </TableCell>
          <TableCell>{number(doc.chunkCount)}</TableCell>
          <TableCell className="text-right"><DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon" disabled={busy} aria-label={t("knowledgeLibrary.documentActions", { name: doc.fileName })}><MoreHorizontal size={16} aria-hidden="true" /></Button>} />
            <DropdownMenuContent align="end">
              {inProgress(doc) ? <Tooltip><TooltipTrigger render={<span className="block cursor-not-allowed" />}><DropdownMenuItem disabled className="data-disabled:opacity-35">{text("settings")}</DropdownMenuItem></TooltipTrigger><TooltipContent side="left">{text("settingsDisabledProcessing")}</TooltipContent></Tooltip> : <DropdownMenuItem onClick={() => setSelected(doc)}>{text("settings")}</DropdownMenuItem>}
              <DropdownMenuItem onClick={() => operation.mutate({ type: "download", document: doc })}>{text("downloadAction")}</DropdownMenuItem>
              <DropdownMenuItem onClick={() => setResources(doc)}>{text("previewAction")}</DropdownMenuItem>
              <DropdownMenuItem onClick={() => setChunksDocument(doc)}>{text("viewChunks")}</DropdownMenuItem>
              {(doc.status.includes("FAILED") || doc.status === "REVIEW_REQUIRED" || doc.status === "CANCELLED") ? <DropdownMenuItem onClick={() => operation.mutate({ type: "retry", document: doc })}>{text("retry")}</DropdownMenuItem> : null}
              <DropdownMenuItem variant="destructive" onClick={() => setDeleteTarget(doc)}>{text("deleteAction")}</DropdownMenuItem>
              {inProgress(doc) ? <DropdownMenuItem onClick={() => operation.mutate({ type: "cancel", document: doc })}>{text("cancel")}</DropdownMenuItem> : null}
            </DropdownMenuContent>
          </DropdownMenu></TableCell>
        </TableRow>)}
      </TableBody></Table>
      {!library.documents.length ? <p className="py-8 text-center text-[var(--color-on-surface-variant)]">{text("emptyDocuments")}</p> : null}
    </SurfaceCard> : null}
    {upload.isPending ? <p role="status" className="mt-4 text-sm">{text("uploading")}</p> : null}
    {operation.isPending ? <p role="status" className="mt-4 text-sm">{text(operation.variables?.type === "download" ? "downloading" : "indexing")}</p> : null}
    <ConfirmDialog open={Boolean(deleteTarget)} title={text("deleteTitle")} description={t("knowledgeLibrary.deleteDescription", { file: deleteTarget?.fileName ?? "" })} cancelText={text("cancelDelete")} confirmText={text("deleteDocument")} pending={deletion.isPending} onCancel={() => setDeleteTarget(null)} onConfirm={() => { if (deleteTarget) deletion.mutate(deleteTarget); }} />
    {selected && library ? <DocumentSettingsDialog libraryId={id} document={selected} defaults={library.defaultChunkConfig} onClose={() => setSelected(null)} onChanged={() => query.refetch()} /> : null}
    {chunksDocument ? <KnowledgeDocumentChunksDialog libraryId={id} document={chunksDocument} onClose={() => setChunksDocument(null)} /> : null}
    {resources ? <KnowledgeDocumentResourcesDialog libraryId={id} document={resources} onClose={() => setResources(null)} /> : null}
    {agentGrants && library ? <KnowledgeGrantsDialog libraryId={id} onClose={() => setAgentGrants(false)} onChanged={refreshLibrary} /> : null}
  </>;
}

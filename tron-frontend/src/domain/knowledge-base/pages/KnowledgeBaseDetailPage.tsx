import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router";
import { useMutation } from "@tanstack/react-query";
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
import { downloadResourceBinary } from "@/domain/resource/api/resourceApi";
import { cancelKnowledgeDocument, deleteKnowledgeDocument, indexKnowledgeDocument, uploadKnowledgeDocument } from "../api/knowledgeBaseApi";
import { useKnowledgeLibrary, useKnowledgeWorkspace, useKnowledgeUser, useKnowledgeViewActive } from "../hooks/useKnowledgeLibraries";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";
import { formatIngestionWaitReason } from "../model/ingestionWaitReason";
import { formatKnowledgeDocumentError } from "../model/knowledgeDocumentError";
import { DocumentSettingsDialog } from "../components/DocumentSettingsDialog";
import { KnowledgeDocumentResourcesDialog } from "../components/KnowledgeDocumentResourcesDialog";
import { ImageAnalysisStatus } from "../components/ImageAnalysisStatus";
import { KnowledgeGrantsDialog } from "../components/KnowledgeGrantsDialog";

export function KnowledgeBaseDetailPage() {
  const workspace = useKnowledgeWorkspace();
  const user = useKnowledgeUser();
  const { knowledgeBaseId = "" } = useParams();
  return <KnowledgeDetail key={`${user}:${workspace}:${knowledgeBaseId}`} workspace={workspace} id={knowledgeBaseId} />;
}

function KnowledgeDetail({ workspace, id }: { workspace: string; id: string }) {
  const { t, i18n } = useTranslation("pages");
  const text = (key: string) => t(`knowledgeLibrary.${key}`);
  const toast = useToast();
  const active = useKnowledgeViewActive();
  const query = useKnowledgeLibrary(workspace, id);
  const library = query.data;
  const [selected, setSelected] = useState<KnowledgeDocument | null>(null);
  const [grants, setGrants] = useState(false);
  const [chunksDocument, setChunksDocument] = useState<KnowledgeDocument | null>(null);
  const [resources, setResources] = useState<KnowledgeDocument | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<KnowledgeDocument | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const deletion = useMutation({
    mutationFn: (doc: KnowledgeDocument) => deleteKnowledgeDocument(workspace, id, doc.id),
    onSuccess: async () => {
      if (!active.current) return;
      setDeleteTarget(null);
      toast.success(text("documentDeleted"));
      await query.refetch();
    },
    onError: async (error) => { if (!active.current) return; toast.error(formatKnowledgeDocumentError(error.message, t)); await query.refetch(); },
  });
  const operation = useMutation({
    mutationFn: async (action: { type: "upload"; file: File } | { type: "retry" | "cancel" | "download"; document: KnowledgeDocument }) => {
      if (action.type === "upload") return uploadKnowledgeDocument(workspace, id, action.file);
      if (action.type === "download") {
        const response = await downloadResourceBinary({ workspaceCode: workspace, resourceCode: action.document.resourceCode });
        if (!active.current) return null;
        const url = URL.createObjectURL(response.blob);
        const anchor = document.createElement("a"); anchor.href = url; anchor.download = action.document.fileName;
        document.body.appendChild(anchor); anchor.click(); anchor.remove();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        return null;
      }
      if (action.type === "cancel") return cancelKnowledgeDocument(workspace, id, action.document.id);
      return indexKnowledgeDocument(workspace, id, action.document.id, true);
    },
    onSuccess: async (result, action) => {
      if (active.current && action.type !== "download") {
        await query.refetch();
        if (!active.current) return;
        if (result?.status?.includes("FAILED") || result?.status === "REVIEW_REQUIRED") toast.error(result.waitReason ? formatIngestionWaitReason(result.waitReason, t) : formatKnowledgeDocumentError(result.error, t) || text("indexFailed"));
        else if (result?.status === "CANCELLED") toast.success(text("cancelled"));
        else if (result && inProgress(result)) toast.success(text("processingStarted"));
        else toast.success(text(action.type === "upload" ? "uploaded" : "indexed"));
      }
    },
    onError: async (error) => { if (!active.current) return; toast.error(formatKnowledgeDocumentError(error.message, t)); await query.refetch(); },
  });
  const number = (value: number) => new Intl.NumberFormat(i18n.resolvedLanguage).format(value);
  const inProgress = (doc: KnowledgeDocument) => ["QUEUED", "PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(doc.status);
  const busy = operation.isPending || deletion.isPending;
  const statusLabel = (doc: KnowledgeDocument) => ["PARSING", "CHUNKING", "EMBEDDING", "PUBLISHING"].includes(doc.status)
    ? text("processingStatus") : t(`knowledgeLibrary.statuses.${doc.status.includes("FAILED") || doc.status === "REVIEW_REQUIRED" ? "FAILED" : doc.status}`, { defaultValue: doc.status });
  const documentError = (doc: KnowledgeDocument) => doc.waitReason ? formatIngestionWaitReason(doc.waitReason, t) : formatKnowledgeDocumentError(doc.error, t);

  return <>
    <PageHeader title={library?.name ?? text("title")} breadcrumbs={[{ label: text("title"), href: "/knowledge-base" }, { label: library?.name ?? text("loading") }]} actions={<>
      <Button variant="outline" disabled={!library || busy} onClick={() => setGrants(true)}>{text("grants")}</Button>
      <Button variant="outline" disabled={!workspace || query.isFetching} onClick={() => void query.refetch()}>{text("refresh")}</Button>
      <Button disabled={!library || busy} onClick={() => input.current?.click()}>{text(operation.isPending && operation.variables?.type === "upload" ? "uploading" : "upload")}</Button>
    </>} />
    <Input ref={input} type="file" className="sr-only" tabIndex={-1} aria-label={text("upload")} accept=".md,.docx,.xlsx,.pdf" disabled={!library || busy} onChange={(event) => {
      const file = event.target.files?.[0]; event.target.value = "";
      if (!file) return;
      if (!/\.(md|docx|xlsx|pdf)$/i.test(file.name)) { toast.error(text("unsupported")); return; }
      if (file.size > 20 * 1024 * 1024) { toast.error(text("tooLarge")); return; }
      operation.mutate({ type: "upload", file });
    }} />
    {!workspace ? <p role="status">{text("selectWorkspace")}</p> : query.isLoading ? <p role="status">{text("loading")}</p> : query.error ? <p role="alert" className="text-[var(--color-error)]">{query.error.message}</p> : library ? <SurfaceCard>
      {library.description ? <p className="mb-4 text-sm">{library.description}</p> : null}
      <Table><TableHeader><TableRow><TableHead><span className="inline-flex items-center gap-1">{text("document")}<InfoTooltip ariaLabel={text("documentHelp")}><div className="max-w-sm space-y-2 text-left"><p>{text("formats")}</p><p>{text("deletionHelp")}</p></div></InfoTooltip></span></TableHead><TableHead>{text("mode")}</TableHead><TableHead>{text("status")}</TableHead><TableHead>{text("chunks")}</TableHead><TableHead className="w-16 text-right">{text("actions")}</TableHead></TableRow></TableHeader><TableBody>
        {library.documents.map((doc) => <TableRow key={doc.id}>
          <TableCell className="max-w-xs"><p className="break-words whitespace-normal font-medium">{doc.fileName}</p>{doc.error ? <p role="alert" className="mt-1 break-words whitespace-normal text-xs text-[var(--color-error)]">{documentError(doc)}</p> : null}{doc.warnings?.length ? <div className="mt-2 text-xs text-[var(--color-warning)]"><p>{text("parseWarnings")}</p><ul className="mt-1 list-disc pl-4 whitespace-normal">{doc.warnings.filter((warning) => !doc.imageAnalysis || warning !== "图片原件和位置已保留；图片语义描述需要单独验证").map((warning, index) => <li key={index}>{warning === "图片原件和位置已保留；图片语义描述需要单独验证" ? text("imageNotice") : warning === "Markdown图片链接原样保留；仅已有资源引用可供授权Agent读取，不自动下载外部图片" ? text("markdownImageNotice") : warning}</li>)}</ul></div> : null}<ImageAnalysisStatus summary={doc.imageAnalysis} /></TableCell>
          <TableCell>{doc.chunkConfig ? text(`modes.${doc.chunkConfig.mode}`) : text("inheritShort")}</TableCell>
          <TableCell title={doc.waitReason ? formatIngestionWaitReason(doc.waitReason, t) : undefined}>
            <p className="whitespace-nowrap text-sm">{statusLabel(doc)}</p>
            {doc.stage ? <p className="mt-1 whitespace-nowrap text-xs text-[var(--color-on-surface-variant)]">{t(`knowledgeLibrary.stages.${doc.stage}`, { defaultValue: doc.stage })}</p> : null}
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
    {operation.isPending ? <p role="status" className="mt-4 text-sm">{text(operation.variables?.type === "download" ? "downloading" : operation.variables?.type === "upload" ? "uploading" : "indexing")}</p> : null}
    <ConfirmDialog open={Boolean(deleteTarget)} title={text("deleteTitle")} description={t("knowledgeLibrary.deleteDescription", { file: deleteTarget?.fileName ?? "" })} cancelText={text("cancelDelete")} confirmText={text("deleteDocument")} pending={deletion.isPending} onCancel={() => setDeleteTarget(null)} onConfirm={() => { if (deleteTarget) deletion.mutate(deleteTarget); }} />
    {selected && library ? <DocumentSettingsDialog workspace={workspace} libraryId={id} document={selected} defaults={library.defaultChunkConfig} onClose={() => setSelected(null)} onChanged={() => query.refetch()} /> : null}
    {chunksDocument ? <KnowledgeDocumentChunksDialog workspace={workspace} libraryId={id} document={chunksDocument} onClose={() => setChunksDocument(null)} /> : null}
    {resources ? <KnowledgeDocumentResourcesDialog workspace={workspace} document={resources} onClose={() => setResources(null)} /> : null}
    {grants && library ? <KnowledgeGrantsDialog workspace={workspace} libraryId={id} grantedCodes={library.agentCodes} onClose={() => setGrants(false)} onChanged={() => query.refetch()} /> : null}
  </>;
}

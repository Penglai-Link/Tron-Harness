import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { downloadKnowledgeResource } from "../api/knowledgeBaseApi";
import { ResourceFilePreview } from "@/domain/resource/components/ResourceFilePreview";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";

function ResourcePreview({ libraryId, resourceCode, label, alt, onOpenResource, pdf = false }: {
  libraryId: string; resourceCode: string; label: string; alt: string; onOpenResource: (code: string) => void; pdf?: boolean;
}) {
  const { t } = useTranslation("pages");
  return <figure className="space-y-2 rounded-lg border border-[var(--color-card-border)] p-3">
    <figcaption className="flex flex-wrap items-center justify-between gap-2"><span className="font-medium">{label}</span>
      <Button size="sm" variant="link" onClick={() => onOpenResource(resourceCode)}>{t("knowledgeLibrary.openResource")}</Button>
    </figcaption>
    <ResourceFilePreview cacheScope={`knowledge-library:${libraryId}`} loadBinary={(signal) => downloadKnowledgeResource(libraryId, resourceCode, signal)} resourceCode={resourceCode} fileName={alt} kind={pdf ? "PDF" : "IMAGE"} format={pdf ? "pdf" : "image"} compact />
  </figure>;
}

export function KnowledgeDocumentResourcesDialog({ libraryId, document, onClose }: {
  libraryId: string; document: KnowledgeDocument; onClose: () => void;
}) {
  const { t } = useTranslation("pages");
  const [previewCode, setPreviewCode] = useState<string | null>(null);
  const images = document.parsingArtifact?.imageResourceCodes ?? [];
  function openResource(code: string) {
    setPreviewCode(code);
  }
  if (previewCode) return <Dialog open onOpenChange={(open) => { if (!open) setPreviewCode(null); }}>
    <DialogContent className="sm:max-w-5xl" closeLabel={t("knowledgeLibrary.close")}><DialogHeader><DialogTitle>{previewCode === document.resourceCode ? document.fileName : t("knowledgeLibrary.resourcesHelp")}</DialogTitle></DialogHeader>
      <ResourceFilePreview cacheScope={`knowledge-library:${libraryId}`} resourceCode={previewCode} fileName={previewCode === document.resourceCode ? document.fileName : "image"} kind={previewCode === document.resourceCode ? "FILE" : "IMAGE"} loadBinary={(signal) => downloadKnowledgeResource(libraryId, previewCode, signal)} />
    </DialogContent>
  </Dialog>;
  return <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
    <DialogContent closeLabel={t("knowledgeLibrary.close")} className="sm:max-w-3xl max-h-[90vh] overflow-y-auto">
      <DialogHeader><DialogTitle>{document.fileName}</DialogTitle><DialogDescription>{t("knowledgeLibrary.resourcesHelp")}</DialogDescription></DialogHeader>
      <Button variant="outline" onClick={() => openResource(document.resourceCode)}>{t("knowledgeLibrary.originalResource")}</Button>
      {document.fileName.toLowerCase().endsWith(".pdf") ? <ResourcePreview libraryId={libraryId} resourceCode={document.resourceCode} label={t("knowledgeLibrary.original")} alt={document.fileName} onOpenResource={openResource} pdf /> : null}
      {images.map((code, index) => <ResourcePreview key={code} libraryId={libraryId} resourceCode={code} label={t("knowledgeLibrary.imageTitle", { number: index + 1 })} alt={t("knowledgeLibrary.imageAlt", { number: index + 1, file: document.fileName })} onOpenResource={openResource} />)}
      <DialogFooter><Button variant="outline" onClick={onClose}>{t("knowledgeLibrary.close")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}

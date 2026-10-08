import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { ResourceFilePreview } from "@/domain/resource/components/ResourceFilePreview";
import type { KnowledgeDocument } from "../model/KnowledgeLibraryModel";

function ResourcePreview({ workspace, resourceCode, label, alt, onOpenResource, pdf = false }: {
  workspace: string; resourceCode: string; label: string; alt: string; onOpenResource: (code: string) => void; pdf?: boolean;
}) {
  const { t } = useTranslation("pages");
  return <figure className="space-y-2 rounded-lg border border-[var(--color-card-border)] p-3">
    <figcaption className="flex flex-wrap items-center justify-between gap-2"><span className="font-medium">{label}</span>
      <Button size="sm" variant="link" onClick={() => onOpenResource(resourceCode)}>{t("knowledgeLibrary.openResource")}</Button>
    </figcaption>
    <ResourceFilePreview workspaceCode={workspace} resourceCode={resourceCode} fileName={alt} kind={pdf ? "PDF" : "IMAGE"} format={pdf ? "pdf" : "image"} compact />
  </figure>;
}

export function KnowledgeDocumentResourcesDialog({ workspace, document, onClose }: {
  workspace: string; document: KnowledgeDocument; onClose: () => void;
}) {
  const { t } = useTranslation("pages");
  const navigate = useNavigate();
  const images = document.parsingArtifact?.imageResourceCodes ?? [];
  function openResource(code: string) {
    onClose();
    navigate(`/resource-management/files/detail?resourceCode=${encodeURIComponent(code)}&preview=1`);
  }
  return <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
    <DialogContent closeLabel={t("knowledgeLibrary.close")} className="sm:max-w-3xl max-h-[90vh] overflow-y-auto">
      <DialogHeader><DialogTitle>{document.fileName}</DialogTitle><DialogDescription>{t("knowledgeLibrary.resourcesHelp")}</DialogDescription></DialogHeader>
      <Button variant="outline" onClick={() => openResource(document.resourceCode)}>{t("knowledgeLibrary.originalResource")}</Button>
      {document.fileName.toLowerCase().endsWith(".pdf") ? <ResourcePreview workspace={workspace} resourceCode={document.resourceCode} label={t("knowledgeLibrary.original")} alt={document.fileName} onOpenResource={openResource} pdf /> : null}
      {images.map((code, index) => <ResourcePreview key={code} workspace={workspace} resourceCode={code} label={t("knowledgeLibrary.imageTitle", { number: index + 1 })} alt={t("knowledgeLibrary.imageAlt", { number: index + 1, file: document.fileName })} onOpenResource={openResource} />)}
      <DialogFooter><Button variant="outline" onClick={onClose}>{t("knowledgeLibrary.close")}</Button></DialogFooter>
    </DialogContent>
  </Dialog>;
}

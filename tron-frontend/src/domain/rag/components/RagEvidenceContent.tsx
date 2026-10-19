import { useTranslation } from "react-i18next";
import { downloadKnowledgeResource } from "@/domain/knowledge-base/api/knowledgeBaseApi";
import { ResourceFilePreview } from "@/domain/resource/components/ResourceFilePreview";
import type { RagEvidence } from "../model/RagModel";
import { RagEvidenceText } from "./RagEvidenceText";

function SourcePosition({ fileName, locations }: { fileName: string; locations: Record<string, unknown>[] }) {
  const { t, i18n } = useTranslation("pages");
  const number = (value: number) => new Intl.NumberFormat(i18n.language).format(value);
  const positions = [...new Set(locations.map(location => {
    const parts: string[] = [];
    if (typeof location.page === "number") parts.push(t("knowledgeLibrary.location.page", { value: number(location.page) }));
    else if (Array.isArray(location.pages) && location.pages.every(value => typeof value === "number") && location.pages.length) parts.push(t("knowledgeLibrary.location.page", { value: location.pages.map(number).join(", ") }));
    if (typeof location.sheet === "string") parts.push(t("knowledgeLibrary.location.sheet", { value: location.sheet }));
    if (Array.isArray(location.cellRanges) && location.cellRanges.every(value => typeof value === "string")) parts.push(location.cellRanges.join(", "));
    if (typeof location.paragraph === "number") parts.push(t("knowledgeLibrary.location.paragraphs", { value: number(location.paragraph) }));
    if (typeof location.lineStart === "number") parts.push(t("knowledgeLibrary.location.lines", { value: number(location.lineStart) }));
    if (typeof location.heading === "string") parts.push(location.heading);
    if (!parts.length && typeof location.sourceSegment === "number") {
      parts.push(t("ragUnified.sourceSegment", { value: number(location.sourceSegment + 1) }));
      if (typeof location.segmentPage === "number") parts.push(t("ragUnified.segmentPage", { value: number(location.segmentPage) }));
    }
    return parts.filter(Boolean).join(" · ");
  }).filter(Boolean))];
  return <p className="break-words text-sm text-muted-foreground">{fileName}{positions.length ? ` · ${positions.join("；")}` : ""}</p>;
}

export function RagEvidenceContent({ hit, libraryId }: { hit: RagEvidence; libraryId?: string }) {
  const { t } = useTranslation("pages");
  const images = hit.images ?? [];
  const body = hit.parts?.filter(part => !part.imageVector && !(images.length && ["image", "image_vector"].includes(String(part.location?.blockKind))));
  return <div className="grid min-w-0 gap-4">
    {body?.length ? body.map(part => <div key={part.chunkCode} className="grid gap-2">
      <RagEvidenceText text={part.text} />
      <SourcePosition fileName={part.fileName ?? hit.title ?? hit.sourceCode} locations={part.sourceLocations.length ? part.sourceLocations : [part.location]} />
    </div>) : <RagEvidenceText text={hit.text} />}
    {images.map((image, index) => <figure key={`${image.id}:${image.resourceCode}`} className="grid min-w-0 gap-3 border-t pt-4">
      <figcaption className="font-medium">{t("knowledgeLibrary.imageTitle", { number: index + 1 })}</figcaption>
      <ResourceFilePreview compact kind="IMAGE" format="image" resourceCode={image.resourceCode} fileName={t("knowledgeLibrary.imageAlt", { number: index + 1, file: image.fileName ?? hit.title ?? hit.sourceCode })}
        workspaceCode={libraryId ? undefined : image.workspaceCode} cacheScope={libraryId ? `knowledge-library:${libraryId}` : undefined}
        loadBinary={libraryId ? signal => downloadKnowledgeResource(libraryId, image.resourceCode, signal) : undefined} />
      {image.annotations.map((annotation, i) => <div key={`${annotation.kind}:${i}`} className="grid gap-1">
        <span className="text-sm font-medium">{t(`ragUnified.imageAnnotations.${annotation.kind}`)}</span>
        <RagEvidenceText text={annotation.text} />
      </div>)}
      <SourcePosition fileName={image.fileName ?? hit.title ?? hit.sourceCode} locations={[image.location]} />
    </figure>)}
    {hit.truncatedBy?.length ? <p role="status" className="text-sm text-muted-foreground">{t("ragUnified.partialSourceGroup")}</p> : null}
  </div>;
}

import type { TFunction } from "i18next";
import type { DocumentLocation } from "./KnowledgeLibraryModel";

export function formatDocumentLocation(location: DocumentLocation, t: TFunction, locale?: string) {
    const number = (value: number) => new Intl.NumberFormat(locale).format(value);
    const range = (start: number, end?: number) => end && end !== start ? `${number(start)}–${number(end)}` : number(start);
    const parts: string[] = [];
    if (location.pages?.length) parts.push(t("knowledgeLibrary.location.page", { value: location.pages.map(number).join(", ") }));
    else if (location.page) parts.push(t("knowledgeLibrary.location.page", { value: number(location.page) }));
    if (location.renderedPage) parts.push(t("knowledgeLibrary.location.renderedPage", { value: number(location.renderedPage) }));
    if (location.chartSources?.length) parts.push(location.chartSources.join("; "));
    if (location.sheet) parts.push(t("knowledgeLibrary.location.sheet", { value: location.sheet }));
    if (location.cellRanges?.length) parts.push(location.cellRanges.slice(0, 8).join(", "));
    if (!parts.length && location.blocks?.length) parts.push(location.blocks.join(", "));
    if (location.rowStart) parts.push(t("knowledgeLibrary.location.rows", { value: range(location.rowStart, location.rowEnd) }));
    if (location.paragraph) parts.push(t("knowledgeLibrary.location.paragraphs", { value: range(location.paragraph, location.paragraphEnd) }));
    if (location.lineStart) parts.push(t("knowledgeLibrary.location.lines", { value: range(location.lineStart, location.lineEnd) }));
    if (location.heading) parts.push(location.heading);
    if (!parts.length) parts.push(t("knowledgeLibrary.location.chars", { value: range(location.charStart + 1, location.charEnd) }));
    return parts.join(" · ");
  }

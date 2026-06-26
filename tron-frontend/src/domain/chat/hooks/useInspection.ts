import type { TargetPageFrameInspectionResult } from "@/shared";

import type { ChatMessageAttachment } from "../model";

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
}

function readString(payload: Record<string, unknown> | null, key: string): string {
  const value = payload?.[key];

  return typeof value === "string" ? value : "";
}

function readNumber(payload: Record<string, unknown> | null, key: string): number | undefined {
  const value = payload?.[key];

  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function readAnnotationImage(payload: Record<string, unknown> | null): string | null {
  const annotationImage = asRecord(payload?.annotationImage);
  const dataUrl = annotationImage?.dataUrl;

  return typeof dataUrl === "string" && dataUrl.startsWith("data:image/") ? dataUrl : null;
}

function inspectionMetadata(result: TargetPageFrameInspectionResult): Record<string, unknown> {
  const dom = result.dom;
  const annotationImage = asRecord(dom.annotationImage);
  const pageUrl = readString(dom, "url") || result.event.frameUrl || "";
  const pageTitle = readString(dom, "title");
  const targetText = readString(dom, "text");
  const selector = readString(dom, "selector");
  const targetPath = readString(dom, "path");
  const targetSelector = readString(dom, "targetSelector");
  const targetPathText = readString(dom, "targetPath");
  const screenshot = {
    height: readNumber(annotationImage, "height"),
    markerNumber: readNumber(annotationImage, "markerNumber"),
    mimeType: readString(annotationImage, "mimeType") || undefined,
    sourceHeight: readNumber(annotationImage, "sourceHeight"),
    sourceWidth: readNumber(annotationImage, "sourceWidth"),
    width: readNumber(annotationImage, "width"),
  };

  return {
    event: {
      eventType: result.event.eventType,
      frameId: result.event.frameId,
      frameUrl: result.event.frameUrl,
      pageId: result.event.pageId,
      timestamp: result.event.timestamp,
    },
    page: {
      title: pageTitle || undefined,
      url: pageUrl || undefined,
    },
    remark: result.remark,
    screenshot,
    target: {
      path: targetPath || targetPathText || undefined,
      selector: selector || targetSelector || undefined,
      tagName: readString(dom, "tagName") || undefined,
      text: targetText || undefined,
    },
  };
}

export function createInspectionAttachment(result: TargetPageFrameInspectionResult): ChatMessageAttachment {
  const dom = result.dom;
  const summary = result.remark || readString(dom, "remark") || readString(dom, "url") || "已选择页面元素";

  return {
    id: `inspection-${Date.now()}`,
    imageUrl: readAnnotationImage(dom),
    kind: "inspection",
    metadata: inspectionMetadata(result),
    summary,
  };
}

export const useInspection = () => ({
  createInspectionAttachment,
});

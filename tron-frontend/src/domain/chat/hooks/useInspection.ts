import type { TargetPageFrameInspectionResult } from "@/shared";

import type { ChatMessageAttachment } from "../model";

export interface InspectionAttachmentUpload {
  file: File;
  kind: ChatMessageAttachment["kind"];
  metadata: Record<string, unknown>;
  summary: string;
}

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

function inspectionSummary(result: TargetPageFrameInspectionResult) {
  const dom = result.dom;
  return result.remark || readString(dom, "remark") || readString(dom, "url") || "已选择页面元素";
}

function extensionForImageType(contentType: string) {
  if (contentType === "image/jpeg") return "jpg";
  if (contentType === "image/webp") return "webp";
  if (contentType === "image/gif") return "gif";
  return "png";
}

async function annotationImageFile(
  result: TargetPageFrameInspectionResult,
  metadata: Record<string, unknown>,
): Promise<File> {
  const image = readAnnotationImage(result.dom);
  if (!image) {
    return new File(
      [JSON.stringify(metadata, null, 2)],
      `page-inspection-${Date.now()}.json`,
      { type: "application/json" },
    );
  }

  const response = await fetch(image);
  const blob = await response.blob();
  const imageMetadata = asRecord(metadata.screenshot);
  const contentType = blob.type || readString(imageMetadata, "mimeType") || "image/png";
  return new File(
    [blob],
    `page-inspection-${Date.now()}.${extensionForImageType(contentType)}`,
    { type: contentType },
  );
}

export async function createInspectionAttachmentUpload(
  result: TargetPageFrameInspectionResult,
): Promise<InspectionAttachmentUpload> {
  const metadata = inspectionMetadata(result);
  const file = await annotationImageFile(result, metadata);

  return {
    kind: "inspection",
    file,
    metadata,
    summary: inspectionSummary(result),
  };
}

export const useInspection = () => ({
  createInspectionAttachmentUpload,
});

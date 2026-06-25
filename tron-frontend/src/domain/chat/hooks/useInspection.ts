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

function readAnnotationImage(payload: Record<string, unknown> | null): string | null {
  const annotationImage = asRecord(payload?.annotationImage);
  const dataUrl = annotationImage?.dataUrl;

  return typeof dataUrl === "string" && dataUrl.startsWith("data:image/") ? dataUrl : null;
}

function inspectionTitle(payload: Record<string, unknown> | null) {
  return readString(payload, "text")
    || readString(payload, "selector")
    || readString(payload, "tagName")
    || "页面审查";
}

export function createInspectionAttachment(result: TargetPageFrameInspectionResult): ChatMessageAttachment {
  const dom = result.dom;
  const title = inspectionTitle(dom);
  const summary = result.remark || readString(dom, "remark") || readString(dom, "url") || "已选择页面元素";

  return {
    id: `inspection-${Date.now()}`,
    imageUrl: readAnnotationImage(dom),
    kind: "inspection",
    metadata: {
      dom,
      event: result.event,
      remark: result.remark,
    },
    summary,
    title,
  };
}

export const useInspection = () => ({
  createInspectionAttachment,
});

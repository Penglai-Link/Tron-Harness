import { describe, expect, it, vi } from "vitest";

import type { TargetPageFrameInspectionResult } from "@/shared";

import { createInspectionAttachment } from "./useInspection";

function findDataUrl(value: unknown): string[] {
  if (!value || typeof value !== "object") {
    return [];
  }
  if (Array.isArray(value)) {
    return value.flatMap(findDataUrl);
  }
  return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) => {
    if (key === "dataUrl" && typeof child === "string") {
      return [child];
    }
    return findDataUrl(child);
  });
}

describe("createInspectionAttachment", () => {
  it("keeps the screenshot only in the top-level imageUrl", () => {
    vi.spyOn(Date, "now").mockReturnValue(123);
    const dataUrl = "data:image/png;base64,AAAA";
    const result: TargetPageFrameInspectionResult = {
      dom: {
        annotationImage: {
          dataUrl,
          height: 200,
          markerNumber: 1,
          mimeType: "image/png",
          sourceHeight: 400,
          sourceWidth: 600,
          width: 300,
        },
        path: "browser region",
        selector: "#target",
        tagName: "BUTTON",
        text: "Send",
        title: "Example page",
        url: "https://example.com/page",
      },
      event: {
        eventType: "tron:node-inspected",
        frameId: "frame-1",
        frameUrl: "https://example.com/page",
        pageId: "desktop-page",
        payload: {
          annotationImage: {
            dataUrl,
          },
        },
        timestamp: 1000,
      },
      remark: "检查按钮",
    };

    const attachment = createInspectionAttachment(result);

    expect(attachment).toMatchObject({
      id: "inspection-123",
      imageUrl: dataUrl,
      kind: "inspection",
      metadata: {
        event: {
          eventType: "tron:node-inspected",
          frameId: "frame-1",
          frameUrl: "https://example.com/page",
          pageId: "desktop-page",
          timestamp: 1000,
        },
        page: {
          title: "Example page",
          url: "https://example.com/page",
        },
        remark: "检查按钮",
        screenshot: {
          height: 200,
          markerNumber: 1,
          mimeType: "image/png",
          sourceHeight: 400,
          sourceWidth: 600,
          width: 300,
        },
        target: {
          path: "browser region",
          selector: "#target",
          tagName: "BUTTON",
          text: "Send",
        },
      },
      summary: "检查按钮",
    });
    expect(findDataUrl(attachment.metadata)).toEqual([]);
  });
});

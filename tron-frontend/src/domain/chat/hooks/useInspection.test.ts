import { describe, expect, it, vi } from "vitest";

import type { TargetPageFrameInspectionResult } from "@/shared";

import { createInspectionAttachmentUpload } from "./useInspection";

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

describe("createInspectionAttachmentUpload", () => {
  it("creates a resource upload payload without persisting the screenshot data URL in metadata", async () => {
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

    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(new Blob(["screenshot"], { type: "image/png" })),
    );
    const attachment = await createInspectionAttachmentUpload(result);

    expect(attachment).toMatchObject({
      file: expect.objectContaining({
        name: "page-inspection-123.png",
        type: "image/png",
      }),
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

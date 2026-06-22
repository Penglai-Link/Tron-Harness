import { describe, expect, it } from "vitest";

import { sanitizeInjectionEventPayload } from "./DesktopBridge";

describe("DesktopBridge", () => {
  it("keeps structured DOM data for node inspection events", () => {
    const event = sanitizeInjectionEventPayload({
      eventType: "tron:node-inspected",
      frameId: "frame-1",
      frameUrl: "https://example.com",
      pageId: "page-1",
      payload: {
        attributes: {
          class: "primary",
          id: "hero",
        },
        rect: {
          height: 40,
          width: 120,
          x: 12,
          y: 24,
        },
        remark: "Needs clearer copy",
        selector: "#hero",
        tagName: "button",
        text: "Start now",
      },
      timestamp: 1000,
    });

    expect(event?.eventType).toBe("tron:node-inspected");
    expect(event?.payload).toMatchObject({
      attributes: {
        class: "primary",
        id: "hero",
      },
      rect: {
        height: 40,
        width: 120,
        x: 12,
        y: 24,
      },
      remark: "Needs clearer copy",
      selector: "#hero",
      tagName: "button",
      text: "Start now",
    });
  });

  it("accepts inspection exit events from the injected frame", () => {
    const event = sanitizeInjectionEventPayload({
      eventType: "tron:inspection-exited",
      frameId: "frame-1",
      pageId: "page-1",
      payload: {
        reason: "escape",
      },
      timestamp: 1000,
    });

    expect(event).toMatchObject({
      eventType: "tron:inspection-exited",
      frameId: "frame-1",
      pageId: "page-1",
      payload: {
        reason: "escape",
      },
    });
  });

  it("keeps annotation image data URLs for node inspection events", () => {
    const dataUrl = `data:image/png;base64,${"x".repeat(400)}`;
    const event = sanitizeInjectionEventPayload({
      eventType: "tron:node-inspected",
      frameId: "frame-1",
      pageId: "page-1",
      payload: {
        annotationImage: {
          dataUrl,
          height: 110,
          markerNumber: 1,
          mimeType: "image/png",
          width: 220,
        },
      },
      timestamp: 1000,
    });

    expect(event?.payload).toMatchObject({
      annotationImage: {
        dataUrl,
        height: 110,
        markerNumber: 1,
        mimeType: "image/png",
        width: 220,
      },
    });
  });
});

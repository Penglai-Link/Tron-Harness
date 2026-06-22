// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";

import { DesktopBridge, sanitizeInjectionEventPayload } from "./DesktopBridge";

type WindowWithTauriInternals = Window & {
  __TAURI_INTERNALS__?: {
    invoke?: (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
  };
};

describe("DesktopBridge", () => {
  afterEach(() => {
    delete (window as unknown as WindowWithTauriInternals).__TAURI_INTERNALS__;
    vi.restoreAllMocks();
  });

  it("uses Tauri v2 internal invoke when the global Tauri object is not exposed", async () => {
    const invoke = vi.fn(async () => ({
      mode: "browser",
      opened: true,
      pageId: "page-1",
    }));
    (window as unknown as WindowWithTauriInternals).__TAURI_INTERNALS__ = { invoke };

    const bridge = new DesktopBridge();
    const response = await bridge.openTargetPage({
      mode: "browser",
      pageId: "page-1",
      url: "https://www.baidu.com",
    });

    expect(response.opened).toBe(true);
    expect(invoke).toHaveBeenCalledWith("openTargetPage", {
      request: {
        mode: "browser",
        pageId: "page-1",
        url: "https://www.baidu.com/",
      },
    });
  });

  it("returns the Tauri invoke failure reason when system browser launch fails", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("command openTargetPage not allowed");
    });
    (window as unknown as WindowWithTauriInternals).__TAURI_INTERNALS__ = { invoke };

    const bridge = new DesktopBridge();
    const response = await bridge.openTargetPage({
      mode: "browser",
      pageId: "page-1",
      url: "https://www.baidu.com",
    });

    expect(response).toMatchObject({
      mode: "browser",
      opened: false,
      pageId: "page-1",
      reason: "command openTargetPage not allowed",
    });
  });

  it("checks target iframe embedding support through the desktop bridge", async () => {
    const invoke = vi.fn(async () => ({
      blocked: true,
      embeddable: false,
      reason: "x-frame-options-deny",
      url: "https://github.com/",
    }));
    (window as unknown as WindowWithTauriInternals).__TAURI_INTERNALS__ = { invoke };

    const bridge = new DesktopBridge();
    const response = await bridge.checkTargetPageEmbedding({
      url: "https://github.com",
    });

    expect(response).toMatchObject({
      blocked: true,
      embeddable: false,
      reason: "x-frame-options-deny",
      url: "https://github.com/",
    });
    expect(invoke).toHaveBeenCalledWith("checkTargetPageEmbedding", {
      request: {
        url: "https://github.com/",
      },
    });
  });

  it("wraps single-argument desktop commands with the Rust parameter name", async () => {
    const invoke = vi.fn(async (command: string) => {
      if (command === "setInjectionEnabled") {
        return true;
      }

      if (command === "closeTargetPage") {
        return { closed: true };
      }

      return undefined;
    });
    (window as unknown as WindowWithTauriInternals).__TAURI_INTERNALS__ = { invoke };

    const bridge = new DesktopBridge();
    await bridge.setInjectionEnabled(true);
    await bridge.closeTargetPage({ pageId: "page-1" });
    await bridge.emitInjectionEvent({
      eventType: "tron:inspection-exited",
      frameId: "frame-1",
      pageId: "page-1",
      payload: {
        reason: "escape",
      },
    });

    expect(invoke).toHaveBeenCalledWith("setInjectionEnabled", {
      request: {
        enabled: true,
      },
    });
    expect(invoke).toHaveBeenCalledWith("closeTargetPage", {
      request: {
        pageId: "page-1",
      },
    });
    expect(invoke).toHaveBeenCalledWith("desktopInjectionEvent", {
      request: {
        event: expect.objectContaining({
          eventType: "tron:inspection-exited",
          pageId: "page-1",
        }),
      },
    });
  });

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

  it("preserves long URLs in navigation events", () => {
    const longUrl = `https://example.com/search?wd=${"query-".repeat(40)}&redirect=https%3A%2F%2Ftarget.example%2Fpath%3Fa%3D1#section`;
    const event = sanitizeInjectionEventPayload({
      eventType: "tron:navigation-requested",
      frameId: "frame-1",
      frameUrl: longUrl,
      pageId: "page-1",
      payload: {
        source: "target-blank-link",
        url: longUrl,
      },
      timestamp: 1000,
    });

    expect(event?.frameUrl).toBe(longUrl);
    expect(event?.payload).toMatchObject({
      source: "target-blank-link",
      url: longUrl,
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

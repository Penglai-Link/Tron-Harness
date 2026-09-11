// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  TargetPageFrame,
  type TargetPageFrameHandle,
} from "./TargetPageFrame";

const bridgeState = vi.hoisted(() => ({
  openTargetPage: vi.fn(),
}));

const visibilityState = vi.hoisted(() => ({
  visible: true,
}));

vi.mock("@/components/editable-tabs", async () => {
  const actual = await vi.importActual<typeof import("@/components/editable-tabs")>(
    "@/components/editable-tabs",
  );
  return {
    ...actual,
    useTabVisibility: () => visibilityState.visible,
  };
});

vi.mock("@/shared/desktop/DesktopBridge", async () => {
  const actual = await vi.importActual<typeof import("@/shared/desktop/DesktopBridge")>(
    "@/shared/desktop/DesktopBridge",
  );
  return {
    ...actual,
    resolveTargetWebviewPreloadUrl: () => "file:///target-preload.cjs",
    DesktopBridge: class {
      isDesktop() {
        return true;
      }

      openTargetPage = bridgeState.openTargetPage;

      checkTargetPageEmbedding() {
        return Promise.resolve({ blocked: false, embeddable: true, url: "" });
      }
    },
  };
});

interface WebviewMock extends HTMLElement {
  canGoBack: ReturnType<typeof vi.fn>;
  canGoForward: ReturnType<typeof vi.fn>;
  getURL: ReturnType<typeof vi.fn>;
  goBack: ReturnType<typeof vi.fn>;
  goForward: ReturnType<typeof vi.fn>;
  reload: ReturnType<typeof vi.fn>;
  send: ReturnType<typeof vi.fn>;
}

function prepareWebview(url = "https://example.com/") {
  const webview = document.querySelector("webview") as WebviewMock;
  Object.assign(webview, {
    canGoBack: vi.fn(() => false),
    canGoForward: vi.fn(() => false),
    getURL: vi.fn(() => url),
    goBack: vi.fn(),
    goForward: vi.fn(),
    reload: vi.fn(),
    send: vi.fn(() => Promise.resolve()),
  });
  return webview;
}

function dispatchWebviewEvent(webview: HTMLElement, type: string, fields = {}) {
  const event = new Event(type);
  Object.assign(event, fields);
  act(() => webview.dispatchEvent(event));
}

describe("TargetPageFrame webview", () => {
  afterEach(() => {
    cleanup();
    bridgeState.openTargetPage.mockReset();
    visibilityState.visible = true;
  });

  it("renders and focuses the address bar for a new empty tab", async () => {
    render(<TargetPageFrame initialUrl="" />);

    const addressInput = screen.getByRole("textbox", { name: "Target address" });
    expect(addressInput).toBeTruthy();
    await waitFor(() => expect(document.activeElement).toBe(addressInput));
  });

  it("does not send to the guest until dom-ready", async () => {
    const frameRef = createRef<TargetPageFrameHandle>();
    render(<TargetPageFrame ref={frameRef} pageId="session-1:page-1" />);

    act(() => frameRef.current?.open("https://example.com"));
    const webview = prepareWebview();

    expect(webview.send).not.toHaveBeenCalled();
    dispatchWebviewEvent(webview, "dom-ready");

    await waitFor(() => {
      expect(webview.send).toHaveBeenCalledWith(
        "tron-desktop:configure-target",
        expect.objectContaining({
          commentAvailable: false,
          enabled: false,
          pageId: "session-1page-1",
        }),
      );
    });
  });

  it("keeps the same guest and pauses inspection while its tab is hidden", async () => {
    const frameRef = createRef<TargetPageFrameHandle>();
    const { rerender } = render(
      <TargetPageFrame ref={frameRef} inspectionMode pageId="page-1" />,
    );
    act(() => frameRef.current?.open("https://example.com"));
    const webview = prepareWebview();
    dispatchWebviewEvent(webview, "dom-ready");
    fireEvent.click(screen.getByRole("button", { name: "Enable inspection" }));

    await waitFor(() => {
      expect(webview.send).toHaveBeenCalledWith(
        "tron-desktop:toggle-injection",
        expect.objectContaining({ enabled: true }),
      );
    });

    visibilityState.visible = false;
    rerender(<TargetPageFrame ref={frameRef} inspectionMode pageId="page-1" />);
    await waitFor(() => {
      expect(webview.send).toHaveBeenCalledWith(
        "tron-desktop:toggle-injection",
        expect.objectContaining({ commentAvailable: false, enabled: false }),
      );
    });

    visibilityState.visible = true;
    rerender(<TargetPageFrame ref={frameRef} inspectionMode pageId="page-1" />);
    await waitFor(() => {
      expect(webview.send.mock.calls.at(-1)).toEqual([
        "tron-desktop:toggle-injection",
        expect.objectContaining({ enabled: true }),
      ]);
    });
    expect(document.querySelector("webview")).toBe(webview);
  });

  it("syncs the address bar and history controls from guest navigation", async () => {
    const onUrlChange = vi.fn();
    const frameRef = createRef<TargetPageFrameHandle>();
    render(<TargetPageFrame ref={frameRef} onUrlChange={onUrlChange} />);
    act(() => frameRef.current?.open("https://www.tencent.com"));
    const webview = prepareWebview("https://www.tencent.com/climate/");
    webview.canGoBack.mockReturnValue(true);

    dispatchWebviewEvent(webview, "did-navigate", {
      isMainFrame: true,
      url: "https://www.tencent.com/climate/",
    });

    await waitFor(() => {
      expect(onUrlChange).toHaveBeenCalledWith("https://www.tencent.com/climate/");
      expect((screen.getByRole("button", { name: "Go back" }) as HTMLButtonElement).disabled).toBe(false);
    });
  });

  it("reconfigures inspection after every dom-ready", async () => {
    const frameRef = createRef<TargetPageFrameHandle>();
    render(<TargetPageFrame ref={frameRef} inspectionMode pageId="page-1" />);
    act(() => frameRef.current?.open("https://example.com"));
    const webview = prepareWebview();
    dispatchWebviewEvent(webview, "dom-ready");
    fireEvent.click(screen.getByRole("button", { name: "Enable inspection" }));
    await waitFor(() => expect(webview.send).toHaveBeenCalled());

    webview.send.mockClear();
    dispatchWebviewEvent(webview, "did-start-loading");
    dispatchWebviewEvent(webview, "dom-ready");

    await waitFor(() => {
      expect(webview.send).toHaveBeenCalledWith(
        "tron-desktop:configure-target",
        expect.objectContaining({
          commentAvailable: true,
          enabled: true,
          pageId: "page-1",
        }),
      );
    });
  });

  it("syncs inspection state when a context-menu comment starts", async () => {
    const frameRef = createRef<TargetPageFrameHandle>();
    render(<TargetPageFrame ref={frameRef} inspectionMode pageId="page-1" />);
    act(() => frameRef.current?.open("https://example.com"));
    const webview = prepareWebview();
    dispatchWebviewEvent(webview, "dom-ready");

    act(() => {
      window.postMessage(
        {
          channel: "tron:frame-event",
          eventType: "tron:inspection-started",
          frameId: "frame-1",
          pageId: "page-1",
          payload: { source: "context-menu" },
          timestamp: Date.now(),
        },
        "*",
      );
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Disable inspection" })).toBeTruthy();
      expect(webview.send).toHaveBeenCalledWith(
        "tron-desktop:toggle-injection",
        expect.objectContaining({ enabled: true }),
      );
    });
  });

  it("limits context-menu review to the matching visible webview", async () => {
    render(
      <>
        <TargetPageFrame inspectionMode pageId="page-1" />
        <TargetPageFrame inspectionMode pageId="page-2" />
      </>,
    );

    act(() => {
      window.postMessage(
        {
          channel: "tron:frame-event",
          eventType: "tron:inspection-started",
          frameId: "frame-1",
          pageId: "page-1",
          payload: { source: "context-menu" },
          timestamp: Date.now(),
        },
        "*",
      );
    });

    await waitFor(() => {
      expect(
        screen.getAllByRole("button", { name: "Disable inspection" }),
      ).toHaveLength(1);
      expect(
        screen.getAllByRole("button", { name: "Enable inspection" }),
      ).toHaveLength(1);
    });
  });

  it("ignores context-menu review events while the webview tab is hidden", async () => {
    visibilityState.visible = false;
    render(<TargetPageFrame inspectionMode pageId="page-1" />);

    act(() => {
      window.postMessage(
        {
          channel: "tron:frame-event",
          eventType: "tron:inspection-started",
          frameId: "frame-1",
          pageId: "page-1",
          payload: { source: "context-menu" },
          timestamp: Date.now(),
        },
        "*",
      );
    });

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Enable inspection" }),
      ).toBeTruthy();
    });
  });
});

// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { DesktopTargetMode } from "@/shared/desktop/DesktopBridge";

import { TargetPageFrame, type TargetPageFrameHandle, type TargetPageFrameInspectionResult } from "./TargetPageFrame";

const bridgeMethods = vi.hoisted(() => ({
  emitInjectionEvent: vi.fn(async () => undefined),
  getBackendStatus: vi.fn(async () => ({
    backendUrl: "http://127.0.0.1:8080",
    backendPort: 8080,
    ready: true,
    injectionEnabled: false,
    databasePath: "",
    graphPath: "",
  })),
  checkTargetPageEmbedding: vi.fn(async (request: { url: string }) => ({
    blocked: false,
    embeddable: true,
    reason: undefined as string | undefined,
    url: request.url,
  })),
  openTargetPage: vi.fn(async () => ({
    pageId: "page-1",
    opened: true,
    mode: "view" as DesktopTargetMode,
    reason: undefined as string | undefined,
  })),
  closeTargetPage: vi.fn(async () => true),
  isDesktop: vi.fn(() => false),
  resizeTargetPageView: vi.fn(async () => true),
  setInjectionEnabled: vi.fn(async (enabled: boolean) => enabled),
}));

vi.mock("@/shared/desktop/DesktopBridge", async () => {
  const actual = await vi.importActual<typeof import("@/shared/desktop/DesktopBridge")>("@/shared/desktop/DesktopBridge");

  return {
    ...actual,
    DesktopBridge: class {
      emitInjectionEvent = bridgeMethods.emitInjectionEvent;
      checkTargetPageEmbedding = bridgeMethods.checkTargetPageEmbedding;
      closeTargetPage = bridgeMethods.closeTargetPage;
      getBackendStatus = bridgeMethods.getBackendStatus;
      isDesktop = bridgeMethods.isDesktop;
      openTargetPage = bridgeMethods.openTargetPage;
      resizeTargetPageView = bridgeMethods.resizeTargetPageView;
      setInjectionEnabled = bridgeMethods.setInjectionEnabled;
    },
  };
});

function FrameHarness({
  onInspectionConfirm,
}: {
  onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
}) {
  const frameRef = useRef<TargetPageFrameHandle | null>(null);

  return (
    <>
      <button type="button" onClick={() => frameRef.current?.open("https://example.com")}>
        open frame
      </button>
      <button type="button" onClick={() => frameRef.current?.open("https://www.baidu.com")}>
        open baidu
      </button>
      <TargetPageFrame ref={frameRef} onInspectionConfirm={onInspectionConfirm} />
    </>
  );
}

function getIframe() {
  return screen.getByTitle(/Tron target desktop-/);
}

function dispatchFrameMessage(eventType: string, payload: Record<string, unknown> = {}) {
  const iframe = getIframe();
  const pageId = iframe.getAttribute("name") ?? "";

  window.dispatchEvent(new MessageEvent("message", {
    data: {
      channel: "tron:frame-event",
      eventType,
      frameId: "frame-1",
      pageId,
      payload,
      timestamp: 1000,
    },
  }));
}

describe("TargetPageFrame", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    bridgeMethods.checkTargetPageEmbedding.mockImplementation(async (request: { url: string }) => ({
      blocked: false,
      embeddable: true,
      reason: undefined,
      url: request.url,
    }));
    bridgeMethods.isDesktop.mockReturnValue(false);
  });

  it("opens from the public method and renders the browser shell around the iframe viewport", async () => {
    render(<FrameHarness />);

    expect(screen.queryByTitle(/Tron target desktop-/)).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));

    expect(await screen.findByLabelText("Go back")).toBeTruthy();
    expect(screen.getByLabelText("Go forward")).toBeTruthy();
    expect(screen.getByLabelText("Refresh frame")).toBeTruthy();
    expect(screen.getByLabelText("Enable inspection")).toBeTruthy();
    expect(screen.getByDisplayValue("example.com")).toBeTruthy();
    expect(getIframe().getAttribute("src")).toBe("https://example.com/");
  });

  it("sandboxes the target page without allowing popups from target blank links", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));

    expect(await screen.findByTitle(/Tron target desktop-/)).toBeTruthy();
    expect(getIframe().getAttribute("sandbox")).toContain("allow-scripts");
    expect(getIframe().getAttribute("sandbox")).not.toContain("allow-popups");
  });

  it("keeps target blank navigation inside the iframe and updates the address history", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));
    await screen.findByTitle(/Tron target desktop-/);

    dispatchFrameMessage("tron:navigation-requested", {
      source: "target-blank-link",
      url: "https://example.org/docs",
    });

    await waitFor(() => {
      expect(getIframe().getAttribute("src")).toBe("https://example.org/docs");
    });
    expect(screen.getByDisplayValue("example.org")).toBeTruthy();
    expect(screen.getByLabelText("Go back")).toHaveProperty("disabled", false);
    expect(bridgeMethods.checkTargetPageEmbedding).toHaveBeenCalledWith({
      url: "https://example.org/docs",
    });
  });

  it("preserves long target blank query parameters when navigating inside the iframe", async () => {
    const longTargetUrl = `https://example.org/search?wd=${"baidu-".repeat(40)}&from=target_blank#result`;

    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));
    await screen.findByTitle(/Tron target desktop-/);

    dispatchFrameMessage("tron:navigation-requested", {
      source: "target-blank-link",
      url: longTargetUrl,
    });

    await waitFor(() => {
      expect(getIframe().getAttribute("src")).toBe(longTargetUrl);
    });
    const addressInput = screen.getByLabelText("Target address");
    expect(addressInput).toHaveProperty("value", "example.org");
    fireEvent.focus(addressInput);
    expect(addressInput).toHaveProperty("value", longTargetUrl);
    expect(bridgeMethods.checkTargetPageEmbedding).toHaveBeenCalledWith({
      url: longTargetUrl,
    });
  });

  it("keeps hash route parameters when embedding precheck returns a URL without fragments", async () => {
    const hashRouteUrl = "http://qbee.qingtui.cn/knowledge/#/share/shareLibrary?shareLibrary=1&objectId=814999419248685056&shareId=857288832535728128&shareChildren=2";
    bridgeMethods.checkTargetPageEmbedding.mockImplementation(async (request: { url: string }) => {
      if (request.url === hashRouteUrl) {
        return {
          blocked: false,
          embeddable: true,
          reason: undefined,
          url: "http://qbee.qingtui.cn/knowledge/",
        };
      }

      return {
        blocked: false,
        embeddable: true,
        reason: undefined,
        url: request.url,
      };
    });

    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));
    await screen.findByTitle(/Tron target desktop-/);

    dispatchFrameMessage("tron:navigation-requested", {
      source: "target-blank-link",
      url: hashRouteUrl,
    });

    await waitFor(() => {
      expect(getIframe().getAttribute("src")).toBe(hashRouteUrl);
    });
    const addressInput = screen.getByLabelText("Target address");
    expect(addressInput).toHaveProperty("value", "qbee.qingtui.cn");
    fireEvent.focus(addressInput);
    expect(addressInput).toHaveProperty("value", hashRouteUrl);
    fireEvent.blur(addressInput, { relatedTarget: null });
    await waitFor(() => {
      expect(addressInput).toHaveProperty("value", "qbee.qingtui.cn");
    });
    fireEvent.focus(addressInput);
    expect(addressInput).toHaveProperty("value", hashRouteUrl);
    fireEvent.keyDown(addressInput, { key: "Escape" });
    expect(addressInput).toHaveProperty("value", "qbee.qingtui.cn");
  });

  it("shows an unsupported-page prompt when the target response forbids iframe embedding", async () => {
    bridgeMethods.checkTargetPageEmbedding.mockResolvedValueOnce({
      blocked: true,
      embeddable: false,
      reason: "x-frame-options-deny",
      url: "https://www.baidu.com/",
    });

    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open baidu" }));

    expect(await screen.findByText("当前页面不支持在内置浏览器中打开")).toBeTruthy();
    expect(screen.getByText("请使用系统浏览器打开该页面。")).toBeTruthy();
    expect(screen.queryByTitle(/Tron target desktop-/)).toBeNull();
    expect(bridgeMethods.checkTargetPageEmbedding).toHaveBeenCalledWith({
      url: "https://www.baidu.com/",
    });
    expect(bridgeMethods.openTargetPage).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "使用系统浏览器打开" }));

    await waitFor(() => {
      expect(bridgeMethods.openTargetPage).toHaveBeenCalledWith(expect.objectContaining({
        mode: "browser",
        url: "https://www.baidu.com/",
      }));
    });
  });

  it("falls back to a browser tab for iframe-blocked pages when the desktop bridge is unavailable", async () => {
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null);
    bridgeMethods.checkTargetPageEmbedding.mockResolvedValueOnce({
      blocked: true,
      embeddable: false,
      reason: "x-frame-options-deny",
      url: "https://www.baidu.com/",
    });
    bridgeMethods.openTargetPage.mockResolvedValueOnce({
      pageId: "page-1",
      opened: false,
      mode: "browser",
      reason: "running-in-browser",
    });

    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open baidu" }));
    expect(await screen.findByText("当前页面不支持在内置浏览器中打开")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "使用系统浏览器打开" }));

    await waitFor(() => {
      expect(openSpy).toHaveBeenCalledWith("https://www.baidu.com/", "_blank");
    });
    expect(screen.queryByTitle(/Tron target desktop-/)).toBeNull();
    openSpy.mockRestore();
  });

  it("tries the iframe when the target response does not explicitly forbid embedding", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open baidu" }));

    expect(await screen.findByTitle(/Tron target desktop-/)).toBeTruthy();
    expect(getIframe().getAttribute("src")).toBe("https://www.baidu.com/");
    expect(screen.queryByText("当前页面不支持在内置浏览器中打开")).toBeNull();
  });

  it("maintains address input internally and reloads the iframe after submit", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));

    const addressInput = await screen.findByLabelText("Target address");
    fireEvent.focus(addressInput);
    fireEvent.change(addressInput, { target: { value: "https://example.org/docs" } });

    expect(screen.getByLabelText("Load target address")).toBeTruthy();

    fireEvent.click(screen.getByLabelText("Load target address"));

    expect(getIframe().getAttribute("src")).toBe("https://example.org/docs");
    expect(screen.queryByLabelText("Load target address")).toBeNull();
    expect(screen.getByDisplayValue("example.org")).toBeTruthy();
    expect(document.activeElement).not.toBe(addressInput);
  });

  it("loads the edited address on Enter and exits address editing mode", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));

    const addressInput = await screen.findByLabelText("Target address");
    fireEvent.focus(addressInput);
    fireEvent.change(addressInput, { target: { value: "https://example.net/page" } });
    fireEvent.keyDown(addressInput, { key: "Enter" });

    expect(getIframe().getAttribute("src")).toBe("https://example.net/page");
    expect(screen.queryByLabelText("Load target address")).toBeNull();
    expect(screen.getByDisplayValue("example.net")).toBeTruthy();
    expect(document.activeElement).not.toBe(addressInput);
  });

  it("turns inspection off after applying a new target address", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));
    const addressInput = await screen.findByLabelText("Target address");
    fireEvent.click(screen.getByLabelText("Enable inspection"));

    await waitFor(() => {
      expect(screen.getByLabelText("Disable inspection")).toBeTruthy();
    });

    fireEvent.focus(addressInput);
    fireEvent.change(addressInput, { target: { value: "https://example.org/docs" } });
    fireEvent.click(screen.getByLabelText("Load target address"));

    await waitFor(() => {
      expect(screen.getByLabelText("Enable inspection")).toBeTruthy();
    });
    expect(getIframe().getAttribute("src")).toBe("https://example.org/docs");
    expect(bridgeMethods.setInjectionEnabled).toHaveBeenLastCalledWith(false);
  });

  it("refreshes the iframe and toggles inspection from the toolbar action", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));

    const iframe = await screen.findByTitle(/Tron target desktop-/);
    const initialKey = iframe.getAttribute("data-frame-key");

    fireEvent.click(screen.getByLabelText("Refresh frame"));
    expect(getIframe().getAttribute("data-frame-key")).not.toBe(initialKey);

    fireEvent.click(screen.getByLabelText("Enable inspection"));

    await waitFor(() => {
      expect(bridgeMethods.setInjectionEnabled).toHaveBeenLastCalledWith(true);
    });
    expect(screen.getByLabelText("Disable inspection")).toBeTruthy();
  });

  it("turns inspection off when refreshing the iframe", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));

    const iframe = await screen.findByTitle(/Tron target desktop-/);
    const initialKey = iframe.getAttribute("data-frame-key");

    fireEvent.click(screen.getByLabelText("Enable inspection"));
    await waitFor(() => {
      expect(screen.getByLabelText("Disable inspection")).toBeTruthy();
    });

    fireEvent.click(screen.getByLabelText("Refresh frame"));

    await waitFor(() => {
      expect(screen.getByLabelText("Enable inspection")).toBeTruthy();
    });
    expect(getIframe().getAttribute("data-frame-key")).not.toBe(initialKey);
    expect(bridgeMethods.setInjectionEnabled).toHaveBeenLastCalledWith(false);
  });

  it("turns inspection off when the injected frame reports an Escape exit", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));
    await screen.findByTitle(/Tron target desktop-/);
    fireEvent.click(screen.getByLabelText("Enable inspection"));

    dispatchFrameMessage("tron:inspection-exited", {
      reason: "escape",
    });

    await waitFor(() => {
      expect(screen.getByLabelText("Enable inspection")).toBeTruthy();
    });
  });

  it("returns DOM and remark information when inspection is confirmed", async () => {
    const onInspectionConfirm = vi.fn();

    render(<FrameHarness onInspectionConfirm={onInspectionConfirm} />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));
    await screen.findByTitle(/Tron target desktop-/);

    dispatchFrameMessage("tron:node-inspected", {
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

    expect(onInspectionConfirm).toHaveBeenCalledWith(expect.objectContaining({
      dom: expect.objectContaining({
        remark: "Needs clearer copy",
        selector: "#hero",
        tagName: "button",
      }),
      remark: "Needs clearer copy",
    }));
    expect(bridgeMethods.emitInjectionEvent).toHaveBeenCalledWith(expect.objectContaining({
      eventType: "tron:node-inspected",
    }));
  });

  it("opens target pages in a desktop WebContentsView and syncs native bounds", async () => {
    bridgeMethods.isDesktop.mockReturnValue(true);
    const originalResizeObserver = globalThis.ResizeObserver;
    const originalRequestAnimationFrame = window.requestAnimationFrame;
    const originalCancelAnimationFrame = window.cancelAnimationFrame;
    class TestResizeObserver {
      observe = vi.fn();
      disconnect = vi.fn();
    }
    globalThis.ResizeObserver = TestResizeObserver as unknown as typeof ResizeObserver;
    window.requestAnimationFrame = ((callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    }) as typeof window.requestAnimationFrame;
    window.cancelAnimationFrame = vi.fn() as unknown as typeof window.cancelAnimationFrame;

    try {
      render(<FrameHarness />);

      fireEvent.click(screen.getByRole("button", { name: "open frame" }));

      const placeholder = await screen.findByLabelText(/Tron target desktop-/);
      placeholder.getBoundingClientRect = () => ({
        bottom: 620,
        height: 560,
        left: 24,
        right: 1024,
        top: 60,
        width: 1000,
        x: 24,
        y: 60,
        toJSON: () => ({}),
      });
      window.dispatchEvent(new Event("resize"));

      await waitFor(() => {
        expect(bridgeMethods.openTargetPage).toHaveBeenCalledWith(expect.objectContaining({
          mode: "view",
          url: "https://example.com/",
        }));
      });
      expect(screen.queryByTitle(/Tron target desktop-/)).toBeNull();
      expect(bridgeMethods.resizeTargetPageView).toHaveBeenCalledWith(expect.objectContaining({
        bounds: expect.objectContaining({
          height: 560,
          width: 1000,
          x: 24,
          y: 60,
        }),
      }));
    } finally {
      globalThis.ResizeObserver = originalResizeObserver;
      window.requestAnimationFrame = originalRequestAnimationFrame;
      window.cancelAnimationFrame = originalCancelAnimationFrame;
    }
  });
});

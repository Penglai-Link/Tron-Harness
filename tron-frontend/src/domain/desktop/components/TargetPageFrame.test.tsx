// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

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
  openTargetPage: vi.fn(async () => ({
    pageId: "page-1",
    opened: true,
    mode: "window" as const,
  })),
  setInjectionEnabled: vi.fn(async (enabled: boolean) => enabled),
}));

vi.mock("@/shared/desktop/DesktopBridge", async () => {
  const actual = await vi.importActual<typeof import("@/shared/desktop/DesktopBridge")>("@/shared/desktop/DesktopBridge");

  return {
    ...actual,
    DesktopBridge: class {
      emitInjectionEvent = bridgeMethods.emitInjectionEvent;
      getBackendStatus = bridgeMethods.getBackendStatus;
      openTargetPage = bridgeMethods.openTargetPage;
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

  it("does not sandbox the target page so native link navigation can work", async () => {
    render(<FrameHarness />);

    fireEvent.click(screen.getByRole("button", { name: "open frame" }));

    expect(await screen.findByTitle(/Tron target desktop-/)).toBeTruthy();
    expect(getIframe().hasAttribute("sandbox")).toBe(false);
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
});

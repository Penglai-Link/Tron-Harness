// @vitest-environment jsdom
import { act, render, waitFor } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  TargetPageFrame,
  type TargetPageFrameHandle,
} from "./TargetPageFrame";

const bridgeState = vi.hoisted(() => ({
  openTargetPage: vi.fn(),
  setTargetPageViewVisibility: vi.fn(),
}));

vi.mock("@/shared/desktop/DesktopBridge", async () => {
  const actual = await vi.importActual<typeof import("@/shared/desktop/DesktopBridge")>(
    "@/shared/desktop/DesktopBridge",
  );

  return {
    ...actual,
    DesktopBridge: class {
      isDesktop() {
        return true;
      }

      getBackendStatus() {
        return Promise.resolve({
          backendPort: 0,
          backendUrl: "",
          injectionEnabled: false,
          ready: false,
        });
      }

      openTargetPage = bridgeState.openTargetPage;

      setTargetPageViewVisibility = bridgeState.setTargetPageViewVisibility;

      closeTargetPage() {
        return Promise.resolve(true);
      }

      resizeTargetPageView() {
        return Promise.resolve(true);
      }

      setInjectionEnabled(enabled: boolean) {
        return Promise.resolve(enabled);
      }

      emitInjectionEvent() {
        return Promise.resolve(true);
      }
    },
  };
});

class ResizeObserverMock {
  observe() {}

  disconnect() {}
}

describe("TargetPageFrame", () => {
  afterEach(() => {
    bridgeState.openTargetPage.mockReset();
    bridgeState.setTargetPageViewVisibility.mockReset();
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);
  });

  it("re-applies visibility after the desktop view is created", async () => {
    let resolveOpen: ((value: { opened: boolean }) => void) | undefined;
    bridgeState.openTargetPage.mockImplementation(
      () => new Promise((resolve) => { resolveOpen = resolve; }),
    );
    bridgeState.setTargetPageViewVisibility.mockResolvedValue(true);
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);

    const frameRef = createRef<TargetPageFrameHandle>();
    render(<TargetPageFrame ref={frameRef} />);

    act(() => frameRef.current?.open("https://example.com"));

    await waitFor(() => {
      expect(bridgeState.setTargetPageViewVisibility).toHaveBeenCalledWith(
        expect.objectContaining({ pageId: expect.any(String), visible: true }),
      );
    });
    const callsBeforeOpen = bridgeState.setTargetPageViewVisibility.mock.calls.length;

    await act(async () => {
      resolveOpen?.({ opened: true });
    });

    await waitFor(() => {
      expect(bridgeState.setTargetPageViewVisibility.mock.calls.length).toBeGreaterThan(callsBeforeOpen);
      expect(bridgeState.setTargetPageViewVisibility.mock.calls.at(-1)?.[0]).toEqual(
        expect.objectContaining({ visible: true }),
      );
    });
  });
});

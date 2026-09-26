// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PanelImperativeHandle, PanelSize } from "react-resizable-panels";

import { usePanel } from "./usePanel";

describe("usePanel", () => {
  it("uses the external open state and reports threshold collapses", () => {
    let collapsed = false;
    const size = { inPixels: 200 };
    const panelControls: PanelImperativeHandle = {
      collapse: vi.fn(() => {
        collapsed = true;
      }),
      expand: vi.fn(() => {
        collapsed = false;
      }),
      getSize: vi.fn(() => ({ asPercentage: 13, inPixels: size.inPixels })),
      isCollapsed: vi.fn(() => collapsed),
      resize: vi.fn(),
    };
    const onOpenChange = vi.fn();
    const onSizeChange = vi.fn();
    const { result } = renderHook(() => usePanel({
      onOpenChange,
      onSizeChange,
      open: true,
    }));

    act(() => {
      result.current.panelRef.current = panelControls;
      result.current.handlePanelPointerDown();
      result.current.panelResize({ inPixels: size.inPixels } as PanelSize);
    });

    expect(result.current.panelOpen).toBe(true);
    expect(onOpenChange).toHaveBeenLastCalledWith(false);

    act(() => {
      result.current.panelResize({ inPixels: size.inPixels } as PanelSize);
      result.current.handlePanelPointerEnd();
    });

    expect(panelControls.collapse).toHaveBeenCalled();
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onSizeChange).not.toHaveBeenCalled();
  });

  it("keeps a drag open when it finishes above the collapse threshold", () => {
    const size = { inPixels: 640 };
    let collapsed = false;
    const panelControls: PanelImperativeHandle = {
      collapse: vi.fn(() => {
        collapsed = true;
      }),
      expand: vi.fn(() => {
        collapsed = false;
      }),
      getSize: vi.fn(() => ({ asPercentage: 42, inPixels: size.inPixels })),
      isCollapsed: vi.fn(() => collapsed),
      resize: vi.fn(),
    };
    const onOpenChange = vi.fn();
    const { result } = renderHook(() => usePanel({
      onOpenChange,
      open: true,
    }));

    act(() => {
      result.current.panelRef.current = panelControls;
      result.current.handlePanelPointerDown();
      size.inPixels = 200;
      result.current.panelResize({ inPixels: size.inPixels } as PanelSize);
      result.current.handlePanelPointerMove();
      size.inPixels = 640;
      result.current.panelResize({ inPixels: size.inPixels } as PanelSize);
      result.current.handlePanelPointerEnd();
    });

    expect(panelControls.collapse).toHaveBeenCalled();
    expect(panelControls.expand).toHaveBeenCalled();
    expect(onOpenChange).toHaveBeenLastCalledWith(true);
    expect(result.current.panelOpen).toBe(true);
  });

  it("reflects controlled open changes without keeping a second open state", () => {
    const { result, rerender } = renderHook(
      ({ open }: { open: boolean }) => usePanel({ open }),
      { initialProps: { open: false } },
    );

    expect(result.current.panelOpen).toBe(false);

    rerender({ open: true });

    expect(result.current.panelOpen).toBe(true);
  });
});

it("does not save transient layout sizes while restoring a different session", () => {
  const onSizeChange = vi.fn();
  const { result, rerender } = renderHook(
    ({ resetKey, initialSize }) => usePanel({ open: true, resetKey, initialSize, onSizeChange }),
    { initialProps: { resetKey: "a", initialSize: 620 } },
  );
  rerender({ resetKey: "b", initialSize: 440 });
  act(() => result.current.panelResize({ inPixels: 620 } as PanelSize));
  expect(onSizeChange).not.toHaveBeenCalled();
});

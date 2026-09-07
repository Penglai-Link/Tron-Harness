// @vitest-environment jsdom
import { render, waitFor } from "@testing-library/react";
import { forwardRef, useEffect, useImperativeHandle } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PageView, type TargetPageFrameInspectionResult } from ".";

const openMock = vi.fn();
const inspectionResult: TargetPageFrameInspectionResult = {
  dom: {
    selector: "#hero",
  },
  event: {
    eventType: "tron:node-inspected",
    frameId: "frame-1",
    frameUrl: "https://example.com",
    pageId: "desktop-page",
    timestamp: 1000,
  },
  remark: "checked",
};

vi.mock("./TargetPageFrame", () => ({
  TargetPageFrame: forwardRef(({
    onInspectionConfirm,
  }: {
    inspectionMode?: boolean;
    onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
  }, ref) => {
    useImperativeHandle(ref, () => ({
      open: openMock,
    }));

    useEffect(() => {
      onInspectionConfirm?.(inspectionResult);
    }, [onInspectionConfirm]);

    return <div data-testid="target-page-frame" />;
  }),
}));

describe("PageView", () => {
  afterEach(() => {
    openMock.mockClear();
  });

  it("opens the target page when url is provided", async () => {
    render(<PageView url="https://example.com" />);

    await waitFor(() => {
      expect(openMock).toHaveBeenCalledWith("https://example.com");
    });
  });

  it("does not forward inspection results by default", async () => {
    const onInspectionConfirm = vi.fn();

    render(<PageView onInspectionConfirm={onInspectionConfirm} url="https://example.com" />);

    await waitFor(() => {
      expect(onInspectionConfirm).not.toHaveBeenCalled();
    });
  });

  it("forwards inspection results when inspection mode is enabled", async () => {
    const onInspectionConfirm = vi.fn();

    render(<PageView inspectionMode onInspectionConfirm={onInspectionConfirm} url="https://example.com" />);

    await waitFor(() => {
      expect(onInspectionConfirm).toHaveBeenCalledWith(inspectionResult);
    });
  });
});

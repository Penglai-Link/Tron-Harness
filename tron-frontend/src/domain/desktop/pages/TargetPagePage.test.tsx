// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { forwardRef, useImperativeHandle, useState, type ButtonHTMLAttributes, type HTMLAttributes } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TargetPagePage } from "./TargetPagePage";

vi.mock("@/shared", () => ({
  Button: ({ children, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) => <button {...props}>{children}</button>,
  SurfaceCard: ({ children, ...props }: HTMLAttributes<HTMLElement>) => <section {...props}>{children}</section>,
}));

vi.mock("../components/TargetPageFrame", () => ({
  TargetPageFrame: forwardRef(({
    onInspectionConfirm,
  }: {
    onInspectionConfirm?: (result: unknown) => void;
  }, ref) => {
    const [visible, setVisible] = useState(false);

    useImperativeHandle(ref, () => ({
      open: () => setVisible(true),
    }));

    if (!visible) {
      return null;
    }

    return (
      <div data-testid="target-page-frame">
        <button
          type="button"
          onClick={() => onInspectionConfirm?.({
            dom: {
              annotationImage: {
                dataUrl: "data:image/png;base64,abc",
                height: 110,
                markerNumber: 1,
                mimeType: "image/png",
                width: 220,
              },
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
              title: "Example",
              url: "https://example.com",
            },
            event: {
              eventType: "tron:node-inspected",
              pageId: "desktop-page",
              frameId: "frame-1",
              frameUrl: "https://example.com",
              timestamp: 1000,
            },
            remark: "Needs clearer copy",
          })}
        >
          emit inspection
        </button>
      </div>
    );
  }),
}));

describe("TargetPagePage", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("shows only the open button until the target frame is opened", () => {
    render(<TargetPagePage />);

    expect(screen.getByRole("button", { name: "Open target" })).toBeTruthy();
    expect(screen.queryByTestId("target-page-frame")).toBeNull();
  });

  it("opens the frame through the component method", async () => {
    render(<TargetPagePage />);

    fireEvent.click(screen.getByRole("button", { name: "Open target" }));

    await waitFor(() => {
      expect(screen.getByTestId("target-page-frame")).toBeTruthy();
    });
  });

  it("shows the latest inspection remark and DOM details from the target frame", async () => {
    render(<TargetPagePage />);

    fireEvent.click(screen.getByRole("button", { name: "Open target" }));
    fireEvent.click(screen.getByRole("button", { name: "emit inspection" }));

    expect(await screen.findByLabelText("Inspection result")).toBeTruthy();
    expect(screen.getByText("Needs clearer copy")).toBeTruthy();
    expect(screen.getByText("#hero")).toBeTruthy();
    expect(screen.getByText("button")).toBeTruthy();
    expect(screen.getByText("Start now")).toBeTruthy();
    expect(screen.getByText("x:12 y:24 w:120 h:40")).toBeTruthy();
    expect(screen.getByText('class="primary" id="hero"')).toBeTruthy();
    expect(screen.getByRole("img", { name: "Annotated DOM selection" })).toBeTruthy();
  });
});

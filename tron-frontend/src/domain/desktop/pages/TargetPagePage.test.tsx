// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ButtonHTMLAttributes } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TargetPagePage } from "./TargetPagePage";

const bridgeMethods = {
  emitInjectionEvent: vi.fn(),
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
};

vi.mock("@/shared", () => ({
  Button: ({ children, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) => <button {...props}>{children}</button>,
}));

vi.mock("@/shared/desktop/DesktopBridge", () => ({
  DesktopBridge: class {
    emitInjectionEvent = bridgeMethods.emitInjectionEvent;
    getBackendStatus = bridgeMethods.getBackendStatus;
    openTargetPage = bridgeMethods.openTargetPage;
    setInjectionEnabled = bridgeMethods.setInjectionEnabled;
  },
  sanitizeInjectionEventPayload: (value: unknown) => value,
  sanitizeUrlInput: (value: string) => value,
}));

vi.mock("../components/TargetPageFrame", () => ({
  TargetPageFrame: ({
    injectionEnabled,
    onInjectionEnabledChange,
  }: {
    injectionEnabled: boolean;
    onInjectionEnabledChange?: (enabled: boolean) => void;
  }) => (
    <div data-testid="target-page-frame">
      <span>{injectionEnabled ? "injection-on" : "injection-off"}</span>
      <button type="button" onClick={() => onInjectionEnabledChange?.(!injectionEnabled)}>
        toggle injection
      </button>
    </div>
  ),
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

  it("opens the frame with injection disabled and toggles injection from the frame toolbar", async () => {
    render(<TargetPagePage />);

    fireEvent.click(screen.getByRole("button", { name: "Open target" }));

    expect(screen.getByTestId("target-page-frame")).toBeTruthy();
    expect(screen.getByText("injection-off")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "toggle injection" }));

    await waitFor(() => {
      expect(screen.getByText("injection-on")).toBeTruthy();
    });
    await waitFor(() => {
      expect(bridgeMethods.setInjectionEnabled).toHaveBeenLastCalledWith(true);
    });
  });
});

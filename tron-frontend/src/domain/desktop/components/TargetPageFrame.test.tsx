// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TargetPageFrame } from "./TargetPageFrame";

describe("TargetPageFrame", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders the minimal browser shell around the iframe viewport", () => {
    render(
      <TargetPageFrame
        pageId="page-1"
        targetUrl="https://example.com"
        desktopBackendStatus={{
          backendUrl: "http://127.0.0.1:8080",
          backendPort: 8080,
          ready: true,
          injectionEnabled: false,
        }}
        injectionEnabled={false}
      />,
    );

    expect(screen.getByLabelText("Go back")).toBeTruthy();
    expect(screen.getByLabelText("Go forward")).toBeTruthy();
    expect(screen.getByLabelText("Refresh frame")).toBeTruthy();
    expect(screen.getByLabelText("Enable injection")).toBeTruthy();
    expect(screen.getByDisplayValue("example.com")).toBeTruthy();

    const iframe = screen.getByTitle("Tron target page-1");
    expect(iframe.getAttribute("src")).toBe("https://example.com");
  });

  it("does not sandbox the target page so native link navigation can work", () => {
    render(
      <TargetPageFrame
        pageId="page-1"
        targetUrl="https://example.com"
        desktopBackendStatus={{
          backendUrl: "http://127.0.0.1:8080",
          backendPort: 8080,
          ready: true,
          injectionEnabled: false,
        }}
        injectionEnabled={false}
      />,
    );

    const iframe = screen.getByTitle("Tron target page-1");
    expect(iframe.hasAttribute("sandbox")).toBe(false);
  });

  it("edits the address bar and reloads the iframe after submit", () => {
    const onTargetUrlChange = vi.fn();

    render(
      <TargetPageFrame
        pageId="page-1"
        targetUrl="https://example.com"
        desktopBackendStatus={{
          backendUrl: "http://127.0.0.1:8080",
          backendPort: 8080,
          ready: true,
          injectionEnabled: false,
        }}
        injectionEnabled={false}
        onTargetUrlChange={onTargetUrlChange}
      />,
    );

    const addressInput = screen.getByLabelText("Target address");
    fireEvent.focus(addressInput);
    fireEvent.change(addressInput, { target: { value: "https://example.org/docs" } });

    expect(screen.getByLabelText("Load target address")).toBeTruthy();

    fireEvent.click(screen.getByLabelText("Load target address"));

    const iframe = screen.getByTitle("Tron target page-1");
    expect(iframe.getAttribute("src")).toBe("https://example.org/docs");
    expect(onTargetUrlChange).toHaveBeenCalledWith("https://example.org/docs");
    expect(screen.queryByLabelText("Load target address")).toBeNull();
    expect(screen.getByDisplayValue("example.org")).toBeTruthy();
  });

  it("refreshes the iframe and toggles injection from the toolbar action", () => {
    const onInjectionEnabledChange = vi.fn();

    render(
      <TargetPageFrame
        pageId="page-1"
        targetUrl="https://example.com"
        desktopBackendStatus={{
          backendUrl: "http://127.0.0.1:8080",
          backendPort: 8080,
          ready: true,
          injectionEnabled: false,
        }}
        injectionEnabled={false}
        onInjectionEnabledChange={onInjectionEnabledChange}
      />,
    );

    const iframe = screen.getByTitle("Tron target page-1");
    const initialKey = iframe.getAttribute("data-frame-key");

    fireEvent.click(screen.getByLabelText("Refresh frame"));
    expect(screen.getByTitle("Tron target page-1").getAttribute("data-frame-key")).not.toBe(initialKey);

    fireEvent.click(screen.getByLabelText("Enable injection"));
    expect(onInjectionEnabledChange).toHaveBeenCalledWith(true);
  });
});

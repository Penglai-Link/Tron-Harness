// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import BottomPanel from "./BottomPanel";

const terminalLifecycle = vi.hoisted(() => ({ mounts: 0, unmounts: 0 }));
const panelControls = vi.hoisted(() => ({
  collapse: vi.fn(),
  expand: vi.fn(),
  resize: vi.fn(),
}));

vi.mock("@/components/terminal", async () => {
  const React = await import("react");

  return {
    default: function MockTerminal() {
      React.useEffect(() => {
        terminalLifecycle.mounts += 1;
        return () => {
          terminalLifecycle.unmounts += 1;
        };
      }, []);

      return <div data-testid="terminal-instance" />;
    },
  };
});

vi.mock("@/components/editable-tabs", () => ({
  EditableTabs: ({ items }: { items: Array<{ children: ReactNode; key: string }> }) => (
    <>{items.map((item) => <div key={item.key}>{item.children}</div>)}</>
  ),
}));

vi.mock("@/components/ui/resizable", () => ({
  ResizableHandle: ({ className }: { className?: string }) => <div className={className} />,
  ResizablePanel: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock("@/shared/hooks/usePanel", () => ({
  usePanel: () => ({
    handlePanelPointerDown: vi.fn(),
    handlePanelPointerEnd: vi.fn(),
    handlePanelPointerMove: vi.fn(),
    panelOpen: true,
    panelRef: { current: panelControls },
    panelResize: vi.fn(),
  }),
}));

describe("BottomPanel", () => {
  afterEach(() => {
    cleanup();
    terminalLifecycle.mounts = 0;
    terminalLifecycle.unmounts = 0;
    panelControls.collapse.mockClear();
    panelControls.expand.mockClear();
    panelControls.resize.mockClear();
  });

  it("keeps the terminal mounted while the panel is hidden", async () => {
    const { rerender } = render(<BottomPanel visible={false} />);

    expect(screen.queryByTestId("terminal-instance")).toBeNull();

    rerender(<BottomPanel visible />);
    await waitFor(() => expect(screen.getByTestId("terminal-instance")).toBeTruthy());
    expect(terminalLifecycle.mounts).toBe(1);

    rerender(<BottomPanel visible={false} />);

    expect(screen.getByTestId("terminal-instance")).toBeTruthy();
    expect(terminalLifecycle.unmounts).toBe(0);

    rerender(<BottomPanel visible />);
    expect(terminalLifecycle.mounts).toBe(1);
    expect(terminalLifecycle.unmounts).toBe(0);
  });
});

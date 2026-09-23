// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import BottomPanel from "./BottomPanel";
import { OperateType } from "@/components/editable-tabs/types";
import { openAgentTerminal, useAgentTerminals } from "@/domain/chat/terminal/store";

vi.mock("@/domain/chat/terminal/AgentTerminalPanel", () => ({
  AgentTerminalTab: ({ terminalId }: { terminalId: string }) => <div data-testid={`agent-terminal-${terminalId}`} />,
}));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: vi.fn(async () => [
  { terminalId: "a", command: "npm", args: ["run", "dev"], cwd: "/projects/novel/front", running: true, exitCode: null },
  { terminalId: "b", command: "npm", args: ["run", "dev"], cwd: "/projects/axiom/front", running: false, exitCode: 1 },
]) }));
vi.mock("react-i18next", () => {
  const t = (key: string) => key;
  return { useTranslation: () => ({ t }) };
});

const terminalLifecycle = vi.hoisted(() => ({ mounts: 0, unmounts: 0 }));
const panelControls = vi.hoisted(() => ({
  collapse: vi.fn(),
  expand: vi.fn(),
  getSize: vi.fn(() => ({ inPixels: 250, inPercent: 25 })),
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
  EditableTabs: ({ items, activeKey, onChange, onEdit, renderAddTrigger, renderTrailingAction, createTab }: {
    items: Array<{ children: ReactNode; key: string; label: ReactNode }>;
    activeKey?: string;
    onChange: (key: string) => void;
    onEdit: (action: OperateType, info: { key: string; remainingTabs: typeof items }) => void;
    renderAddTrigger: (props: { onAdd: () => void }) => ReactNode;
    renderTrailingAction?: () => ReactNode;
    createTab: () => { key: string };
  }) => (
    <>
      <div role="tablist">
        {items.map((item) => (
          <span key={item.key}>
            <button role="tab" aria-selected={activeKey === item.key} onClick={() => onChange(item.key)}>{item.label}</button>
            <button aria-label={`close-${item.key}`} onClick={() => onEdit(OperateType.Remove, { key: item.key, remainingTabs: items.filter((tab) => tab.key !== item.key) })}>×</button>
          </span>
        ))}
        {renderAddTrigger({ onAdd: () => { const tab = createTab(); onChange(tab.key); } })}
      </div>
      <div data-testid="terminal-trailing-action">{renderTrailingAction?.()}</div>
      {items.map((item) => <div key={item.key}>{item.children}</div>)}
    </>
  ),
}));

vi.mock("@/components/ui/resizable", () => ({
  ResizableHandle: ({ className }: { className?: string }) => <div className={className} />,
  ResizablePanel: ({ children, maxSize }: { children: ReactNode; maxSize: string }) => <div data-testid="bottom-resizable-panel" data-max-size={maxSize}>{children}</div>,
}));

vi.mock("@/components/ui/dropdown-menu", () => ({
  DropdownMenu: ({ children }: { children: ReactNode }) => <>{children}</>,
  DropdownMenuTrigger: ({ render }: { render: ReactNode }) => render,
  DropdownMenuContent: ({ children }: { children: ReactNode }) => <div role="menu">{children}</div>,
  DropdownMenuItem: ({ children, onClick }: { children: ReactNode; onClick: () => void }) => <button role="menuitem" onClick={onClick}>{children}</button>,
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
    useAgentTerminals.setState({ sessions: {} });
    sessionStorage.clear();
    terminalLifecycle.mounts = 0;
    terminalLifecycle.unmounts = 0;
    panelControls.collapse.mockClear();
    panelControls.expand.mockClear();
    panelControls.getSize.mockClear();
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

  it("uses one panel for manual and agent tabs and keeps terminals mounted when hidden", async () => {
    const changeBottomPanel = vi.fn();
    const view = render(<BottomPanel visible sessionCode="chat-a" changeBottomPanel={changeBottomPanel} />);
    await screen.findByTestId("terminal-instance");
    expect(screen.getAllByTestId("bottom-resizable-panel")).toHaveLength(1);
    await waitFor(() => expect(useAgentTerminals.getState().sessions["chat-a"]?.terminals).toHaveLength(2));
    act(() => {
      openAgentTerminal("chat-a", "a");
      openAgentTerminal("chat-a", "b");
    });
    expect(screen.getByRole("tab", { name: "npm · novel/front" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "npm · axiom/front" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "npm · axiom/front" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.queryByRole("button", { name: "agentTerminal.collapsePanel" })).toBeNull();
    view.rerender(<BottomPanel visible={false} sessionCode="chat-a" changeBottomPanel={changeBottomPanel} />);
    expect(terminalLifecycle.unmounts).toBe(0);
    view.rerender(<BottomPanel visible sessionCode="chat-a" changeBottomPanel={changeBottomPanel} />);
    expect(screen.getAllByTestId("bottom-resizable-panel")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "close-agent-terminal:chat-a:b" }));
    expect(useAgentTerminals.getState().sessions["chat-a"].tabs).toEqual(["a"]);
  });

  it("maximizes from the trailing action and restores the previous height", async () => {
    const view = render(<BottomPanel visible />);
    await screen.findByTestId("terminal-instance");

    const trailingAction = screen.getByTestId("terminal-trailing-action");
    expect(trailingAction.querySelector('button[aria-label="agentTerminal.maximizePanel"]')).toBeTruthy();
    expect(screen.getByTestId("bottom-resizable-panel").getAttribute("data-max-size")).toBe("70%");

    fireEvent.click(screen.getByRole("button", { name: "agentTerminal.maximizePanel" }));
    expect(screen.getByTestId("bottom-resizable-panel").getAttribute("data-max-size")).toBe("100%");
    await waitFor(() => expect(panelControls.resize).toHaveBeenCalledWith("100%"));

    fireEvent.click(screen.getByRole("button", { name: "agentTerminal.restorePanel" }));
    expect(screen.getByTestId("bottom-resizable-panel").getAttribute("data-max-size")).toBe("70%");
    await waitFor(() => expect(panelControls.resize).toHaveBeenCalledWith("250px"));

    fireEvent.click(screen.getByRole("button", { name: "agentTerminal.maximizePanel" }));
    view.rerender(<BottomPanel visible={false} />);
    view.rerender(<BottomPanel visible />);
    expect(screen.getByRole("button", { name: "agentTerminal.maximizePanel" })).toBeTruthy();
    expect(screen.getByTestId("bottom-resizable-panel").getAttribute("data-max-size")).toBe("70%");
  });

  it("offers only running terminals in the app menu", async () => {
    render(<BottomPanel visible sessionCode="chat-a" />);
    await waitFor(() => expect(useAgentTerminals.getState().sessions["chat-a"]?.terminals).toHaveLength(2));
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(screen.getByRole("button", { name: "agentTerminal.open" })).toBeTruthy();
    const runningTerminal = screen.getByRole("menuitem", { name: /novel\/front/ });
    expect(screen.queryByRole("menuitem", { name: /axiom\/front/ })).toBeNull();
    fireEvent.click(runningTerminal);
    expect(useAgentTerminals.getState().sessions["chat-a"].tabs).toEqual(["a"]);
  });
});

// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AgentTerminalPanel } from "./AgentTerminalPanel";
import {
  AgentTerminalToolCard,
  AgentTerminalSessionContext,
} from "./AgentTerminalToolCard";
import {
  appendTerminalOutput,
  closeAgentTerminal,
  openAgentTerminal,
  useAgentTerminals,
} from "./store";
const api = vi.hoisted(() => vi.fn());
vi.mock("@/shared/api/httpClient", () => ({ requestApi: api }));
vi.mock("react-i18next", () => {
  const t = (key: string) => key;
  return { useTranslation: () => ({ t }) };
});
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    loadAddon() {}
    open() {}
    write() {}
    reset() {}
    dispose() {}
  },
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
  },
}));
const info = (id: string) => ({
  terminalId: id,
  command: `command-${id}`,
  args: [],
  cwd: `/work/${id}`,
  running: true,
  exitCode: null,
});
beforeEach(() => {
  sessionStorage.clear();
  useAgentTerminals.setState({ sessions: {} });
  api.mockReset();
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
  api.mockImplementation(async (endpoint: string) =>
    endpoint.endsWith("/terminals")
      ? [info(endpoint.includes("chat-a") ? "a" : "b")]
      : {
          ...info("a"),
          output: "hello",
          nextCursor: 5,
          truncated: false,
          outputClosed: true,
          running: false,
        },
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
describe("Agent terminal conversation lifecycle", () => {
  it("deduplicates tabs and restores selection and cursor independently for each chat without stopping processes", () => {
    openAgentTerminal("chat-a", "a");
    openAgentTerminal("chat-a", "a");
    openAgentTerminal("chat-b", "b");
    appendTerminalOutput("chat-a", "a", {
      ...info("a"),
      output: "hello",
      nextCursor: 5,
      truncated: false,
      outputClosed: false,
    });
    expect(useAgentTerminals.getState().sessions["chat-a"].tabs).toEqual(["a"]);
    closeAgentTerminal("chat-a", "a");
    expect(useAgentTerminals.getState().sessions["chat-a"].logs.a.cursor).toBe(
      5,
    );
    expect(useAgentTerminals.getState().sessions["chat-b"].selected).toBe("b");
    openAgentTerminal("chat-a", "a");
    useAgentTerminals.setState({ sessions: {} });
    useAgentTerminals.getState().update("chat-a", (value) => value);
    expect(useAgentTerminals.getState().sessions["chat-a"].selected).toBe("a");
    expect(api).not.toHaveBeenCalled();
  });
  it("shows only current chat tabs; switching, hiding and closing never send stop", async () => {
    openAgentTerminal("chat-a", "a");
    openAgentTerminal("chat-b", "b");
    const view = render(<AgentTerminalPanel sessionCode="chat-a" visible />);
    await screen.findByRole("tab", { name: "command-a" });
    view.rerender(<AgentTerminalPanel sessionCode="chat-b" visible />);
    await screen.findByRole("tab", { name: "command-b" });
    expect(screen.queryByRole("tab", { name: "command-a" })).toBeNull();
    view.rerender(<AgentTerminalPanel sessionCode="chat-a" visible={false} />);
    await screen.findByRole("tab", { name: "command-a" });
    fireEvent.click(
      screen.getByRole("button", { name: "agentTerminal.close" }),
    );
    expect(screen.queryByRole("tab")).toBeNull();
    expect(
      api.mock.calls.every(([url]) => !String(url).startsWith("POST")),
    ).toBe(true);
  });
  it("opens the same terminal from a tool card and stops it only on explicit stop", async () => {
    const listener = vi.fn();
    window.addEventListener("tron:open-agent-terminal", listener);
    render(
      <AgentTerminalSessionContext.Provider value="chat-a">
        <AgentTerminalToolCard
          card={{
            toolName: "terminal_start",
            args: {},
            result: JSON.stringify(info("a")),
            ended: true,
          }}
        />
      </AgentTerminalSessionContext.Provider>,
    );
    await screen.findByText("agentTerminal.running");
    expect(screen.getByText("agentTerminal.actionDone")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "agentTerminal.view" }));
    expect(useAgentTerminals.getState().sessions["chat-a"].tabs).toEqual(["a"]);
    expect(listener).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "agentTerminal.stop" }));
    await waitFor(() =>
      expect(api).toHaveBeenCalledWith(
        "POST /api/v1/chats/sessions/chat-a/terminals/a/stop",
      ),
    );
    window.removeEventListener("tron:open-agent-terminal", listener);
  });
  it("bounds the UI log cache", () => {
    act(() =>
      appendTerminalOutput("chat-a", "a", {
        ...info("a"),
        output: "x".repeat(300000),
        nextCursor: 300000,
        truncated: true,
        outputClosed: false,
      }),
    );
    expect(
      useAgentTerminals.getState().sessions["chat-a"].logs.a.text.length,
    ).toBe(262144);
    expect(
      useAgentTerminals.getState().sessions["chat-a"].logs.a.truncated,
    ).toBe(true);
  });
});

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
import { AgentTerminalTab } from "./AgentTerminalPanel";
import {
  AgentTerminalToolCard,
  AgentTerminalSessionContext,
} from "./AgentTerminalToolCard";
import {
  appendTerminalOutput,
  closeAgentTerminal,
  openAgentTerminal,
  openAgentTerminalFromLiveResult,
  useAgentTerminals,
} from "./store";
const api = vi.hoisted(() => vi.fn());
const terminalState = vi.hoisted(() => ({
  theme: "light",
  instances: [] as Array<{ options: { theme: Record<string, string> } }>,
}));
vi.mock("@/shared/api/httpClient", () => ({ requestApi: api }));
vi.mock("@/shared/theme/ThemeProvider", () => ({
  useTheme: () => ({ theme: terminalState.theme }),
}));
vi.mock("react-i18next", () => {
  const t = (key: string) => key;
  return { useTranslation: () => ({ t }) };
});
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    options: { theme: Record<string, string> };
    constructor(options: { theme: Record<string, string> }) {
      this.options = options;
      terminalState.instances.push(this);
    }
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
  terminalState.theme = "light";
  terminalState.instances = [];
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
  it("keeps agent output scoped to its chat; hiding the panel does not stop the process", async () => {
    openAgentTerminal("chat-a", "a");
    openAgentTerminal("chat-b", "b");
    const view = render(<AgentTerminalTab sessionCode="chat-a" terminalId="a" visible />);
    await waitFor(() => expect(api).toHaveBeenCalledWith(
      "/api/v1/chats/sessions/chat-a/terminals/a",
      expect.any(Object),
    ));
    view.rerender(<AgentTerminalTab sessionCode="chat-b" terminalId="b" visible />);
    await waitFor(() => expect(api).toHaveBeenCalledWith(
      "/api/v1/chats/sessions/chat-b/terminals/b",
      expect.any(Object),
    ));
    view.rerender(<AgentTerminalTab sessionCode="chat-b" terminalId="b" visible={false} />);
    closeAgentTerminal("chat-b", "b");
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
    expect(useAgentTerminals.getState().sessions["chat-a"].tabs).toEqual([]);
    expect(listener).toHaveBeenCalledTimes(0);
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
  it("opens a terminal only once for a successful live start result", () => {
    const seenCalls = new Set<string>();
    const event = {
      type: "TOOL_CALL_RESULT",
      runId: "run-1",
      toolCallId: "call-1",
      toolCallName: "terminal_start",
      content: JSON.stringify({ terminalId: "term-1", running: true }),
    };
    expect(openAgentTerminalFromLiveResult("chat-a", event, seenCalls)).toBe(true);
    expect(openAgentTerminalFromLiveResult("chat-a", event, seenCalls)).toBe(false);
    expect(useAgentTerminals.getState().sessions["chat-a"].tabs).toEqual([
      "term-1",
    ]);
    expect(
      openAgentTerminalFromLiveResult(
        "chat-a",
        { ...event, toolCallName: "terminal_read", toolCallId: "call-2" },
        seenCalls,
      ),
    ).toBe(false);
    expect(
      openAgentTerminalFromLiveResult(
        "chat-a",
        { ...event, toolCallId: "call-3", content: "command failed" },
        seenCalls,
      ),
    ).toBe(false);
  });
  it("updates the Agent terminal colors when the app theme changes", () => {
    const view = render(
      <AgentTerminalTab sessionCode="chat-a" terminalId="a" visible={false} />,
    );
    const terminal = terminalState.instances[0];
    expect(terminal.options.theme.background).toBe("#ffffff");
    expect(terminal.options.theme.foreground).toBe("#1c1b1b");
    act(() => {
      terminalState.theme = "matrix-dark";
      view.rerender(
        <AgentTerminalTab sessionCode="chat-a" terminalId="a" visible={false} />,
      );
    });
    expect(terminal.options.theme.background).toBe("#09150d");
    expect(terminal.options.theme.foreground).toBe("#e4ffe8");
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

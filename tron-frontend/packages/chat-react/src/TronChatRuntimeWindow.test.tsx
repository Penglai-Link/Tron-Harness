// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import type { ChatAdapter, ChatSession } from "@tron/chat-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TronChatRuntimeWindow } from "./TronChatRuntimeWindow";

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, "scrollTo", {
    configurable: true,
    value: vi.fn(),
    writable: true,
  });
});

function createAdapter() {
  const loadMessages = vi.fn().mockResolvedValue([]);

  const adapter: ChatAdapter = {
    capabilities: {
      supportsAbort: true,
      supportsAgui: false,
      supportsProtocolReplay: false,
      supportsStreaming: true,
    },
    createSession: vi.fn(),
    loadAguiEvents: vi.fn().mockResolvedValue([]),
    loadMessages,
    streamMessage: vi.fn(),
  };

  return { adapter, loadMessages };
}

function restoredSession(): ChatSession {
  return {
    agentCode: "main-agent",
    appCode: null,
    createdAt: "2026-06-10T00:00:00.000Z",
    id: 1,
    lastRunCode: null,
    lastRunStatus: null,
    sessionCode: "SESSION-RESTORED",
    status: "ACTIVE",
    systemAgentSessionId: null,
    taskCode: null,
    title: "Restored session",
    updatedAt: "2026-06-10T00:00:00.000Z",
    userCode: "user-1",
    workspaceCode: "WS-1",
  };
}

describe("TronChatRuntimeWindow", () => {
  it("synchronizes a later initialSession value into the runtime state", async () => {
    const { adapter, loadMessages } = createAdapter();
    const { rerender } = render(
      <TronChatRuntimeWindow
        adapter={adapter}
        config={{ floating: false, workspaceCode: "WS-1" }}
      />,
    );

    expect(screen.getByText("No session selected")).toBeTruthy();

    rerender(
      <TronChatRuntimeWindow
        adapter={adapter}
        config={{ floating: false, workspaceCode: "WS-1" }}
        initialSession={restoredSession()}
      />,
    );

    await waitFor(() => {
      expect(loadMessages).toHaveBeenCalledWith("SESSION-RESTORED");
    });

    expect(screen.queryByText("No session selected")).toBeNull();
    expect(screen.getByText((content) => content.includes("SESSION-RESTORED"))).toBeTruthy();
    expect(screen.queryByText("No persisted messages in this session.")).toBeNull();
  });
});

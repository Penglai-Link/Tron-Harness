// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";

import { createChatTerminalI18n } from "../i18n";
import { renderModelSession } from "./modelSession";

describe("renderModelSession agent activity", () => {
  it("renders reasoning summaries and grouped tool details without raw reasoning", () => {
    const host = document.createElement("div");
    const i18n = createChatTerminalI18n({ locale: "en-US" });

    renderModelSession(
      host,
      {
        activityEvents: [
          { runId: "run-1", timestamp: 1, type: "REASONING_START" },
          {
            delta: "internal chain of thought",
            runId: "run-1",
            timestamp: 2,
            type: "REASONING_TEXT_CONTENT",
          },
          { runId: "run-1", timestamp: 3, type: "REASONING_END" },
          {
            runId: "run-1",
            timestamp: 4,
            toolCallArgs: { path: "/workspace" },
            toolCallId: "tool-1",
            toolCallName: "workspace.inspect",
            type: "TOOL_CALL_START",
          },
          {
            delta: [{ op: "add", path: "/logs/-", value: "Loaded workspace" }],
            runId: "run-1",
            timestamp: 5,
            toolCallId: "tool-1",
            toolCallName: "workspace.inspect",
            type: "ACTIVITY_DELTA",
          },
          {
            content: "Workspace context loaded.",
            runId: "run-1",
            timestamp: 6,
            toolCallId: "tool-1",
            toolCallName: "workspace.inspect",
            type: "TOOL_CALL_RESULT",
          },
          { runId: "run-1", timestamp: 7, toolCallId: "tool-1", type: "TOOL_CALL_END" },
        ],
        commandStates: new Map(),
        i18n,
        tokens: [],
      },
      {
        onDraftCommandChange: vi.fn(),
        onExecuteCommand: vi.fn(),
        onCancelCommandEdit: vi.fn(),
        onRejectCommand: vi.fn(),
        onSaveCommandEdit: vi.fn(),
        onStartEditCommand: vi.fn(),
      },
    );

    expect(host.textContent).toContain("Reasoning captured");
    expect(host.textContent).toContain("Tools called 1");
    expect(host.textContent).toContain("workspace.inspect");
    expect(host.textContent).toContain("Loaded workspace");
    expect(host.textContent).toContain("Workspace context loaded.");
    expect(host.textContent).not.toContain("internal chain of thought");
  });
});

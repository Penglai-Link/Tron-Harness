// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";

import { createChatTerminalI18n } from "../i18n";
import { renderModelSession } from "./modelSession";

describe("renderModelSession agent activity", () => {
  it("renders an activity snapshot when reasoning cards have not started yet", () => {
    const host = document.createElement("div");
    const i18n = createChatTerminalI18n({ locale: "en-US" });

    renderModelSession(
      host,
      {
        activityEvents: [
          {
            activity: "THINKING",
            runId: "run-1",
            summary: "Inspecting the workspace.",
            timestamp: 1,
            type: "ACTIVITY_SNAPSHOT",
          },
        ],
        activityStartedAt: 0,
        activityNow: 3_000,
        activityComplete: false,
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

    expect(host.textContent).toContain("Thinking");
    expect(host.querySelector(".ai-terminal-model-session__activity-status--running")).toBeTruthy();
  });

  it("shows processed time after reasoning ends while the model is still running", () => {
    const host = document.createElement("div");
    const i18n = createChatTerminalI18n({ locale: "en-US" });

    renderModelSession(
      host,
      {
        activityEvents: [
          { runId: "run-1", timestamp: 1, type: "REASONING_START" },
          { delta: "planning", runId: "run-1", timestamp: 1.5, type: "REASONING_MESSAGE_CONTENT" },
          { runId: "run-1", timestamp: 2, type: "REASONING_END" },
        ],
        activityStartedAt: 0,
        activityNow: 5_000,
        activityComplete: false,
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

    expect(host.textContent).toContain("Processed 0m 5s");
    expect(host.textContent).not.toContain("Thinking");
    expect(host.querySelector(".ai-terminal-model-session__activity-status--running")).toBeTruthy();
  });

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
            timestamp: 0,
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
        activityStartedAt: 0,
        activityNow: 0,
        activityComplete: true,
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

    expect(host.textContent).toContain("Elapsed 0m 0s");
    expect(host.textContent).toContain("Tools called 1");
    expect(host.textContent).toContain("workspace.inspect");
    expect(host.textContent).not.toContain("Loaded workspace");
    expect(host.textContent).toContain("Workspace context loaded.");
    expect(host.textContent).not.toContain("internal chain of thought");
    expect(host.querySelector("summary svg")?.getAttribute("viewBox")).toBe("0 0 24 24");
    expect(host.querySelector("details")?.hasAttribute("open")).toBe(false);
    const summaries = [...host.querySelectorAll("summary")].map((summary) => summary.textContent ?? "");
    expect(summaries[0]).toContain("Elapsed 0m 0s");
    expect(summaries[1]).toContain("Tools called 1");
    expect(host.querySelector(".ai-terminal-model-session__activity-status--running")).toBeNull();
    expect(host.querySelectorAll(".ai-terminal-model-session__activity > details")).toHaveLength(1);
    expect(host.querySelector(".ai-terminal-model-session__activity-item--nested")).toBeTruthy();
    (host.querySelector("summary") as HTMLElement).click();
    expect(host.querySelector("details")?.open).toBe(true);
  });

  it("keeps tool details collapsed while a session is running", () => {
    const host = document.createElement("div");
    const i18n = createChatTerminalI18n({ locale: "en-US" });

    renderModelSession(
      host,
      {
        activityEvents: [
          {
            runId: "run-1",
            timestamp: 1,
            toolCallId: "tool-1",
            toolCallName: "workspace.inspect",
            type: "TOOL_CALL_START",
          },
        ],
        activityStartedAt: 0,
        activityNow: 1_000,
        activityComplete: false,
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

    (host.querySelector("summary") as HTMLElement).click();

    expect(host.querySelector("details")?.open).toBe(false);
  });

  it("renders one thinking state for repeated empty reasoning starts", () => {
    const host = document.createElement("div");
    const i18n = createChatTerminalI18n({ locale: "en-US" });

    renderModelSession(
      host,
      {
        activityEvents: [
          { runId: "run-1", timestamp: 1, type: "REASONING_START" },
          { runId: "run-1", timestamp: 2, type: "REASONING_START" },
        ],
        activityStartedAt: 0,
        activityNow: 2_000,
        activityComplete: false,
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

    expect(host.querySelectorAll("summary")).toHaveLength(1);
    expect(host.textContent).toContain("Thinking");
  });
});

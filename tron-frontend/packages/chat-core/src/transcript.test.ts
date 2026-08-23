import { describe, expect, it } from "vitest";

import {
  aguiReasoningEffort,
  aguiWorkRailsByMessageCode,
  buildAguiUserWorkCards,
  isRecentPendingUserTurn,
  openAguiProtocolRunCodes,
} from "./transcript";
import {
  aguiWorkElapsedMilliseconds,
  currentAguiWorkActivity,
  currentAguiWorkSnapshot,
  isAguiRunTerminal,
} from "./aguiWork";
import type {
  ChatAguiStreamEvent,
  ChatMessage,
} from "./types";

function userMessage(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    actor: "User",
    channel: "MAIN_AGENT",
    content: "hello",
    createdAt: "2026-06-10T00:00:00.000Z",
    id: 1,
    messageCode: "MSG-USER",
    role: "USER",
    sessionCode: "SESSION-1",
    ...overrides,
  };
}

function assistantMessage(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    actor: "Assistant",
    channel: "MAIN_AGENT",
    content: "reply",
    createdAt: "2026-06-10T00:00:02.000Z",
    id: 2,
    messageCode: "MSG-ASSISTANT",
    role: "ASSISTANT",
    runCode: "RUN-1",
    sessionCode: "SESSION-1",
    ...overrides,
  };
}

function event(overrides: Partial<ChatAguiStreamEvent> & Pick<ChatAguiStreamEvent, "type">): ChatAguiStreamEvent {
  return {
    runId: "RUN-1",
    threadId: "SESSION-1",
    timestamp: 1_000,
    ...overrides,
  };
}

describe("buildAguiUserWorkCards", () => {
  it("prefers structured reasoning content over raw reasoning fallback and groups tools", () => {
    const cards = buildAguiUserWorkCards([
      event({ type: "RUN_STARTED", timestamp: 1_000 }),
      event({ type: "REASONING_START", timestamp: 1_001 }),
      event({ type: "REASONING_TEXT_CONTENT", delta: "Raw reasoning stream segment", timestamp: 1_002 }),
      event({ type: "REASONING_MESSAGE_CONTENT", delta: "Structured summary", timestamp: 1_003 }),
      event({
        type: "TOOL_CALL_START",
        timestamp: 1_004,
        toolCallArgs: { path: "/tmp/demo.txt" },
        toolCallId: "tool-1",
        toolCallName: "fs.read",
      }),
      event({
        type: "TOOL_CALL_RESULT",
        content: "tool result",
        timestamp: 1_005,
        toolCallId: "tool-1",
        toolCallName: "fs.read",
      }),
      event({ type: "TOOL_CALL_END", timestamp: 1_006, toolCallId: "tool-1" }),
      event({ type: "REASONING_END", timestamp: 1_007 }),
    ]);

    expect(cards).toHaveLength(2);
    expect(cards[0]?.kind).toBe("reasoning");
    if (cards[0]?.kind !== "reasoning") throw new Error("expected reasoning card");
    expect(cards[0].value.content).toBe("Structured summary");
    expect(cards[0].value.toolCards).toHaveLength(1);
    expect(cards[1]?.kind).toBe("toolGroup");
    if (cards[1]?.kind !== "toolGroup") throw new Error("expected tool group card");
    expect(cards[1].value.toolCards[0]?.toolName).toBe("fs.read");
    expect(cards[1].value.toolCards[0]?.result).toBe("tool result");
  });

  it("uses raw reasoning fallback when no structured reasoning message is present", () => {
    const cards = buildAguiUserWorkCards([
      event({ type: "REASONING_START", timestamp: 2_000 }),
      event({ type: "REASONING_TEXT_CONTENT", delta: "fallback reasoning", timestamp: 2_001 }),
      event({ type: "REASONING_END", timestamp: 2_002 }),
    ]);

    expect(cards).toHaveLength(1);
    expect(cards[0]?.kind).toBe("reasoning");
    if (cards[0]?.kind !== "reasoning") throw new Error("expected reasoning card");
    expect(cards[0].value.content).toBe("fallback reasoning");
  });
});

describe("aguiReasoningEffort", () => {
  it("reads the effective effort from protocol and canonical replay events", () => {
    expect(aguiReasoningEffort([
      event({ type: "RUN_STARTED", reasoningEffort: "High" }),
    ])).toBe("High");
    expect(aguiReasoningEffort([{
      eventName: "agent.run.started",
      id: "1",
      metadata: { reasoningEffort: "Medium" },
      occurredAt: "2026-06-10T00:00:00.000Z",
      outcome: "STARTED",
      runCode: "RUN-1",
      streamKey: "SESSION-1",
      type: "agent.run.started",
    }])).toBe("Medium");
  });

  it("does not invent an effort for legacy runs", () => {
    expect(aguiReasoningEffort([event({ type: "RUN_STARTED" })])).toBeNull();
  });
});

describe("AG-UI current work activity", () => {
  it("uses a structured activity snapshot when one is available", () => {
    const events = [
      event({ type: "RUN_STARTED", timestamp: 1_000 }),
      event({ activity: "PREPARING", type: "ACTIVITY_SNAPSHOT", timestamp: 1_001 }),
      event({ type: "REASONING_MESSAGE_START", messageId: "reasoning-1", timestamp: 1_002 }),
      event({ type: "REASONING_MESSAGE_CONTENT", delta: "Reviewing the repository.", messageId: "reasoning-1", timestamp: 1_003 }),
      event({ type: "REASONING_MESSAGE_CONTENT", delta: "Preparing the implementation plan.", messageId: "reasoning-1", timestamp: 1_004 }),
      event({ activity: "THINKING", summary: "Preparing the implementation plan.", type: "ACTIVITY_SNAPSHOT", timestamp: 1_005 }),
    ];

    expect(currentAguiWorkActivity(events)).toBe("THINKING");
    expect(currentAguiWorkSnapshot(events)).toEqual({
      activity: "THINKING",
      summary: "Preparing the implementation plan.",
    });
    expect(
      buildAguiUserWorkCards(events)
        .filter((card) => card.kind === "reasoning")
        .map((card) => card.value.content),
    ).toEqual([
      "Reviewing the repository. Preparing the implementation plan.",
    ]);
    expect(aguiWorkElapsedMilliseconds(events, 3_005)).toBe(2_005);
    expect(isAguiRunTerminal(events)).toBe(false);

    const finished = [...events, event({ type: "RUN_FINISHED", timestamp: 3_500 })];
    expect(aguiWorkElapsedMilliseconds(finished, 9_000)).toBe(2_500);
    expect(isAguiRunTerminal(finished)).toBe(true);
  });

  it("derives one stable checkpoint from raw reasoning when no summary snapshot exists", () => {
    const events = [
      event({ type: "RUN_STARTED", timestamp: 1_000 }),
      event({ activity: "THINKING", type: "ACTIVITY_SNAPSHOT", timestamp: 1_001 }),
      event({ type: "REASONING_START", timestamp: 1_002 }),
      event({ type: "REASONING_TEXT_CONTENT", delta: "Inspecting the source code.", timestamp: 1_003 }),
      event({ type: "REASONING_TEXT_CONTENT", delta: " The next sentence is still streaming", timestamp: 1_004 }),
    ];

    expect(currentAguiWorkSnapshot(events)).toEqual({
      activity: "THINKING",
      summary: "Inspecting the source code.",
    });
  });

  it("omits a raw user-request restatement from the checkpoint", () => {
    const events = [
      event({ type: "REASONING_START", timestamp: 1_000 }),
      event({
        type: "REASONING_TEXT_CONTENT",
        delta: "The user is saying the task needs to continue. Reviewing the incomplete files.",
        timestamp: 1_001,
      }),
    ];

    expect(currentAguiWorkSnapshot(events)).toEqual({
      activity: "THINKING",
      summary: "Reviewing the incomplete files.",
    });
  });

  it("replaces the raw checkpoint only when a new reasoning turn begins", () => {
    const firstTurn = [
      event({ type: "REASONING_START", timestamp: 1_000 }),
      event({ type: "REASONING_TEXT_CONTENT", delta: "Inspecting the source code.", timestamp: 1_001 }),
      event({ type: "REASONING_END", timestamp: 1_002 }),
    ];
    const secondTurn = [
      ...firstTurn,
      event({ type: "REASONING_START", timestamp: 1_003 }),
      event({ type: "REASONING_TEXT_CONTENT", delta: "Planning", timestamp: 1_004 }),
    ];

    expect(currentAguiWorkSnapshot(firstTurn)).toEqual({
      activity: "THINKING",
      summary: "Inspecting the source code.",
    });
    expect(currentAguiWorkSnapshot(secondTurn)).toBeNull();
  });
});

describe("aguiWorkRailsByMessageCode", () => {
  it("anchors replay runs to the preceding user message", () => {
    const messages = [
      userMessage(),
      assistantMessage(),
    ];
    const rails = aguiWorkRailsByMessageCode(messages, [
      event({ type: "RUN_STARTED", timestamp: 3_000 }),
      event({ type: "REASONING_START", timestamp: 3_001 }),
      event({ type: "REASONING_TEXT_CONTENT", delta: "reasoning", timestamp: 3_002 }),
      event({ type: "REASONING_END", timestamp: 3_003 }),
      event({ type: "RUN_FINISHED", timestamp: 3_004 }),
    ], {}, 5);

    expect(rails.get("MSG-USER")).toHaveLength(1);
    expect(rails.get("MSG-USER")?.[0]?.runCode).toBe("RUN-1");
  });
});

describe("pending run helpers", () => {
  it("detects recent pending user turns and ignores stale ones", () => {
    expect(isRecentPendingUserTurn([
      userMessage({ createdAt: "2026-06-10T00:29:59.000Z" }),
    ], Date.parse("2026-06-10T00:30:00.000Z"))).toBe(true);

    expect(isRecentPendingUserTurn([
      userMessage({ createdAt: "2026-06-09T23:00:00.000Z" }),
    ], Date.parse("2026-06-10T00:30:01.000Z"))).toBe(false);
  });

  it("tracks only replay runs that have not reached a terminal event", () => {
    const openRuns = openAguiProtocolRunCodes([
      event({ runId: "RUN-OPEN", type: "RUN_STARTED", timestamp: 4_000 }),
      event({ runId: "RUN-OPEN", type: "REASONING_START", timestamp: 4_001 }),
      event({ runId: "RUN-CLOSED", type: "RUN_STARTED", timestamp: 4_002 }),
      event({ runId: "RUN-CLOSED", type: "RUN_FINISHED", timestamp: 4_003 }),
    ], "SESSION-1");

    expect(openRuns.has("RUN-OPEN")).toBe(true);
    expect(openRuns.has("RUN-CLOSED")).toBe(false);
  });
});

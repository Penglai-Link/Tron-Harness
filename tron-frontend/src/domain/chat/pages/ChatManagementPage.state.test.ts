// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";

vi.hoisted(() => {
  Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
    value: () => null,
  });
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      localStorage: {
        getItem: () => null,
        setItem: () => undefined,
      },
      location: {
        origin: "http://localhost",
        pathname: "/chats",
        search: "",
      },
    },
  });
});

import {
  ChatStreamOwnershipRegistry,
  buildContextMemoryBlocks,
  claimChatRunTerminalDrain,
  defaultChatModelOption,
  latestTerminalChatRunCode,
  latestContextBudgetBreakdownFromEvents,
  latestShortTermMemoryRuntimeNotice,
  normalizeContextBudgetBreakdown,
  resolveChatStreamSettlement,
  shouldPollChatRecoveryData,
  shouldPollChatRunRecovery,
} from "./ChatManagementPage";

describe("chat model default selection", () => {
  const fixedDefault = {
    enabled: true,
    isDefault: true,
    modelSelection: { kind: "Fixed" as const, modelCode: "MODEL-1" },
    source: "internal-agent" as const,
  };
  const autoRoute = {
    enabled: true,
    isDefault: false,
    modelSelection: {
      kind: "ProviderRoute" as const,
      providerCode: "PersonAi" as const,
      routeCode: "ChatAuto" as const,
    },
    source: "internal-agent" as const,
  };

  it("prefers enabled Person-AI Auto when a fixed model is marked as default", () => {
    expect(defaultChatModelOption([fixedDefault, autoRoute])).toBe(autoRoute);
  });

  it("keeps the configured model default when the Auto route is unavailable", () => {
    expect(defaultChatModelOption([
      fixedDefault,
      { ...autoRoute, enabled: false },
    ])).toBe(fixedDefault);
  });
});

describe("tool schema context budget", () => {
  it("localizes every context memory block name and avoids duplicate labels", () => {
    const blocks = buildContextMemoryBlocks({
      breakdown: null,
      requestEstimate: null,
      translate: (key) => ({
        "chat.shortTermMemory.contextPanel.systemPromptName": "系统提示词",
        "chat.shortTermMemory.contextPanel.toolSchemaName": "工具定义",
        "chat.shortTermMemory.contextPanel.checkpointName": "当前会话检查点",
        "chat.shortTermMemory.contextPanel.workingStateName": "当前会话工作状态",
        "chat.shortTermMemory.contextPanel.familyContextName": "会话族上下文",
        "chat.shortTermMemory.contextPanel.sessionHistoryName": "当前会话历史",
      }[key] ?? key),
    });

    expect(blocks.map((block) => block.name)).toEqual([
      "系统提示词",
      "工具定义",
      "当前会话检查点",
      "当前会话工作状态",
      "会话族上下文",
      "当前会话历史",
    ]);
    expect(blocks.every((block) => !("meta" in block))).toBe(true);
  });

  it.each([6_000, 0])("uses the independent schema limit %s", (limit) => {
    const breakdown = normalizeContextBudgetBreakdown({
      estimatedInputTokens: 10_000,
      effectiveInputBudget: 28_000,
      toolSchemaBudgetTokens: limit,
      toolSchemaTokens: 1_000,
    });
    const blocks = buildContextMemoryBlocks({
      breakdown,
      requestEstimate: null,
      translate: (key) => key,
    });
    expect(blocks.find((block) => block.color === "tool")).toMatchObject({
      limitTokens: limit,
      tokens: 1_000,
    });
  });

  it("keeps the shared-budget fallback for historical records", () => {
    const breakdown = normalizeContextBudgetBreakdown({
      estimatedInputTokens: 10_000,
      effectiveInputBudget: 28_000,
    });
    const blocks = buildContextMemoryBlocks({
      breakdown,
      requestEstimate: null,
      translate: (key) => key,
    });
    expect(blocks.find((block) => block.color === "tool")?.limitTokens).toBe(28_000);
  });

  it("reads a live context budget from an AG-UI custom event", () => {
    const breakdown = latestContextBudgetBreakdownFromEvents([{
      name: "tron.short_term_memory.context.assembled",
      threadId: "SESSION-1",
      timestamp: 100,
      type: "CUSTOM",
      value: {
        budgetBreakdown: {
          estimatedInputTokens: 12_000,
          effectiveInputBudget: 28_000,
          toolSchemaBudgetTokens: 6_000,
          toolSchemaTokens: 3_000,
        },
      },
    }]);

    expect(breakdown).toMatchObject({
      estimatedInputTokens: 12_000,
      toolSchemaBudgetTokens: 6_000,
      toolSchemaTokens: 3_000,
    });
  });

  it("shows the latest automatic compaction lifecycle from AG-UI", () => {
    expect(latestShortTermMemoryRuntimeNotice([
      {
        name: "tron.short_term_memory.compaction.started",
        timestamp: 100,
        type: "CUSTOM",
        value: { compactionInputTokenEstimate: 24_000, maxInputTokens: 28_000 },
      },
      {
        name: "tron.short_term_memory.compaction.completed",
        timestamp: 200,
        type: "CUSTOM",
        value: { checkpointMessageCode: "ATM-1" },
      },
    ])).toMatchObject({
      kind: "compaction_completed",
      timestamp: 200,
    });
  });

  it("keeps a tool-schema budget error until a later run succeeds", () => {
    const error = {
      code: "tool_schema_over_budget",
      message: "TOOL_SCHEMA_OVER_BUDGET: estimatedTokens=27294, limitTokens=19200, toolCount=93; tool exposure was not changed",
      timestamp: 100,
      type: "RUN_ERROR",
    };
    expect(latestShortTermMemoryRuntimeNotice([error])).toMatchObject({
      estimatedTokens: 27_294,
      kind: "tool_schema_over_budget",
      limitTokens: 19_200,
      toolCount: 93,
    });
    expect(latestShortTermMemoryRuntimeNotice([
      error,
      { timestamp: 200, type: "RUN_FINISHED" },
    ])).toBeNull();
  });
});

describe("chat page run recovery state", () => {
  it("keeps polling after a 409 queues the message even when status and replay are unresolved", () => {
    expect(shouldPollChatRunRecovery({
      currentRunActive: false,
      hasQueuedSubmission: true,
      sessionCode: "SESSION-1",
      shouldPollData: false,
    })).toBe(true);
  });

  it("does not poll an idle session without queued recovery work", () => {
    expect(shouldPollChatRunRecovery({
      currentRunActive: false,
      hasQueuedSubmission: false,
      sessionCode: "SESSION-1",
      shouldPollData: false,
    })).toBe(false);
  });

  it("stops polling when recovery confirms that every active state is empty", () => {
    expect(shouldPollChatRecoveryData({
      activeRunConfirmedIdle: true,
      hasActiveRun: false,
      hasOpenReplayRun: false,
      hasPendingUserInput: false,
      hasQueuedSubmission: false,
    })).toBe(false);
  });

  it("continues polling for a queued message or pending user input", () => {
    expect(shouldPollChatRecoveryData({
      activeRunConfirmedIdle: false,
      hasActiveRun: false,
      hasOpenReplayRun: false,
      hasPendingUserInput: false,
      hasQueuedSubmission: true,
    })).toBe(true);
    expect(shouldPollChatRecoveryData({
      activeRunConfirmedIdle: false,
      hasActiveRun: false,
      hasOpenReplayRun: false,
      hasPendingUserInput: true,
      hasQueuedSubmission: false,
    })).toBe(true);
  });

  it("stops polling when active-run status confirms idle despite stale replay events", () => {
    expect(shouldPollChatRecoveryData({
      activeRunConfirmedIdle: true,
      hasActiveRun: false,
      hasOpenReplayRun: true,
      hasPendingUserInput: false,
      hasQueuedSubmission: false,
    })).toBe(false);
  });

  it("finds the newest protocol terminal even when the active run was missed", () => {
    expect(latestTerminalChatRunCode([
      {
        runId: "RUN-NEW",
        threadId: "SESSION-1",
        timestamp: 20,
        type: "RUN_FINISHED",
      },
      {
        runId: "RUN-OLD",
        threadId: "SESSION-1",
        timestamp: 10,
        type: "RUN_ERROR",
      },
      {
        runId: "RUN-OTHER",
        threadId: "SESSION-2",
        timestamp: 30,
        type: "RUN_FINISHED",
      },
    ], "SESSION-1")).toBe("RUN-NEW");
  });

  it("claims a recovered terminal only once so one effect cycle drains one FIFO item", () => {
    const drainedRunKeys = new Set<string>();

    expect(claimChatRunTerminalDrain(
      drainedRunKeys,
      "SESSION-1",
      "RUN-1",
    )).toBe(true);
    expect(claimChatRunTerminalDrain(
      drainedRunKeys,
      "SESSION-1",
      "RUN-1",
    )).toBe(false);
  });
});

describe("chat page stream ownership", () => {
  it("detaches the old session and ignores its stale finally after a new owner starts", () => {
    const registry = new ChatStreamOwnershipRegistry();
    const oldController = new AbortController();
    const oldOwner = registry.acquire("SESSION-OLD", oldController);

    expect(registry.detach("SESSION-OLD")).toBe(oldOwner);
    oldController.abort();
    expect(oldController.signal.aborted).toBe(true);

    const newController = new AbortController();
    const newOwner = registry.acquire("SESSION-NEW", newController);

    expect(registry.release(oldOwner)).toBe(false);
    expect(registry.current).toBe(newOwner);
    expect(registry.owns(newOwner)).toBe(true);
    expect(newController.signal.aborted).toBe(false);
  });

  it("does not detach an observer owned by another session", () => {
    const registry = new ChatStreamOwnershipRegistry();
    const owner = registry.acquire("SESSION-1", new AbortController());

    expect(registry.detach("SESSION-2")).toBeNull();
    expect(registry.current).toBe(owner);
  });

  it("does not requeue a source already admitted before explicit Stop", () => {
    const registry = new ChatStreamOwnershipRegistry();
    const owner = registry.acquire("SESSION-1", new AbortController());
    registry.release(owner);

    expect(resolveChatStreamSettlement({
      activeSessionCode: "SESSION-1",
      detached: false,
      owner,
      registry,
      stopped: true,
    })).toEqual({
      canCommit: false,
      restoreSourceSubmission: false,
    });
  });

  it("does not restore or commit a detached stream settlement", () => {
    const registry = new ChatStreamOwnershipRegistry();
    const owner = registry.acquire("SESSION-1", new AbortController());
    registry.detach("SESSION-1");

    expect(resolveChatStreamSettlement({
      activeSessionCode: "SESSION-1",
      detached: true,
      owner,
      registry,
      stopped: false,
    })).toEqual({
      canCommit: false,
      restoreSourceSubmission: false,
    });
  });

  it("blocks a stale resolve before the session-switch detach effect runs", () => {
    const registry = new ChatStreamOwnershipRegistry();
    const owner = registry.acquire("SESSION-OLD", new AbortController());

    expect(resolveChatStreamSettlement({
      activeSessionCode: "SESSION-NEW",
      detached: false,
      owner,
      registry,
      stopped: false,
    })).toEqual({
      canCommit: false,
      restoreSourceSubmission: false,
    });
  });
});

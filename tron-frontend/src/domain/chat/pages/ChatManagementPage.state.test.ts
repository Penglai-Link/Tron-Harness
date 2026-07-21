import { describe, expect, it, vi } from "vitest";

vi.hoisted(() => {
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
  claimChatRunTerminalDrain,
  latestTerminalChatRunCode,
  resolveChatStreamSettlement,
  shouldPollChatRunRecovery,
} from "./ChatManagementPage";

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

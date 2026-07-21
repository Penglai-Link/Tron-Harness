import { describe, expect, it } from "vitest";

import type { ChatAguiStreamEvent } from "./ChatModel";
import {
  isChatReplayRunTerminal,
  resolveLatestOpenChatReplayRun,
} from "./chatReplayRun";

function event(
  type: string,
  runId: string,
  timestamp: number,
  extra: Partial<ChatAguiStreamEvent> = {},
): ChatAguiStreamEvent {
  return {
    threadId: "SESSION-1",
    timestamp,
    type,
    runId,
    ...extra,
  };
}

describe("chat replay run recovery", () => {
  it("chooses the latest open run by timestamp even when replay input is out of order", () => {
    const events = [
      event("RUN_STARTED", "RUN-2", 20, { supportsSteer: true }),
      event("RUN_STARTED", "RUN-1", 10),
    ];

    expect(resolveLatestOpenChatReplayRun(events, "SESSION-1")).toEqual({
      runCode: "RUN-2",
      supportsSteer: true,
    });
  });

  it.each(["RUN_FINISHED", "RUN_ERROR"])(
    "treats %s as terminal after timestamp ordering",
    (terminalType) => {
      const events = [
        event(terminalType, "RUN-1", 30),
        event("RUN_STARTED", "RUN-1", 10, { supportsSteer: true }),
      ];

      expect(resolveLatestOpenChatReplayRun(events, "SESSION-1")).toBeNull();
      expect(isChatReplayRunTerminal(events, "SESSION-1", "RUN-1")).toBe(true);
    },
  );

  it("ignores other sessions and preserves an explicit unsupported capability", () => {
    const events = [
      event("RUN_STARTED", "RUN-OTHER", 30, { threadId: "SESSION-2" }),
      event("RUN_STARTED", "RUN-1", 20, { supportsSteer: false }),
    ];

    expect(resolveLatestOpenChatReplayRun(events, "SESSION-1")).toEqual({
      runCode: "RUN-1",
      supportsSteer: false,
    });
  });

  it("uses provider order when any replay event has no timestamp", () => {
    const events = [
      { ...event("RUN_STARTED", "RUN-1", 10), timestamp: undefined },
      event("RUN_FINISHED", "RUN-1", 5),
    ];

    expect(resolveLatestOpenChatReplayRun(events, "SESSION-1")).toBeNull();
  });
});

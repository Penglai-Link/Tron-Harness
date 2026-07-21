import { describe, expect, it } from "vitest";

import type { ActiveChatRun } from "./ChatModel";
import { resolveChatRunUiState } from "./chatRunUiState";

function authoritativeRun(
  state: string,
  supportsSteer: boolean,
): ActiveChatRun {
  return {
    runCode: "RUN-1",
    runtimeAdapterKey: "main-agent",
    state,
    supportsSteer,
  };
}

describe("chat run UI state", () => {
  it("offers steer and queue for an authoritative steerable RUNNING run", () => {
    expect(resolveChatRunUiState({
      authoritativeRun: authoritativeRun("RUNNING", true),
      localRunActive: false,
      localRunCode: "",
      localRunSupportsSteer: null,
      replayRunHint: null,
    })).toEqual({
      active: true,
      canSteer: true,
      canStop: true,
      runCode: "RUN-1",
      runningSubmitModes: ["steer", "queue"],
    });
  });

  it("offers queue only while the authoritative run is FINISHING", () => {
    expect(resolveChatRunUiState({
      authoritativeRun: authoritativeRun("FINISHING", true),
      localRunActive: false,
      localRunCode: "",
      localRunSupportsSteer: null,
      replayRunHint: null,
    })).toMatchObject({
      active: true,
      canSteer: false,
      canStop: false,
      runningSubmitModes: ["queue"],
    });
  });

  it("uses an authoritative run code while the local stream is still starting", () => {
    expect(resolveChatRunUiState({
      authoritativeRun: authoritativeRun("RUNNING", true),
      localRunActive: true,
      localRunCode: "",
      localRunSupportsSteer: null,
      replayRunHint: null,
    })).toMatchObject({
      canSteer: true,
      canStop: true,
      runCode: "RUN-1",
      runningSubmitModes: ["steer", "queue"],
    });
  });

  it("returns idle state when the backend reports no active run", () => {
    expect(resolveChatRunUiState({
      authoritativeRun: null,
      localRunActive: false,
      localRunCode: "",
      localRunSupportsSteer: null,
      replayRunHint: null,
    })).toEqual({
      active: false,
      canSteer: false,
      canStop: false,
      runCode: "",
      runningSubmitModes: [],
    });
  });

  it("uses replay only as a queue-only loading hint", () => {
    expect(resolveChatRunUiState({
      authoritativeRun: null,
      localRunActive: false,
      localRunCode: "",
      localRunSupportsSteer: null,
      replayRunHint: { runCode: "RUN-REPLAY", supportsSteer: true },
    })).toEqual({
      active: true,
      canSteer: false,
      canStop: false,
      runCode: "RUN-REPLAY",
      runningSubmitModes: ["queue"],
    });
  });
});

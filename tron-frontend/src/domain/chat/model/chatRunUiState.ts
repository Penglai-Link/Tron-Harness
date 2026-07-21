import type { ChatRunningSubmitMode } from "@tron/chat-core";

import type { ActiveChatRun } from "./ChatModel";
import type { ChatReplayRun } from "./chatReplayRun";

export interface ResolveChatRunUiStateInput {
  authoritativeRun: ActiveChatRun | null;
  localRunActive: boolean;
  localRunCode: string;
  localRunSupportsSteer: boolean | null;
  replayRunHint: ChatReplayRun | null;
}

export interface ChatRunUiState {
  active: boolean;
  canSteer: boolean;
  canStop: boolean;
  runCode: string;
  runningSubmitModes: ChatRunningSubmitMode[];
}

export function resolveChatRunUiState({
  authoritativeRun,
  localRunActive,
  localRunCode,
  localRunSupportsSteer,
  replayRunHint,
}: ResolveChatRunUiStateInput): ChatRunUiState {
  const authoritativeRunning = Boolean(
    authoritativeRun
    && authoritativeRun.state.toUpperCase() !== "FINISHING",
  );
  const active = localRunActive
    || Boolean(authoritativeRun)
    || Boolean(replayRunHint);
  const runCode = localRunCode
    || authoritativeRun?.runCode
    || replayRunHint?.runCode
    || "";
  const authoritativeMatchesLocal = Boolean(
    authoritativeRun
    && (!localRunCode || authoritativeRun.runCode === localRunCode),
  );
  const supportsSteer = Boolean(
    localRunActive
      ? localRunSupportsSteer === true
        || (
          localRunSupportsSteer !== false
          && authoritativeMatchesLocal
          && authoritativeRunning
          && authoritativeRun?.supportsSteer
        )
      : authoritativeRunning && authoritativeRun?.supportsSteer,
  );
  const canSteer = Boolean(active && runCode && supportsSteer);
  const canStop = Boolean(
    runCode
    && (
      authoritativeRunning
      || (
        localRunActive
        && localRunCode
        && !(
          authoritativeMatchesLocal
          && authoritativeRun?.state.toUpperCase() === "FINISHING"
        )
      )
    ),
  );

  return {
    active,
    canSteer,
    canStop,
    runCode,
    runningSubmitModes: active
      ? (canSteer ? ["steer", "queue"] : ["queue"])
      : [],
  };
}

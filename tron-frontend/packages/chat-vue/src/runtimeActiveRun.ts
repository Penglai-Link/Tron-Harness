import type {
  ChatActiveRun,
  ChatAguiStreamEvent,
} from "@tron/chat-core";

export interface RuntimeActiveRunTrackingState {
  activeRun: ChatActiveRun | null;
  pendingProtocolTerminalRun: ChatActiveRun | null;
  statusPending: boolean;
}

export interface RuntimeActiveRunResolution {
  consumedSuppressedRunCode?: string;
  shouldDrainQueue: boolean;
  state: RuntimeActiveRunTrackingState;
}

function eventRunCode(event: ChatAguiStreamEvent) {
  return String(event.runId ?? event.runCode ?? "");
}

export function hasProtocolTerminalForRun(
  events: ChatAguiStreamEvent[],
  sessionCode: string,
  runCode: string,
) {
  return events.some((event) => {
    const type = String(event.type).toUpperCase();
    if (type !== "RUN_FINISHED" && type !== "RUN_ERROR") return false;
    if (eventRunCode(event) !== runCode) return false;
    const eventSessionCode = String(event.threadId ?? "");
    return !eventSessionCode || eventSessionCode === sessionCode;
  });
}

export function retainActiveRunOnStatusFailure(
  state: RuntimeActiveRunTrackingState,
): RuntimeActiveRunTrackingState {
  return {
    ...state,
    statusPending: true,
  };
}

export function resolveSuccessfulActiveRunSnapshot(args: {
  nextActiveRun: ChatActiveRun | null;
  protocolEvents: ChatAguiStreamEvent[];
  protocolReplayEnabled: boolean;
  sessionCode: string;
  state: RuntimeActiveRunTrackingState;
  suppressedRunCodes: ReadonlySet<string>;
}): RuntimeActiveRunResolution {
  const {
    nextActiveRun,
    protocolEvents,
    protocolReplayEnabled,
    sessionCode,
    state,
    suppressedRunCodes,
  } = args;

  if (nextActiveRun) {
    return {
      shouldDrainQueue: false,
      state: {
        activeRun: nextActiveRun,
        pendingProtocolTerminalRun: null,
        statusPending: false,
      },
    };
  }

  const previousActiveRun = state.activeRun;
  if (previousActiveRun) {
    if (suppressedRunCodes.has(previousActiveRun.runCode)) {
      return {
        consumedSuppressedRunCode: previousActiveRun.runCode,
        shouldDrainQueue: false,
        state: {
          activeRun: null,
          pendingProtocolTerminalRun: null,
          statusPending: false,
        },
      };
    }
    if (
      protocolReplayEnabled &&
      !hasProtocolTerminalForRun(
        protocolEvents,
        sessionCode,
        previousActiveRun.runCode,
      )
    ) {
      return {
        shouldDrainQueue: false,
        state: {
          activeRun: null,
          pendingProtocolTerminalRun: previousActiveRun,
          statusPending: false,
        },
      };
    }
    return {
      shouldDrainQueue: true,
      state: {
        activeRun: null,
        pendingProtocolTerminalRun: null,
        statusPending: false,
      },
    };
  }

  const pendingProtocolTerminalRun = state.pendingProtocolTerminalRun;
  if (pendingProtocolTerminalRun) {
    const terminalObserved =
      !protocolReplayEnabled ||
      hasProtocolTerminalForRun(
        protocolEvents,
        sessionCode,
        pendingProtocolTerminalRun.runCode,
      );
    return {
      shouldDrainQueue: terminalObserved,
      state: {
        activeRun: null,
        pendingProtocolTerminalRun: terminalObserved
          ? null
          : pendingProtocolTerminalRun,
        statusPending: terminalObserved ? false : state.statusPending,
      },
    };
  }

  return {
    shouldDrainQueue: state.statusPending,
    state: {
      activeRun: null,
      pendingProtocolTerminalRun: null,
      statusPending: false,
    },
  };
}

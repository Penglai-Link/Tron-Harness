<script setup lang="ts">
import {
  buildTranscriptPresentation,
  createMockChatAdapter,
  enqueueChatSubmission,
  FetchTransportError,
  mergeChatMessagesChronologically,
  mergeTranscriptAguiEvents,
  normalizeChatSession,
  removeQueuedChatSubmission,
  takeNextQueuedChatSubmission,
  type ChatActiveRun,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatFloatingState,
  type ChatMessage,
  type ChatQueuedSubmission,
  type ChatRunningSubmitMode,
  type ChatSession,
  type ChatSubmitMode,
  type ChatUserInputAnswer,
  type ChatUserInputRequest,
  type SendChatMessageInput,
  isRecentPendingUserTurn,
  isSteerQueueFallbackCode,
  openAguiProtocolRunCodes,
} from "@tron/chat-core";
import {
  computed,
  onBeforeUnmount,
  ref,
  shallowRef,
  useSlots,
  watch,
} from "vue";
import { defaultErrorReporter, normalizeOptionalValue } from "../utils";
import {
  resolveSuccessfulActiveRunSnapshot,
  retainActiveRunOnStatusFailure,
  type RuntimeActiveRunTrackingState,
} from "../runtimeActiveRun";
import {
  ownsRuntimeStreamObserver,
  resolveRuntimeStreamSettlement,
  type RuntimeStreamAbortReason,
} from "../runtimeStreamObserver";
import TronChatPanel from "./TronChatPanel.vue";
import TronChatWindow from "./TronChatWindow.vue";
import { TronChatRuntimeWindowProps } from "../types";

const props = withDefaults(defineProps<TronChatRuntimeWindowProps>(), {
  adapter: undefined,
  currentUserDisplayName: "",
  debugOptions: undefined,
  floatingSubtitle: "Workspace assistant",
  floatingTitle: "Main Agent",
  initialSession: null,
  onError: defaultErrorReporter,
  onInfo: undefined,
  onSessionChange: undefined,
  onWindowStateChange: undefined,
  windowState: undefined,
});

const emit = defineEmits<{
  error: [message: string, title?: string];
  info: [message: string, title?: string];
  "session-change": [session: ChatSession | null];
  "window-state-change": [state: ChatFloatingState];
}>();

const forwardedSlotNames: string[] = Object.keys(useSlots());

const activeAdapter = computed(() => props.adapter ?? createMockChatAdapter());
const normalizedInitialSession = computed(() =>
  props.initialSession ? normalizeChatSession(props.initialSession) : null,
);
const session = ref<ChatSession | null>(normalizedInitialSession.value);
const messages = ref<ChatMessage[]>([]);
const userInputRequest = ref<ChatUserInputRequest | null>(null);
const userInputAnswerPending = ref(false);
const aguiEvents = ref<ChatAguiEvent[]>([]);
const protocolEvents = ref<ChatAguiStreamEvent[]>([]);
const liveAguiEvents = ref<ChatAguiStreamEvent[]>([]);
const liveAguiAnchorMessageCodes = ref<Record<string, string>>({});
const draft = ref("");
const sending = ref(false);
const steering = ref(false);
const loading = ref(false);
const liveAguiPending = ref(false);
const activeRunCode = ref("");
const activeRunSupportsSteer = ref<boolean | null>(null);
const remoteActiveRun = ref<ChatActiveRun | null>(null);
const pendingProtocolTerminalRun = ref<ChatActiveRun | null>(null);
const activeRunStatusPending = ref(false);
const stopping = ref(false);
const queuedSubmissions = ref<ChatQueuedSubmission[]>([]);
const activeSessionCodeRef = ref("");
const activeObserverRef = shallowRef<{
  abortReason: RuntimeStreamAbortReason;
  controller: AbortController;
  sessionCode: string;
} | null>(null);
const steerAbortRef = ref<AbortController | null>(null);
const stopAbortRef = ref<AbortController | null>(null);
const autoDrainEpochRef = ref(0);
const queuedSubmissionSequenceRef = ref(0);
const optimisticMessageSequenceRef = ref(0);
const queueGenerationRef = ref(0);
const terminalHandledRunCodesRef = new Set<string>();

const aguiEnabled = computed(
  () =>
    props.config.showAgui !== false &&
    activeAdapter.value.capabilities.supportsAgui,
);
const protocolReplayEnabled = computed(
  () =>
    aguiEnabled.value &&
    activeAdapter.value.capabilities.supportsProtocolReplay &&
    Boolean(activeAdapter.value.loadProtocolEvents),
);
const effectiveActiveRunCode = computed(
  () =>
    activeRunCode.value ||
    remoteActiveRun.value?.runCode ||
    pendingProtocolTerminalRun.value?.runCode ||
    "",
);
const runActive = computed(
  () =>
    sending.value ||
    Boolean(remoteActiveRun.value) ||
    Boolean(pendingProtocolTerminalRun.value) ||
    activeRunStatusPending.value,
);
const effectiveRunState = computed(() => {
  if (
    pendingProtocolTerminalRun.value?.runCode === effectiveActiveRunCode.value
  ) {
    return "FINISHING";
  }
  return remoteActiveRun.value?.runCode === effectiveActiveRunCode.value
    ? remoteActiveRun.value.state
    : "RUNNING";
});
const effectiveSupportsSteer = computed(() => {
  if (
    pendingProtocolTerminalRun.value?.runCode === effectiveActiveRunCode.value
  ) {
    return false;
  }
  return remoteActiveRun.value?.runCode === effectiveActiveRunCode.value
    ? remoteActiveRun.value.supportsSteer
    : activeRunSupportsSteer.value;
});
const canSteer = computed(() =>
  Boolean(
    activeAdapter.value.capabilities.supportsSteer &&
      activeAdapter.value.steerMessage &&
      effectiveActiveRunCode.value &&
      effectiveRunState.value === "RUNNING" &&
      effectiveSupportsSteer.value !== false,
  ),
);
const runningSubmitModes = computed<ChatRunningSubmitMode[]>(() =>
  canSteer.value ? ["steer", "queue"] : ["queue"],
);
const visibleQueuedSubmissions = computed(() =>
  queuedSubmissions.value.filter(
    (submission) => submission.sessionCode === session.value?.sessionCode,
  ),
);
const canStop = computed(() =>
  Boolean(
    session.value &&
      !stopping.value &&
      ((effectiveActiveRunCode.value &&
        effectiveRunState.value === "RUNNING" &&
        activeAdapter.value.capabilities.supportsStop &&
        activeAdapter.value.stopRun) ||
        (sending.value &&
          activeAdapter.value.capabilities.supportsAbort &&
          !(
            activeAdapter.value.capabilities.supportsStop &&
            activeAdapter.value.stopRun
          ))),
  ),
);

function isRunAlreadyActiveError(error: unknown) {
  return (
    (error instanceof FetchTransportError &&
      error.code === "CHAT_RUN_ALREADY_ACTIVE") ||
    (typeof error === "object" &&
      error !== null &&
      "code" in error &&
      error.code === "CHAT_RUN_ALREADY_ACTIVE")
  );
}

function detachRunObserver(reason: "detach" | "stop") {
  const observer = activeObserverRef.value;
  if (observer) {
    observer.abortReason = reason;
    observer.controller.abort();
    if (activeObserverRef.value === observer) {
      activeObserverRef.value = null;
      sending.value = false;
      liveAguiPending.value = false;
      activeRunCode.value = "";
      activeRunSupportsSteer.value = null;
    }
  }
  detachSteerObserver();
}

function detachSteerObserver() {
  const controller = steerAbortRef.value;
  if (controller) {
    steerAbortRef.value = null;
    controller.abort();
  }
  steering.value = false;
}

async function loadActiveRunSnapshot(
  sessionCode: string,
  signal?: AbortSignal,
) {
  const adapter = activeAdapter.value;
  if (!adapter.capabilities.supportsActiveRun || !adapter.getActiveRun) {
    throw new Error("Active-run status is not supported by this adapter");
  }
  const result = await adapter.getActiveRun(sessionCode, signal);
  return result.activeRun;
}

type ActiveRunQueryResult =
  | { activeRun: ChatActiveRun | null; kind: "success" }
  | { kind: "failure" }
  | { kind: "unsupported" };

async function queryActiveRunSnapshot(
  sessionCode: string,
  signal?: AbortSignal,
): Promise<ActiveRunQueryResult> {
  const adapter = activeAdapter.value;
  if (!adapter.capabilities.supportsActiveRun || !adapter.getActiveRun) {
    return { kind: "unsupported" };
  }
  try {
    return {
      activeRun: await loadActiveRunSnapshot(sessionCode, signal),
      kind: "success",
    };
  } catch {
    return { kind: "failure" };
  }
}

function replaceQueuedSubmissions(next: ChatQueuedSubmission[]) {
  queuedSubmissions.value = next;
}

function createSendInput(
  activeSession: ChatSession,
  content: string,
): SendChatMessageInput {
  return {
    agentCode: normalizeOptionalValue(props.config.agentCode),
    appCode: normalizeOptionalValue(
      activeSession.appCode ?? props.config.appCode,
    ),
    clientCapabilities: { userInputRequests: true },
    content,
    taskCode: normalizeOptionalValue(
      activeSession.taskCode ?? props.config.taskCode,
    ),
    workspaceCode: activeSession.workspaceCode,
  };
}

function enqueueSubmission(
  activeSession: ChatSession,
  input: SendChatMessageInput,
  position: "front" | "back" = "back",
) {
  queuedSubmissionSequenceRef.value += 1;
  const submission: ChatQueuedSubmission = {
    id: `QUEUED-${Date.now()}-${queuedSubmissionSequenceRef.value}`,
    input,
    sessionCode: activeSession.sessionCode,
  };
  replaceQueuedSubmissions(
    position === "front"
      ? [submission, ...queuedSubmissions.value]
      : enqueueChatSubmission(queuedSubmissions.value, submission),
  );
  return submission;
}

function takeNextQueuedSubmission(sessionCode: string) {
  const result = takeNextQueuedChatSubmission(
    queuedSubmissions.value,
    sessionCode,
  );
  replaceQueuedSubmissions(result.remaining);
  return result.submission;
}

function drainNextQueuedSubmission(sessionCode: string) {
  const currentSession = session.value;
  if (
    !currentSession ||
    currentSession.sessionCode !== sessionCode ||
    sending.value ||
    remoteActiveRun.value ||
    pendingProtocolTerminalRun.value ||
    activeRunStatusPending.value
  ) {
    return;
  }
  const nextSubmission = takeNextQueuedSubmission(sessionCode);
  if (nextSubmission) {
    void runMessage(currentSession, nextSubmission.input, nextSubmission);
  }
}

function currentActiveRunTrackingState(): RuntimeActiveRunTrackingState {
  return {
    activeRun: remoteActiveRun.value,
    pendingProtocolTerminalRun: pendingProtocolTerminalRun.value,
    statusPending: activeRunStatusPending.value,
  };
}

function applyActiveRunTrackingState(state: RuntimeActiveRunTrackingState) {
  remoteActiveRun.value = state.activeRun;
  pendingProtocolTerminalRun.value = state.pendingProtocolTerminalRun;
  activeRunStatusPending.value = state.statusPending;
}

function markActiveRunStatusPending(sessionCode: string) {
  if (activeSessionCodeRef.value !== sessionCode) return;
  applyActiveRunTrackingState(
    retainActiveRunOnStatusFailure(currentActiveRunTrackingState()),
  );
}

function acceptActiveRunSnapshot(
  nextActiveRun: ChatActiveRun | null,
  sessionCode: string,
  protocolSnapshot: ChatAguiStreamEvent[] = protocolEvents.value,
  drainNow = true,
) {
  if (activeSessionCodeRef.value !== sessionCode) return false;
  const resolution = resolveSuccessfulActiveRunSnapshot({
    nextActiveRun,
    protocolEvents: protocolSnapshot,
    protocolReplayEnabled: protocolReplayEnabled.value,
    sessionCode,
    state: currentActiveRunTrackingState(),
    suppressedRunCodes: terminalHandledRunCodesRef,
  });
  applyActiveRunTrackingState(resolution.state);
  if (resolution.consumedSuppressedRunCode) {
    terminalHandledRunCodesRef.delete(resolution.consumedSuppressedRunCode);
  }
  if (resolution.shouldDrainQueue && drainNow && !sending.value) {
    drainNextQueuedSubmission(sessionCode);
  }
  return resolution.shouldDrainQueue;
}

function releasePendingProtocolTerminal(
  sessionCode: string,
  protocolSnapshot: ChatAguiStreamEvent[],
) {
  if (!pendingProtocolTerminalRun.value) return false;
  return acceptActiveRunSnapshot(null, sessionCode, protocolSnapshot, true);
}

async function refreshActiveRun(
  sessionCode: string,
  signal?: AbortSignal,
  drainNow = true,
) {
  const next = await loadActiveRunSnapshot(sessionCode, signal);
  acceptActiveRunSnapshot(next, sessionCode, protocolEvents.value, drainNow);
  return next;
}

function prependQueuedSubmission(submission: ChatQueuedSubmission) {
  replaceQueuedSubmissions([
    submission,
    ...removeQueuedChatSubmission(queuedSubmissions.value, submission.id),
  ]);
}

function restoreSourceSubmission(
  sourceSubmission: ChatQueuedSubmission | undefined,
  queueGeneration: number,
) {
  if (!sourceSubmission || queueGeneration !== queueGenerationRef.value) {
    return;
  }
  prependQueuedSubmission(sourceSubmission);
}

watch(
  normalizedInitialSession,
  (nextSession) => {
    activeSessionCodeRef.value = nextSession?.sessionCode ?? "";
    session.value = nextSession;
  },
  { immediate: true },
);

watch(
  session,
  (nextSession, previousSession) => {
    if (
      previousSession?.sessionCode &&
      previousSession.sessionCode !== nextSession?.sessionCode
    ) {
      queueGenerationRef.value += 1;
      detachRunObserver("detach");
      stopAbortRef.value?.abort();
      stopping.value = false;
      terminalHandledRunCodesRef.clear();
      remoteActiveRun.value = null;
      pendingProtocolTerminalRun.value = null;
      activeRunStatusPending.value = false;
      activeRunCode.value = "";
      activeRunSupportsSteer.value = null;
    }
    activeSessionCodeRef.value = nextSession?.sessionCode ?? "";
    props.onSessionChange?.(nextSession);
    emit("session-change", nextSession);
  },
  { immediate: true },
);

watch(
  () => session.value?.sessionCode,
  async (nextSessionCode, _, onCleanup) => {
    let active = true;
    const abortController = new AbortController();
    onCleanup(() => {
      active = false;
      abortController.abort();
    });

    if (!nextSessionCode) {
      messages.value = [];
      aguiEvents.value = [];
      protocolEvents.value = [];
      userInputRequest.value = null;
      remoteActiveRun.value = null;
      pendingProtocolTerminalRun.value = null;
      activeRunStatusPending.value = false;
      return;
    }

    loading.value = true;
    const [dataResult, activeRunResult] = await Promise.all([
      Promise.all([
        activeAdapter.value.loadMessages(nextSessionCode),
        aguiEnabled.value
          ? activeAdapter.value.loadAguiEvents(nextSessionCode)
          : Promise.resolve([]),
        protocolReplayEnabled.value && activeAdapter.value.loadProtocolEvents
          ? activeAdapter.value.loadProtocolEvents(nextSessionCode)
          : Promise.resolve([]),
        activeAdapter.value.capabilities.supportsUserInputRequests && activeAdapter.value.loadUserInputRequests
          ? activeAdapter.value.loadUserInputRequests(nextSessionCode, abortController.signal)
          : Promise.resolve([]),
      ]).then(
        ([nextMessages, nextAguiEvents, nextProtocolEvents, nextUserInputRequests]) => ({
          kind: "success" as const,
          nextAguiEvents,
          nextMessages,
          nextProtocolEvents,
          nextUserInputRequests,
        }),
        (error: unknown) => ({ error, kind: "failure" as const }),
      ),
      queryActiveRunSnapshot(nextSessionCode, abortController.signal),
    ]);
    if (!active || abortController.signal.aborted) return;
    if (dataResult.kind === "success") {
      messages.value = dataResult.nextMessages;
      aguiEvents.value = dataResult.nextAguiEvents;
      protocolEvents.value = dataResult.nextProtocolEvents;
      userInputRequest.value = dataResult.nextUserInputRequests.find(
        (request) => request.status === "Pending",
      ) ?? null;
    } else {
      const message =
        dataResult.error instanceof Error
          ? dataResult.error.message
          : "Chat data could not be loaded";
      props.onError?.(message, "Chat load failed");
      emit("error", message, "Chat load failed");
    }
    if (activeRunResult.kind === "success") {
      acceptActiveRunSnapshot(
        activeRunResult.activeRun,
        nextSessionCode,
        dataResult.kind === "success"
          ? dataResult.nextProtocolEvents
          : protocolEvents.value,
        false,
      );
    } else if (activeRunResult.kind === "failure") {
      markActiveRunStatusPending(nextSessionCode);
    }
    loading.value = false;
  },
  { immediate: true },
);

const shouldPollData = computed(() =>
  Boolean(
    session.value?.sessionCode &&
      (isRecentPendingUserTurn(messages.value) ||
        Boolean(userInputRequest.value) ||
        (protocolReplayEnabled.value &&
          openAguiProtocolRunCodes(
            protocolEvents.value,
            session.value.sessionCode,
          ).size > 0)),
  ),
);
const shouldPollRuntime = computed(() =>
  Boolean(
    session.value?.sessionCode &&
      (runActive.value ||
        shouldPollData.value ||
        activeRunStatusPending.value ||
        pendingProtocolTerminalRun.value),
  ),
);

watch(
  [() => session.value?.sessionCode, shouldPollRuntime],
  (_, __, onCleanup) => {
    const sessionCode = session.value?.sessionCode;
    if (!sessionCode || !shouldPollRuntime.value) return;
    let refreshing = false;
    const intervalId = window.setInterval(async () => {
      if (refreshing || activeSessionCodeRef.value !== sessionCode) return;
      refreshing = true;
      const [dataResult, activeRunResult] = await Promise.all([
        Promise.all([
          activeAdapter.value.loadMessages(sessionCode),
          aguiEnabled.value
            ? activeAdapter.value.loadAguiEvents(sessionCode)
            : Promise.resolve([]),
          protocolReplayEnabled.value && activeAdapter.value.loadProtocolEvents
            ? activeAdapter.value.loadProtocolEvents(sessionCode)
            : Promise.resolve([]),
          activeAdapter.value.capabilities.supportsUserInputRequests && activeAdapter.value.loadUserInputRequests
            ? activeAdapter.value.loadUserInputRequests(sessionCode)
            : Promise.resolve([]),
        ]).then(
          ([nextMessages, nextAguiEvents, nextProtocolEvents, nextUserInputRequests]) => ({
            kind: "success" as const,
            nextAguiEvents,
            nextMessages,
            nextProtocolEvents,
            nextUserInputRequests,
          }),
          () => ({ kind: "failure" as const }),
        ),
        queryActiveRunSnapshot(sessionCode),
      ]);
      if (activeSessionCodeRef.value === sessionCode) {
        let protocolSnapshot = protocolEvents.value;
        if (dataResult.kind === "success") {
          messages.value = dataResult.nextMessages;
          aguiEvents.value = dataResult.nextAguiEvents;
          protocolEvents.value = dataResult.nextProtocolEvents;
          userInputRequest.value = dataResult.nextUserInputRequests.find(
            (request) => request.status === "Pending",
          ) ?? null;
          protocolSnapshot = dataResult.nextProtocolEvents;
        }
        if (activeRunResult.kind === "success") {
          acceptActiveRunSnapshot(
            activeRunResult.activeRun,
            sessionCode,
            protocolSnapshot,
          );
        } else if (activeRunResult.kind === "failure") {
          markActiveRunStatusPending(sessionCode);
          releasePendingProtocolTerminal(sessionCode, protocolSnapshot);
        } else {
          releasePendingProtocolTerminal(sessionCode, protocolSnapshot);
        }
      }
      refreshing = false;
    }, 1000);

    onCleanup(() => {
      window.clearInterval(intervalId);
    });
  },
  { immediate: true },
);

const sendDisabled = computed(() => {
  if (loading.value || props.config.disabledInput || userInputRequest.value) return true;
  if (!draft.value.trim()) return true;
  return !session.value && !props.config.workspaceCode;
});
const queueDisabled = computed(
  () =>
    loading.value ||
    Boolean(props.config.disabledInput) ||
    Boolean(userInputRequest.value) ||
    !draft.value.trim() ||
    !session.value,
);
const steerDisabled = computed(
  () => queueDisabled.value || steering.value || !canSteer.value,
);

function reportError(message: string, title?: string) {
  props.onError?.(message, title);
  emit("error", message, title);
}

function reportInfo(message: string, title?: string) {
  props.onInfo?.(message, title);
  emit("info", message, title);
}

async function ensureSession() {
  if (session.value) return session.value;
  if (!props.config.workspaceCode) {
    reportError(
      "Workspace code is required before starting a session.",
      "Session create failed",
    );
    return null;
  }

  const created = await activeAdapter.value.createSession({
    agentCode: normalizeOptionalValue(props.config.agentCode),
    appCode: normalizeOptionalValue(props.config.appCode),
    taskCode: normalizeOptionalValue(props.config.taskCode),
    title: normalizeOptionalValue(props.config.title) ?? props.floatingTitle,
    workspaceCode: props.config.workspaceCode,
  });
  activeSessionCodeRef.value = created.sessionCode;
  session.value = created;
  messages.value = [];
  aguiEvents.value = [];
  protocolEvents.value = [];
  return created;
}

async function runMessage(
  activeSession: ChatSession,
  sendInput: SendChatMessageInput,
  sourceSubmission?: ChatQueuedSubmission,
) {
  const queueGeneration = queueGenerationRef.value;
  const autoDrainEpoch = autoDrainEpochRef.value;
  if (
    sending.value ||
    remoteActiveRun.value ||
    pendingProtocolTerminalRun.value ||
    activeRunStatusPending.value
  ) {
    restoreSourceSubmission(sourceSubmission, queueGeneration);
    return false;
  }
  const content = sendInput.content.trim();
  if (!content) {
    restoreSourceSubmission(sourceSubmission, queueGeneration);
    return false;
  }
  const adapter = activeAdapter.value;
  const isAguiEnabled = aguiEnabled.value;
  const abortController = new AbortController();
  const observer = {
    abortReason: "none" as RuntimeStreamAbortReason,
    controller: abortController,
    sessionCode: activeSession.sessionCode,
  };
  activeObserverRef.value = observer;
  activeRunCode.value = "";
  activeRunSupportsSteer.value = null;
  optimisticMessageSequenceRef.value += 1;
  const optimisticKey = `${Date.now()}-${optimisticMessageSequenceRef.value}`;
  const optimisticCreatedAt = new Date().toISOString();
  const optimisticUserCode = `LOCAL-${optimisticKey}-USER`;
  const optimisticAssistantCode = `LOCAL-${optimisticKey}-ASSISTANT`;
  sending.value = true;
  liveAguiPending.value = isAguiEnabled;
  messages.value = [
    ...messages.value,
    {
      actor: props.currentUserDisplayName || "User",
      channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
      content,
      createdAt: optimisticCreatedAt,
      id: -Date.now() - optimisticMessageSequenceRef.value,
      messageCode: optimisticUserCode,
      role: "USER",
      runCode: null,
      sessionCode: activeSession.sessionCode,
    },
    {
      actor: activeSession.agentCode ?? props.config.agentCode ?? "Assistant",
      channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
      content: "",
      createdAt: optimisticCreatedAt,
      id: -Date.now() - optimisticMessageSequenceRef.value - 1,
      messageCode: optimisticAssistantCode,
      role: "ASSISTANT",
      runCode: null,
      sessionCode: activeSession.sessionCode,
    },
  ];

  let completed = false;
  let completedRunCode = "";
  let completedSession: ChatSession | null = null;
  let streamSucceeded = false;
  // Once a request crosses the transport boundary, automatic retry is unsafe:
  // the backend may already have persisted its USER message. The sole safe
  // restore path is CHAT_RUN_ALREADY_ACTIVE, which rejects before admission.
  let shouldRestoreSourceSubmission = false;
  let drainConflictQueue = false;
  try {
    const result = await adapter.streamMessage(
      activeSession,
      sendInput,
      {
        onAguiEvent: (event) => {
          if (
            !ownsRuntimeStreamObserver(
              activeObserverRef.value,
              observer,
              activeSessionCodeRef.value,
            )
          ) {
            return;
          }
          const runCode = String(event.runId ?? event.runCode ?? "");
          if (typeof event.supportsSteer === "boolean") {
            activeRunSupportsSteer.value = event.supportsSteer;
          }
          if (runCode && !activeRunCode.value) activeRunCode.value = runCode;
          if (!isAguiEnabled) return;
          liveAguiEvents.value = [...liveAguiEvents.value, event];
          if (!runCode) return;
          liveAguiAnchorMessageCodes.value = liveAguiAnchorMessageCodes.value[
            runCode
          ]
            ? liveAguiAnchorMessageCodes.value
            : {
                ...liveAguiAnchorMessageCodes.value,
                [runCode]: optimisticUserCode,
              };
        },
        onDelta: (delta) => {
          if (
            !ownsRuntimeStreamObserver(
              activeObserverRef.value,
              observer,
              activeSessionCodeRef.value,
            )
          ) {
            return;
          }
          liveAguiPending.value = false;
          messages.value = messages.value.map((message) =>
            message.messageCode === optimisticAssistantCode
              ? { ...message, content: `${message.content}${delta}` }
              : message,
          );
        },
      },
      abortController.signal,
    );
    const settlement = resolveRuntimeStreamSettlement({
      activeObserver: activeObserverRef.value,
      activeSessionCode: activeSessionCodeRef.value,
      observer,
    });
    userInputRequest.value =
      result.userInputRequest?.status === "Pending"
        ? result.userInputRequest
        : null;
    streamSucceeded = settlement.streamSucceeded;
    shouldRestoreSourceSubmission = settlement.restoreSourceSubmission;

    if (
      settlement.canCommit &&
      result.session.sessionCode === activeSession.sessionCode
    ) {
      const [nextMessages, nextAguiEvents, nextProtocolEvents] =
        await Promise.all([
          adapter.loadMessages(result.session.sessionCode).catch(() => null),
          isAguiEnabled
            ? adapter
                .loadAguiEvents(result.session.sessionCode)
                .catch(() => null)
            : Promise.resolve(null),
          isAguiEnabled &&
          protocolReplayEnabled.value &&
          adapter.loadProtocolEvents
            ? adapter
                .loadProtocolEvents(result.session.sessionCode)
                .catch(() => null)
            : Promise.resolve(isAguiEnabled ? [] : null),
        ]);

      if (
        ownsRuntimeStreamObserver(
          activeObserverRef.value,
          observer,
          activeSessionCodeRef.value,
        )
      ) {
        completedRunCode = result.run.runCode;
        terminalHandledRunCodesRef.add(result.run.runCode);
        if (
          (remoteActiveRun.value as unknown as ChatActiveRun)?.runCode ===
          result.run.runCode
        ) {
          remoteActiveRun.value = null;
        }
        pendingProtocolTerminalRun.value = null;
        activeRunStatusPending.value = false;
        session.value = result.session;
        if (nextMessages) {
          messages.value = nextMessages;
        } else {
          messages.value = mergeChatMessagesChronologically(
            messages.value,
            result.messages,
            [optimisticUserCode, optimisticAssistantCode],
          );
        }
        if (isAguiEnabled) {
          if (nextAguiEvents) aguiEvents.value = nextAguiEvents;
          if (nextProtocolEvents) protocolEvents.value = nextProtocolEvents;
        }
        liveAguiEvents.value = [];
        liveAguiAnchorMessageCodes.value = {};
        completedSession = result.session;
        completed = autoDrainEpochRef.value === autoDrainEpoch;
      }
    }
  } catch (error) {
    const settlement = resolveRuntimeStreamSettlement({
      activeObserver: activeObserverRef.value,
      activeSessionCode: activeSessionCodeRef.value,
      observer,
    });
    const stillViewingSession =
      activeSessionCodeRef.value === activeSession.sessionCode;
    if (!settlement.canCommit) {
      streamSucceeded = settlement.streamSucceeded;
      shouldRestoreSourceSubmission = settlement.restoreSourceSubmission;
      if (observer.abortReason === "stop" && stillViewingSession) {
        messages.value = messages.value.filter(
          (message) =>
            message.messageCode !== optimisticAssistantCode ||
            Boolean(message.content.trim()),
        );
      }
    } else if (isRunAlreadyActiveError(error)) {
      messages.value = messages.value.filter(
        (message) =>
          message.messageCode !== optimisticUserCode &&
          message.messageCode !== optimisticAssistantCode,
      );
      if (sourceSubmission) {
        restoreSourceSubmission(sourceSubmission, queueGeneration);
      } else if (queueGeneration === queueGenerationRef.value) {
        enqueueSubmission(activeSession, sendInput, "front");
      }
      shouldRestoreSourceSubmission = false;
      markActiveRunStatusPending(activeSession.sessionCode);
      try {
        const nextActiveRun = await loadActiveRunSnapshot(
          activeSession.sessionCode,
        );
        drainConflictQueue = acceptActiveRunSnapshot(
          nextActiveRun,
          activeSession.sessionCode,
          protocolEvents.value,
          false,
        );
      } catch {
        // Keep the queued message until authoritative status can be refreshed.
      }
      if (activeSessionCodeRef.value === activeSession.sessionCode) {
        reportInfo(
          "The active run changed before send; the message was kept in the queue",
          "Message queued",
        );
      }
    } else {
      messages.value = messages.value.filter(
        (message) =>
          message.messageCode !== optimisticAssistantCode ||
          Boolean(message.content.trim()),
      );
    }
    if (settlement.canCommit && !isRunAlreadyActiveError(error)) {
      const message =
        error instanceof Error ? error.message : "Chat send failed";
      reportError(message, "Chat send failed");
    }
  } finally {
    if (activeObserverRef.value === observer) {
      sending.value = false;
      liveAguiPending.value = false;
      activeObserverRef.value = null;
      activeRunCode.value = "";
      activeRunSupportsSteer.value = null;
    }
  }

  if (!streamSucceeded && shouldRestoreSourceSubmission) {
    restoreSourceSubmission(sourceSubmission, queueGeneration);
  }

  if (
    drainConflictQueue &&
    activeSessionCodeRef.value === activeSession.sessionCode
  ) {
    drainNextQueuedSubmission(activeSession.sessionCode);
  }

  if (
    completed &&
    completedSession &&
    activeSessionCodeRef.value === activeSession.sessionCode
  ) {
    if (
      (remoteActiveRun.value as unknown as ChatActiveRun)?.runCode ===
      completedRunCode
    ) {
      remoteActiveRun.value = null;
    }
    const nextSubmission = takeNextQueuedSubmission(activeSession.sessionCode);
    if (nextSubmission) {
      void runMessage(completedSession, nextSubmission.input, nextSubmission);
    }
  }
  return completed;
}

async function handleSend() {
  if (sendDisabled.value || userInputRequest.value || userInputAnswerPending.value) return;
  const activeSession = await ensureSession();
  if (!activeSession) return;
  const content = draft.value.trim();
  if (!content) return;
  const sendInput = createSendInput(activeSession, content);
  draft.value = "";
  if (runActive.value) {
    enqueueSubmission(activeSession, sendInput);
    return;
  }
  await runMessage(activeSession, sendInput);
}

async function handleQueue() {
  if (queueDisabled.value || userInputRequest.value || userInputAnswerPending.value) return;
  const activeSession = await ensureSession();
  if (!activeSession) return;
  const content = draft.value.trim();
  if (!content) return;
  const sendInput = createSendInput(activeSession, content);
  draft.value = "";
  if (runActive.value) {
    enqueueSubmission(activeSession, sendInput);
    return;
  }
  await runMessage(activeSession, sendInput);
}

async function handleSteer() {
  if (steerDisabled.value || userInputRequest.value || userInputAnswerPending.value) return;
  const activeSession = session.value;
  if (!activeSession) return;
  const content = draft.value.trim();
  if (!content) return;
  const sendInput = createSendInput(activeSession, content);
  const expectedRunCode = effectiveActiveRunCode.value;
  const adapter = activeAdapter.value;
  const queueGeneration = queueGenerationRef.value;
  draft.value = "";
  if (
    !runActive.value ||
    !expectedRunCode ||
    !adapter.capabilities.supportsSteer ||
    !adapter.steerMessage
  ) {
    enqueueSubmission(activeSession, sendInput);
    return;
  }

  const abortController = new AbortController();
  steerAbortRef.value = abortController;
  steering.value = true;
  try {
    const result = await adapter.steerMessage(
      activeSession,
      sendInput,
      { expectedRunCode },
      abortController.signal,
    );
    if (steerAbortRef.value !== abortController) {
      // A resolved ACK means the backend accepted and persisted the steer.
      // Ignore a stale observer result without sending the same input again.
      return;
    }
    if (activeSessionCodeRef.value === result.sessionCode) {
      messages.value = mergeChatMessagesChronologically(messages.value, [
        result.message,
      ]);
    }
  } catch (error) {
    const observerDetached = steerAbortRef.value !== abortController;
    const queueIsCurrent = queueGeneration === queueGenerationRef.value;
    const safeQueueFallback =
      error instanceof FetchTransportError &&
      isSteerQueueFallbackCode(error.code);
    if (safeQueueFallback && queueIsCurrent) {
      enqueueSubmission(activeSession, sendInput, "front");
      try {
        await refreshActiveRun(activeSession.sessionCode, undefined, true);
      } catch {
        markActiveRunStatusPending(activeSession.sessionCode);
        // Keep the queued fallback until the next authoritative refresh.
      }
    }
    const stopped = error instanceof Error && error.name === "AbortError";
    if (safeQueueFallback) {
      if (queueIsCurrent) {
        reportInfo(
          "The active run rejected steer; the message was queued",
          "Message queued",
        );
      }
    } else if (observerDetached || stopped) {
      if (queueIsCurrent) {
        reportInfo(
          "Steer delivery was not confirmed and was not retried",
          "Steer not retried",
        );
      }
    } else {
      const message =
        error instanceof Error ? error.message : "Chat steer failed";
      reportError(message, "Chat steer failed");
    }
  } finally {
    if (steerAbortRef.value === abortController) {
      steerAbortRef.value = null;
      steering.value = false;
    }
  }
}

async function handleSubmit(mode: ChatSubmitMode = "send") {
  if (mode === "queue") {
    await handleQueue();
    return;
  }
  if (mode === "steer") {
    await handleSteer();
    return;
  }
  await handleSend();
}

async function handleAnswerUserInput(answers: ChatUserInputAnswer[]) {
  const activeSession = session.value;
  const request = userInputRequest.value;
  const adapter = activeAdapter.value;
  if (
    !activeSession ||
    !request ||
    userInputAnswerPending.value ||
    !adapter.answerUserInputRequestStream
  ) {
    return;
  }
  userInputAnswerPending.value = true;
  try {
    const result = await adapter.answerUserInputRequestStream(
      activeSession,
      request.requestCode,
      { answers },
      undefined,
    );
    if (activeSessionCodeRef.value !== activeSession.sessionCode) return;
    const nextMessages = await adapter.loadMessages(activeSession.sessionCode).catch(() => null);
    messages.value = nextMessages
      ? nextMessages
      : mergeChatMessagesChronologically(messages.value, result.messages);
    userInputRequest.value =
      result.userInputRequest?.status === "Pending"
        ? result.userInputRequest
        : null;
    session.value = result.session;
    await refreshActiveRun(activeSession.sessionCode, undefined, false).catch(() => undefined);
    reportInfo("The assistant continued with your answers", "Answer submitted");
  } catch (error) {
    reportError(
      error instanceof Error ? error.message : "Answer submission failed",
      "Answer submission failed",
    );
  } finally {
    userInputAnswerPending.value = false;
  }
}

function handleEditQueuedMessage(id: string, content: string) {
  replaceQueuedSubmissions(
    removeQueuedChatSubmission(queuedSubmissions.value, id),
  );
  draft.value = content;
}

function handleRemoveQueuedMessage(id: string) {
  replaceQueuedSubmissions(
    removeQueuedChatSubmission(queuedSubmissions.value, id),
  );
}

async function handleStop() {
  if (stopping.value) return;
  const activeSession = session.value;
  if (!activeSession) return;

  const adapter = activeAdapter.value;
  let expectedRunCode = effectiveActiveRunCode.value;
  if (
    !expectedRunCode &&
    adapter.capabilities.supportsStop &&
    adapter.stopRun
  ) {
    try {
      expectedRunCode =
        (await refreshActiveRun(activeSession.sessionCode, undefined, false))
          ?.runCode ?? "";
    } catch {
      markActiveRunStatusPending(activeSession.sessionCode);
      // Fall back only when this adapter explicitly supports local abort.
    }
  }
  if (expectedRunCode && adapter.capabilities.supportsStop && adapter.stopRun) {
    const abortController = new AbortController();
    stopAbortRef.value = abortController;
    stopping.value = true;
    autoDrainEpochRef.value += 1;
    terminalHandledRunCodesRef.add(expectedRunCode);
    try {
      await adapter.stopRun(
        activeSession,
        { expectedRunCode },
        abortController.signal,
      );
      if (activeSessionCodeRef.value !== activeSession.sessionCode) return;
      remoteActiveRun.value = {
        runCode: expectedRunCode,
        runtimeAdapterKey: remoteActiveRun.value?.runtimeAdapterKey ?? "",
        state: "FINISHING",
        supportsSteer: false,
      };
      pendingProtocolTerminalRun.value = null;
      activeRunStatusPending.value = false;
      detachRunObserver("stop");
      reportInfo("Current response has been stopped", "Chat stopped");
    } catch (error) {
      terminalHandledRunCodesRef.delete(expectedRunCode);
      if (abortController.signal.aborted) return;
      const message =
        error instanceof Error ? error.message : "Chat stop failed";
      reportError(message, "Chat stop failed");
      try {
        await refreshActiveRun(activeSession.sessionCode, undefined, true);
      } catch {
        markActiveRunStatusPending(activeSession.sessionCode);
        // Keep the queue paused until the next successful status refresh.
      }
    } finally {
      if (stopAbortRef.value === abortController) {
        stopAbortRef.value = null;
        stopping.value = false;
      }
    }
    return;
  }

  if (
    sending.value &&
    adapter.capabilities.supportsAbort &&
    !(adapter.capabilities.supportsStop && adapter.stopRun)
  ) {
    autoDrainEpochRef.value += 1;
    if (expectedRunCode) terminalHandledRunCodesRef.add(expectedRunCode);
    detachRunObserver("stop");
    reportInfo("Current response has been stopped", "Chat stopped");
  }
}

function handleNewSession() {
  queueGenerationRef.value += 1;
  detachRunObserver("detach");
  stopAbortRef.value?.abort();
  session.value = null;
  messages.value = [];
  userInputRequest.value = null;
  aguiEvents.value = [];
  protocolEvents.value = [];
  liveAguiEvents.value = [];
  liveAguiAnchorMessageCodes.value = {};
  activeSessionCodeRef.value = "";
  activeRunCode.value = "";
  activeRunSupportsSteer.value = null;
  remoteActiveRun.value = null;
  pendingProtocolTerminalRun.value = null;
  activeRunStatusPending.value = false;
  stopping.value = false;
  replaceQueuedSubmissions([]);
}

onBeforeUnmount(() => {
  queueGenerationRef.value += 1;
  detachRunObserver("detach");
  stopAbortRef.value?.abort();
});

const presentationDebugEvents = computed(
  () =>
    buildTranscriptPresentation({
      debugOptions: props.debugOptions,
      liveAguiAnchorMessageCodes: liveAguiAnchorMessageCodes.value,
      liveAguiEvents: liveAguiEvents.value,
      liveAguiPending: liveAguiPending.value,
      messages: messages.value,
      protocolEvents: protocolEvents.value,
      sessionCode: session.value?.sessionCode ?? "",
    }).debugEventsByRunCode,
);

const resolvedDebugOptions = computed(() => ({
  ...props.debugOptions,
  debugEventTypes: props.debugOptions?.debugEventTypes?.length
    ? Array.from(
        new Set([
          ...props.debugOptions.debugEventTypes,
          ...Array.from(presentationDebugEvents.value.keys()).flatMap(
            (runCode) =>
              mergeTranscriptAguiEvents(
                protocolEvents.value,
                liveAguiEvents.value,
              )
                .filter(
                  (event) => (event.runId ?? event.runCode ?? "") === runCode,
                )
                .map((event) => String(event.type).toUpperCase()),
          ),
        ]),
      )
    : props.debugOptions?.debugEventTypes,
}));

function handleDraftChange(value: string) {
  draft.value = value;
}

function handleEditMessage(content: string) {
  draft.value = content;
}

const panelCommonProps = computed(() => ({
  createSessionLabel: "Start chat",
  currentUserDisplayName: props.currentUserDisplayName,
  debugOptions: resolvedDebugOptions.value,
  disabledInput: Boolean(props.config.disabledInput) || Boolean(userInputRequest.value),
  draft: draft.value,
  emptyDescription: props.config.workspaceCode
    ? "Open a session and start talking to your configured agent."
    : "Set a workspace code before starting a session.",
  inputPlaceholder: props.config.inputPlaceholder,
  liveAguiAnchorMessageCodes: liveAguiAnchorMessageCodes.value,
  liveAguiEvents: liveAguiEvents.value,
  liveAguiPending: liveAguiPending.value,
  messages: messages.value,
  onCreateSession: () => {
    void ensureSession();
  },
  onNewSession: handleNewSession,
  onDraftChange: handleDraftChange,
  onEditMessage: handleEditMessage,
  onEditQueuedMessage: handleEditQueuedMessage,
  onRemoveQueuedMessage: handleRemoveQueuedMessage,
  onAnswerUserInput: (answers: ChatUserInputAnswer[]) => {
    void handleAnswerUserInput(answers);
  },
  onSend: (mode?: ChatSubmitMode) => {
    void handleSubmit(mode);
  },
  onStop: () => {
    void handleStop();
  },
  protocolEvents: protocolEvents.value,
  queueDisabled: queueDisabled.value,
  queuedMessages: visibleQueuedSubmissions.value,
  runningSubmitModes: runningSubmitModes.value,
  sendDisabled: sendDisabled.value || Boolean(userInputRequest.value),
  sending: runActive.value,
  session: session.value,
  showAgui: aguiEnabled.value,
  showStop: canStop.value,
  steerDisabled: steerDisabled.value || Boolean(userInputRequest.value),
  stopLabel: "Stop",
  title: props.config.title || props.floatingTitle,
  userInputLabels: props.config.userInputLabels,
  workRailLabels: props.config.workRailLabels,
  userInputAnswerPending: userInputAnswerPending.value,
  userInputRequest: userInputRequest.value,
}));

function handleWindowStateChange(state: ChatFloatingState) {
  props.onWindowStateChange?.(state);
  emit("window-state-change", state);
}
</script>

<template>
  <TronChatWindow
    v-if="config.floating !== false"
    v-bind="panelCommonProps"
    :state="windowState"
    :storage-key="config.storageKey || 'tron.chat.runtime.window'"
    :title="floatingTitle"
    :title-subtitle="floatingSubtitle"
    @state-change="handleWindowStateChange"
  >
    <template v-for="name in forwardedSlotNames" #[name]="slotProps">
      <slot :name="name" v-bind="slotProps || {}" />
    </template>
  </TronChatWindow>
  <TronChatPanel v-else v-bind="panelCommonProps" :show-header="true">
    <template v-for="name in forwardedSlotNames" #[name]="slotProps">
      <slot :name="name" v-bind="slotProps || {}" />
    </template>
  </TronChatPanel>
</template>

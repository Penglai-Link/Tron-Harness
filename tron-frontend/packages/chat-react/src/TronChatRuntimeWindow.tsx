import {
  buildTranscriptPresentation,
  createMockChatAdapter,
  enqueueChatSubmission,
  FetchTransportError,
  isRecentPendingUserTurn,
  isSteerQueueFallbackCode,
  mergeChatMessagesChronologically,
  mergeTranscriptAguiEvents,
  normalizeChatSession,
  openAguiProtocolRunCodes,
  removeQueuedChatSubmission,
  takeNextQueuedChatSubmission,
  type ChatFloatingState,
  type ChatAdapter,
  type ChatActiveRun,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatClientConfig,
  type ChatDebugOptions,
  type ChatMessage,
  type ChatQueuedSubmission,
  type ChatRunningSubmitMode,
  type ChatSession,
  type ChatSendResult,
  type ChatSubmitMode,
  type SendChatMessageInput,
} from "@tron/chat-core";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { TronChatPanel } from "./TronChatPanel";
import { TronChatWindow } from "./TronChatWindow";

interface ToastLikeCallback {
  (message: string, title?: string): void;
}

export interface TronChatRuntimeWindowProps {
  adapter?: ChatAdapter;
  config: ChatClientConfig;
  currentUserDisplayName?: string;
  debugOptions?: ChatDebugOptions;
  floatingTitle?: string;
  floatingSubtitle?: string;
  footerAddon?: ReactNode;
  headerContent?: ReactNode;
  initialSession?: ChatSession | null;
  onError?: ToastLikeCallback;
  onInfo?: ToastLikeCallback;
  onSessionChange?: (session: ChatSession | null) => void;
  sidebarContent?: ReactNode;
  windowState?: ChatFloatingState;
  onWindowStateChange?: (state: ChatFloatingState) => void;
}

function defaultErrorReporter(message: string) {
  console.error(message);
}

function normalizeOptionalValue(value: string | null | undefined) {
  const normalized = value?.trim();
  return normalized ? normalized : undefined;
}

export function TronChatRuntimeWindow({
  adapter,
  config,
  currentUserDisplayName,
  debugOptions,
  floatingSubtitle = "Workspace assistant",
  floatingTitle = "Main Agent",
  footerAddon,
  headerContent,
  initialSession = null,
  onError = defaultErrorReporter,
  onInfo,
  onSessionChange,
  sidebarContent,
  windowState,
  onWindowStateChange,
}: TronChatRuntimeWindowProps) {
  const activeAdapter = useMemo(
    () => adapter ?? createMockChatAdapter(),
    [adapter],
  );
  const normalizedInitialSession = useMemo(
    () => (initialSession ? normalizeChatSession(initialSession) : null),
    [initialSession],
  );
  const [session, setSession] = useState<ChatSession | null>(normalizedInitialSession);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [aguiEvents, setAguiEvents] = useState<ChatAguiEvent[]>([]);
  const [protocolEvents, setProtocolEvents] = useState<ChatAguiStreamEvent[]>([]);
  const [liveAguiEvents, setLiveAguiEvents] = useState<ChatAguiStreamEvent[]>([]);
  const [liveAguiAnchorMessageCodes, setLiveAguiAnchorMessageCodes] = useState<Record<string, string>>({});
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [steering, setSteering] = useState(false);
  const [loading, setLoading] = useState(false);
  const [liveAguiPending, setLiveAguiPending] = useState(false);
  const [activeRunCode, setActiveRunCode] = useState("");
  const [activeRunSupportsSteer, setActiveRunSupportsSteer] = useState<boolean | null>(null);
  const [remoteActiveRun, setRemoteActiveRun] = useState<ChatActiveRun | null>(null);
  const [activeRunStatusPending, setActiveRunStatusPending] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [queuedSubmissions, setQueuedSubmissions] = useState<ChatQueuedSubmission[]>([]);
  const activeSessionCodeRef = useRef("");
  const activeRunCodeRef = useRef("");
  const remoteActiveRunRef = useRef<ChatActiveRun | null>(null);
  const activeRunStatusPendingRef = useRef(false);
  const abortRef = useRef<AbortController | null>(null);
  const steerAbortRef = useRef<AbortController | null>(null);
  const steeringRef = useRef(false);
  const autoDrainEpochRef = useRef(0);
  const streamAbortReasonsRef = useRef(
    new WeakMap<AbortController, "detach" | "none" | "stop">(),
  );
  const sendingRef = useRef(false);
  const queuedSubmissionsRef = useRef<ChatQueuedSubmission[]>([]);
  const queueGenerationRef = useRef(0);
  const queuedSubmissionSequenceRef = useRef(0);
  const optimisticMessageSequenceRef = useRef(0);
  const currentLiveAguiRunCodesRef = useRef<Set<string>>(new Set());
  const observedRemoteRunRef = useRef<{ runCode: string; sessionCode: string } | null>(null);
  const drainedRemoteRunKeysRef = useRef<Set<string>>(new Set());
  const suppressedAutoDrainRunKeysRef = useRef<Set<string>>(new Set());
  const aguiEnabled = config.showAgui !== false && activeAdapter.capabilities.supportsAgui;
  const protocolReplayEnabled = aguiEnabled
    && activeAdapter.capabilities.supportsProtocolReplay
    && Boolean(activeAdapter.loadProtocolEvents);
  const effectiveActiveRunCode = activeRunCode || remoteActiveRun?.runCode || "";
  const runActive = sending || Boolean(remoteActiveRun);
  const effectiveRunState = remoteActiveRun?.runCode === effectiveActiveRunCode
    ? remoteActiveRun.state
    : "RUNNING";
  const effectiveSupportsSteer = remoteActiveRun?.runCode === effectiveActiveRunCode
    ? remoteActiveRun.supportsSteer
    : activeRunSupportsSteer;
  const canSteer = Boolean(
    activeAdapter.capabilities.supportsSteer
      && activeAdapter.steerMessage
      && effectiveActiveRunCode
      && effectiveRunState === "RUNNING"
      && effectiveSupportsSteer !== false,
  );
  const runningSubmitModes = useMemo<ChatRunningSubmitMode[]>(
    () => (canSteer ? ["steer", "queue"] : ["queue"]),
    [canSteer],
  );
  const canStop = Boolean(
    session
      && !stopping
      && (
        (
          effectiveActiveRunCode
          && effectiveRunState === "RUNNING"
          && activeAdapter.capabilities.supportsStop
          && activeAdapter.stopRun
        )
        || (
          sending
          && activeAdapter.capabilities.supportsAbort
          && !(
            activeAdapter.capabilities.supportsStop
            && activeAdapter.stopRun
          )
        )
      ),
  );
  const visibleQueuedSubmissions = useMemo(
    () => queuedSubmissions.filter((submission) => (
      submission.sessionCode === session?.sessionCode
    )),
    [queuedSubmissions, session?.sessionCode],
  );

  function setSendingState(value: boolean) {
    sendingRef.current = value;
    setSending(value);
  }

  function replaceActiveRunStatusPending(value: boolean) {
    activeRunStatusPendingRef.current = value;
    setActiveRunStatusPending(value);
  }

  function currentStreamAbortReason(
    controller: AbortController,
  ): "detach" | "none" | "stop" {
    return streamAbortReasonsRef.current.get(controller) ?? "none";
  }

  function detachRunObserver(
    reason: "detach" | "stop",
    releaseState = true,
  ) {
    const controller = abortRef.current;
    if (!controller) return false;
    streamAbortReasonsRef.current.set(controller, reason);
    abortRef.current = null;
    if (releaseState) {
      setSendingState(false);
      setLiveAguiPending(false);
      activeRunCodeRef.current = "";
      setActiveRunCode("");
      setActiveRunSupportsSteer(null);
    }
    controller.abort();
    return true;
  }

  function detachSteerObserver(releaseState = true) {
    const controller = steerAbortRef.current;
    if (!controller) return false;
    steerAbortRef.current = null;
    if (releaseState) {
      steeringRef.current = false;
      setSteering(false);
    }
    controller.abort();
    return true;
  }

  function replaceRemoteActiveRun(next: ChatActiveRun | null) {
    remoteActiveRunRef.current = next;
    setRemoteActiveRun(next);
  }

  async function loadActiveRunSnapshot(
    sessionCode: string,
    signal?: AbortSignal,
  ) {
    if (
      !activeAdapter.capabilities.supportsActiveRun
      || !activeAdapter.getActiveRun
    ) {
      return null;
    }
    const result = await activeAdapter.getActiveRun(sessionCode, signal);
    return result.activeRun;
  }

  async function refreshActiveRun(sessionCode: string) {
    try {
      const next = await loadActiveRunSnapshot(sessionCode);
      if (activeSessionCodeRef.current === sessionCode) {
        replaceRemoteActiveRun(next);
        if (next) {
          replaceActiveRunStatusPending(false);
        } else if (
          activeRunStatusPendingRef.current
          && !sendingRef.current
        ) {
          replaceActiveRunStatusPending(false);
          const nextSubmission = takeNextQueuedSubmission(sessionCode);
          if (nextSubmission && session?.sessionCode === sessionCode) {
            void runMessage(session, nextSubmission.input, nextSubmission);
          }
        }
      }
      return next;
    } catch {
      return undefined;
    }
  }

  function replaceQueuedSubmissions(next: ChatQueuedSubmission[]) {
    queuedSubmissionsRef.current = next;
    setQueuedSubmissions(next);
  }

  function createSendInput(
    activeSession: ChatSession,
    content: string,
  ): SendChatMessageInput {
    return {
      agentCode: normalizeOptionalValue(config.agentCode),
      appCode: normalizeOptionalValue(activeSession.appCode ?? config.appCode),
      content,
      taskCode: normalizeOptionalValue(activeSession.taskCode ?? config.taskCode),
      workspaceCode: activeSession.workspaceCode,
    };
  }

  function enqueueSubmission(
    activeSession: ChatSession,
    input: SendChatMessageInput,
    position: "front" | "back" = "back",
  ) {
    queuedSubmissionSequenceRef.current += 1;
    const submission: ChatQueuedSubmission = {
      id: `QUEUED-${Date.now()}-${queuedSubmissionSequenceRef.current}`,
      input,
      sessionCode: activeSession.sessionCode,
    };
    replaceQueuedSubmissions(
      position === "front"
        ? [submission, ...queuedSubmissionsRef.current]
        : enqueueChatSubmission(queuedSubmissionsRef.current, submission),
    );
    return submission;
  }

  function takeNextQueuedSubmission(sessionCode: string) {
    const result = takeNextQueuedChatSubmission(
      queuedSubmissionsRef.current,
      sessionCode,
    );
    replaceQueuedSubmissions(result.remaining);
    return result.submission;
  }

  useEffect(() => {
    setSession((current) => {
      if (!normalizedInitialSession) {
        return current ? null : current;
      }

      if (
        current
        && current.sessionCode === normalizedInitialSession.sessionCode
        && current.updatedAt === normalizedInitialSession.updatedAt
        && current.lastRunCode === normalizedInitialSession.lastRunCode
        && current.lastRunStatus === normalizedInitialSession.lastRunStatus
        && current.title === normalizedInitialSession.title
      ) {
        return current;
      }

      return normalizedInitialSession;
    });
  }, [normalizedInitialSession]);

  useEffect(() => {
    const nextSessionCode = session?.sessionCode ?? "";
    const previousSessionCode = activeSessionCodeRef.current;
    if (
      previousSessionCode
      && previousSessionCode !== nextSessionCode
    ) {
      detachRunObserver("detach");
      detachSteerObserver();
    }
    if (previousSessionCode !== nextSessionCode) {
      replaceRemoteActiveRun(null);
      replaceActiveRunStatusPending(false);
      observedRemoteRunRef.current = null;
    }
    activeSessionCodeRef.current = nextSessionCode;
    onSessionChange?.(session);
  }, [onSessionChange, session]);

  useEffect(() => () => {
    detachRunObserver("detach", false);
    detachSteerObserver(false);
  }, []);

  useEffect(() => {
    let active = true;
    const abortController = new AbortController();
    async function loadSessionState() {
      if (!session?.sessionCode) {
        setMessages([]);
        setAguiEvents([]);
        setProtocolEvents([]);
        replaceRemoteActiveRun(null);
        return;
      }
      setLoading(true);
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents, nextActiveRun] = await Promise.all([
          activeAdapter.loadMessages(session.sessionCode),
          aguiEnabled ? activeAdapter.loadAguiEvents(session.sessionCode) : Promise.resolve([]),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(session.sessionCode)
            : Promise.resolve([]),
          loadActiveRunSnapshot(session.sessionCode, abortController.signal)
            .catch(() => undefined),
        ]);
        if (!active) return;
        setMessages(nextMessages);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
        if (nextActiveRun !== undefined) {
          replaceRemoteActiveRun(nextActiveRun);
          if (nextActiveRun) {
            replaceActiveRunStatusPending(false);
          } else if (
            activeRunStatusPendingRef.current
            && !sendingRef.current
          ) {
            replaceActiveRunStatusPending(false);
            const nextSubmission = takeNextQueuedSubmission(
              session.sessionCode,
            );
            if (nextSubmission) {
              void runMessage(
                session,
                nextSubmission.input,
                nextSubmission,
              );
            }
          }
        }
      } catch (error) {
        if (!active || abortController.signal.aborted) return;
        onError(error instanceof Error ? error.message : "Chat data could not be loaded", "Chat load failed");
      } finally {
        if (active) setLoading(false);
      }
    }
    void loadSessionState();
    return () => {
      active = false;
      abortController.abort();
    };
  }, [activeAdapter, aguiEnabled, onError, protocolReplayEnabled, session?.sessionCode]);

  const awaitingRemoteTerminal = Boolean(
    session?.sessionCode
    && observedRemoteRunRef.current?.sessionCode === session.sessionCode,
  );
  const shouldPollData = Boolean(
    session?.sessionCode
    && (
      runActive
      || activeRunStatusPending
      || awaitingRemoteTerminal
      || isRecentPendingUserTurn(messages)
      || (protocolReplayEnabled && openAguiProtocolRunCodes(protocolEvents, session.sessionCode).size > 0)
    ),
  );

  useEffect(() => {
    if (!session?.sessionCode || !shouldPollData) return;
    const intervalId = window.setInterval(async () => {
      if (!session?.sessionCode) return;
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents, nextActiveRun] = await Promise.all([
          activeAdapter.loadMessages(session.sessionCode),
          aguiEnabled ? activeAdapter.loadAguiEvents(session.sessionCode) : Promise.resolve([]),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(session.sessionCode)
            : Promise.resolve([]),
          loadActiveRunSnapshot(session.sessionCode).catch(() => undefined),
        ]);
        if (activeSessionCodeRef.current !== session.sessionCode) return;
        setMessages(nextMessages);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
        if (nextActiveRun !== undefined) {
          replaceRemoteActiveRun(nextActiveRun);
          if (nextActiveRun) {
            replaceActiveRunStatusPending(false);
          } else if (
            activeRunStatusPendingRef.current
            && !sendingRef.current
          ) {
            replaceActiveRunStatusPending(false);
            const nextSubmission = takeNextQueuedSubmission(
              session.sessionCode,
            );
            if (nextSubmission) {
              void runMessage(
                session,
                nextSubmission.input,
                nextSubmission,
              );
            }
          }
        }
      } catch {
        // Keep last successful snapshot until the next poll.
      }
    }, 1000);
    return () => window.clearInterval(intervalId);
  }, [activeAdapter, aguiEnabled, messages, protocolEvents, protocolReplayEnabled, session?.sessionCode, shouldPollData]);

  useEffect(() => {
    const sessionCode = session?.sessionCode;
    if (!sessionCode) {
      observedRemoteRunRef.current = null;
      return;
    }
    if (remoteActiveRun) {
      observedRemoteRunRef.current = {
        runCode: remoteActiveRun.runCode,
        sessionCode,
      };
      return;
    }

    const observed = observedRemoteRunRef.current;
    if (!observed || observed.sessionCode !== sessionCode) return;
    if (
      protocolReplayEnabled
      && !protocolEvents.some((event) => (
        String(event.runId ?? event.runCode ?? "") === observed.runCode
        && ["RUN_ERROR", "RUN_FINISHED"].includes(
          String(event.type).toUpperCase(),
        )
      ))
    ) {
      return;
    }
    observedRemoteRunRef.current = null;
    const runKey = `${sessionCode}:${observed.runCode}`;
    const autoDrainSuppressed = suppressedAutoDrainRunKeysRef.current.delete(runKey);
    if (
      sendingRef.current
      || drainedRemoteRunKeysRef.current.has(runKey)
      || autoDrainSuppressed
    ) {
      drainedRemoteRunKeysRef.current.add(runKey);
      return;
    }
    drainedRemoteRunKeysRef.current.add(runKey);
    const nextSubmission = takeNextQueuedSubmission(sessionCode);
    if (nextSubmission && session.sessionCode === activeSessionCodeRef.current) {
      void runMessage(session, nextSubmission.input, nextSubmission);
    }
  }, [protocolEvents, protocolReplayEnabled, remoteActiveRun, session]);

  const sendDisabled = useMemo(() => {
    if (loading || config.disabledInput) return true;
    if (!draft.trim()) return true;
    return !session && !config.workspaceCode;
  }, [config.disabledInput, config.workspaceCode, draft, loading, session]);
  const queueDisabled = useMemo(
    () => loading || Boolean(config.disabledInput) || !draft.trim() || !session,
    [config.disabledInput, draft, loading, session],
  );
  const steerDisabled = queueDisabled || steering || !canSteer;

  async function ensureSession() {
    if (session) return session;
    if (!config.workspaceCode) {
      onError("Workspace code is required before starting a session.", "Session create failed");
      return null;
    }
    const created = await activeAdapter.createSession({
      agentCode: normalizeOptionalValue(config.agentCode),
      appCode: normalizeOptionalValue(config.appCode),
      taskCode: normalizeOptionalValue(config.taskCode),
      title: normalizeOptionalValue(config.title) ?? floatingTitle,
      workspaceCode: config.workspaceCode,
    });
    activeSessionCodeRef.current = created.sessionCode;
    setSession(created);
    setMessages([]);
    setAguiEvents([]);
    setProtocolEvents([]);
    return created;
  }

  async function runMessage(
    activeSession: ChatSession,
    sendInput: SendChatMessageInput,
    sourceSubmission: ChatQueuedSubmission | null = null,
  ) {
    const queueGeneration = queueGenerationRef.current;
    const restoreSourceSubmission = () => {
      if (!sourceSubmission || queueGeneration !== queueGenerationRef.current) return;
      const remaining = removeQueuedChatSubmission(
        queuedSubmissionsRef.current,
        sourceSubmission.id,
      );
      replaceQueuedSubmissions([sourceSubmission, ...remaining]);
    };
    if (sendingRef.current || remoteActiveRunRef.current) {
      restoreSourceSubmission();
      return false;
    }
    const content = sendInput.content.trim();
    if (!content) {
      restoreSourceSubmission();
      return false;
    }
    const autoDrainEpoch = autoDrainEpochRef.current;
    const abortController = new AbortController();
    abortRef.current = abortController;
    streamAbortReasonsRef.current.set(abortController, "none");
    currentLiveAguiRunCodesRef.current.clear();
    activeRunCodeRef.current = "";
    setActiveRunCode("");
    setActiveRunSupportsSteer(null);
    optimisticMessageSequenceRef.current += 1;
    const optimisticKey = `${Date.now()}-${optimisticMessageSequenceRef.current}`;
    const optimisticCreatedAt = new Date().toISOString();
    const optimisticUserCode = `LOCAL-${optimisticKey}-USER`;
    const optimisticAssistantCode = `LOCAL-${optimisticKey}-ASSISTANT`;
    setSendingState(true);
    setLiveAguiPending(aguiEnabled);
    setMessages((current) => [
      ...current,
      {
        actor: currentUserDisplayName || "User",
        channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
        content,
        createdAt: optimisticCreatedAt,
        id: -Date.now() - optimisticMessageSequenceRef.current,
        messageCode: optimisticUserCode,
        role: "USER",
        runCode: null,
        sessionCode: activeSession.sessionCode,
      },
      {
        actor: activeSession.agentCode ?? config.agentCode ?? "Assistant",
        channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
        content: "",
        createdAt: optimisticCreatedAt,
        id: -Date.now() - optimisticMessageSequenceRef.current - 1,
        messageCode: optimisticAssistantCode,
        role: "ASSISTANT",
        runCode: null,
        sessionCode: activeSession.sessionCode,
      },
    ]);

    let completed = false;
    let streamSucceeded = false;
    let runConflictDetected = false;
    // Once a request crosses the transport boundary, automatic retry is unsafe:
    // the backend may already have persisted its USER message. The sole safe
    // restore path is CHAT_RUN_ALREADY_ACTIVE, which rejects before admission.
    let shouldRestoreSourceSubmission = false;
    let completedSession: ChatSession | null = null;
    try {
      const result = await activeAdapter.streamMessage(
        activeSession,
        sendInput,
        {
          onAguiEvent: (event) => {
            if (abortRef.current !== abortController) return;
            const runCode = String(event.runId ?? event.runCode ?? "");
            if (typeof event.supportsSteer === "boolean") {
              setActiveRunSupportsSteer(event.supportsSteer);
            }
            if (runCode && !activeRunCodeRef.current) {
              activeRunCodeRef.current = runCode;
              setActiveRunCode(runCode);
            }
            if (!aguiEnabled) return;
            setLiveAguiEvents((current) => [...current, event]);
            if (runCode) {
              currentLiveAguiRunCodesRef.current.add(runCode);
              setLiveAguiAnchorMessageCodes((current) => (
                current[runCode]
                  ? current
                  : { ...current, [runCode]: optimisticUserCode }
              ));
            }
          },
          onDelta: (delta) => {
            if (abortRef.current !== abortController) return;
            setLiveAguiPending(false);
            setMessages((current) => current.map((message) => (
              message.messageCode === optimisticAssistantCode
                ? { ...message, content: `${message.content}${delta}` }
                : message
            )));
          },
        },
        abortController.signal,
      );
      const resolvedAbortReason = currentStreamAbortReason(abortController);
      if (resolvedAbortReason !== "none") {
        streamSucceeded = true;
        shouldRestoreSourceSubmission = false;
        // A transport is allowed to settle normally while its observer is being
        // aborted. Route that race through regular abort handling without
        // committing stale UI state or re-sending an already accepted input.
        throw new Error("Chat stream observer closed");
      }
      if (activeSessionCodeRef.current !== activeSession.sessionCode) {
        streamSucceeded = true;
        shouldRestoreSourceSubmission = false;
        return false;
      }
      streamSucceeded = true;
      shouldRestoreSourceSubmission = false;
      setSession(result.session);
      const nextMessages = await activeAdapter
        .loadMessages(result.session.sessionCode)
        .catch(() => null);
      if (nextMessages) {
        setMessages(nextMessages);
      } else {
        setMessages((current) => mergeChatMessagesChronologically(
          current,
          result.messages,
          [optimisticUserCode, optimisticAssistantCode],
        ));
      }
      if (aguiEnabled) {
        const [nextAguiEvents, nextProtocolEvents] = await Promise.all([
          activeAdapter
            .loadAguiEvents(result.session.sessionCode)
            .catch(() => aguiEvents),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter
                .loadProtocolEvents(result.session.sessionCode)
                .catch(() => protocolEvents)
            : Promise.resolve([]),
        ]);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
      }
      setLiveAguiEvents([]);
      setLiveAguiAnchorMessageCodes({});
      completedSession = result.session;
      completed = currentStreamAbortReason(abortController) === "none"
        && autoDrainEpochRef.current === autoDrainEpoch;
    } catch (error) {
      const abortReason = currentStreamAbortReason(abortController);
      const runConflict = error instanceof FetchTransportError
        && error.code === "CHAT_RUN_ALREADY_ACTIVE";
      runConflictDetected = runConflict;
      const stillViewingSession = activeSessionCodeRef.current === activeSession.sessionCode;
      if (stillViewingSession) {
        setMessages((current) => current.filter((message) => (
          runConflict
            ? message.messageCode !== optimisticUserCode
              && message.messageCode !== optimisticAssistantCode
            : message.messageCode !== optimisticAssistantCode
              || Boolean(message.content.trim())
        )));
      }
      if (runConflict) {
        if (sourceSubmission) {
          restoreSourceSubmission();
        } else {
          enqueueSubmission(activeSession, sendInput, "front");
        }
        shouldRestoreSourceSubmission = false;
        replaceActiveRunStatusPending(true);
        await refreshActiveRun(activeSession.sessionCode);
        onInfo?.("Another response is already running; your message was queued.", "Message queued");
      } else if (abortReason === "stop") {
        streamSucceeded = true;
        shouldRestoreSourceSubmission = false;
        onInfo?.("Current response has been stopped", "Chat stopped");
      } else if (abortReason === "detach") {
        shouldRestoreSourceSubmission = false;
      } else {
        onError(error instanceof Error ? error.message : "Chat send failed", "Chat send failed");
      }
    } finally {
      const ownsObserver = abortRef.current === abortController;
      if (ownsObserver) {
        setSendingState(false);
        setLiveAguiPending(false);
        abortRef.current = null;
        activeRunCodeRef.current = "";
        setActiveRunCode("");
        setActiveRunSupportsSteer(null);
      }
    }

    if (!streamSucceeded && shouldRestoreSourceSubmission) {
      restoreSourceSubmission();
    }
    if (
      runConflictDetected
      && activeRunStatusPendingRef.current
      && activeSessionCodeRef.current === activeSession.sessionCode
    ) {
      void refreshActiveRun(activeSession.sessionCode);
    }

    if (
      completed
      && completedSession
      && activeSessionCodeRef.current === activeSession.sessionCode
    ) {
      const nextSubmission = takeNextQueuedSubmission(activeSession.sessionCode);
      if (nextSubmission) {
        void runMessage(completedSession, nextSubmission.input, nextSubmission);
      }
    }
    return completed;
  }

  async function handleSend() {
    if (sendingRef.current || sendDisabled) return;
    const activeSession = await ensureSession();
    if (!activeSession) return;
    const content = draft.trim();
    if (!content) return;
    const sendInput = createSendInput(activeSession, content);
    setDraft("");
    if (remoteActiveRunRef.current) {
      enqueueSubmission(activeSession, sendInput);
      return;
    }
    await runMessage(activeSession, sendInput);
  }

  async function handleQueue() {
    if (queueDisabled) return;
    const activeSession = await ensureSession();
    if (!activeSession) return;
    const content = draft.trim();
    if (!content) return;
    const sendInput = createSendInput(activeSession, content);
    setDraft("");
    if (sendingRef.current || remoteActiveRunRef.current) {
      enqueueSubmission(activeSession, sendInput);
      return;
    }
    await runMessage(activeSession, sendInput);
  }

  async function handleSteer() {
    if (steeringRef.current || steerDisabled) return;
    const activeSession = session;
    if (!activeSession) return;
    const content = draft.trim();
    if (!content) return;
    const sendInput = createSendInput(activeSession, content);
    const expectedRunCode = activeRunCodeRef.current
      || remoteActiveRunRef.current?.runCode
      || "";
    const queueGeneration = queueGenerationRef.current;
    const autoDrainEpoch = autoDrainEpochRef.current;
    setDraft("");
    if (
      !expectedRunCode
      || !activeAdapter.capabilities.supportsSteer
      || !activeAdapter.steerMessage
    ) {
      enqueueSubmission(activeSession, sendInput);
      return;
    }

    const abortController = new AbortController();
    steerAbortRef.current = abortController;
    steeringRef.current = true;
    setSteering(true);
    try {
      const result = await activeAdapter.steerMessage(
        activeSession,
        sendInput,
        { expectedRunCode },
        abortController.signal,
      );
      if (steerAbortRef.current !== abortController) {
        // A resolved ACK means the backend accepted and persisted the steer.
        // Ignore a stale observer result without sending the same input again.
        return;
      }
      if (activeSessionCodeRef.current === result.sessionCode) {
        setMessages((current) => mergeChatMessagesChronologically(
          current,
          [result.message],
        ));
      }
    } catch (error) {
      const observerDetached = steerAbortRef.current !== abortController;
      const safeQueueFallback = error instanceof FetchTransportError
        && isSteerQueueFallbackCode(error.code);
      if (safeQueueFallback && queueGenerationRef.current === queueGeneration) {
        enqueueSubmission(activeSession, sendInput, "front");
        const nextActiveRun = await refreshActiveRun(activeSession.sessionCode);
        if (
          !sendingRef.current
          && nextActiveRun === null
          && autoDrainEpochRef.current === autoDrainEpoch
          && activeSessionCodeRef.current === activeSession.sessionCode
        ) {
          const nextSubmission = takeNextQueuedSubmission(
            activeSession.sessionCode,
          );
          if (nextSubmission) {
            void runMessage(activeSession, nextSubmission.input, nextSubmission);
          }
        }
      }
      const stopped = error instanceof Error && error.name === "AbortError";
      if (safeQueueFallback) {
        onInfo?.("The active run rejected steer; the message was queued", "Message queued");
      } else if (observerDetached || stopped) {
        onInfo?.("Steer delivery was not confirmed and was not retried", "Steer not retried");
      } else {
        onError(
          error instanceof Error ? error.message : "Chat steer failed",
          "Chat steer failed",
        );
      }
    } finally {
      if (steerAbortRef.current === abortController) {
        steerAbortRef.current = null;
        steeringRef.current = false;
        setSteering(false);
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

  function handleEditQueuedMessage(id: string, content: string) {
    replaceQueuedSubmissions(
      removeQueuedChatSubmission(queuedSubmissionsRef.current, id),
    );
    setDraft(content);
  }

  function handleRemoveQueuedMessage(id: string) {
    replaceQueuedSubmissions(
      removeQueuedChatSubmission(queuedSubmissionsRef.current, id),
    );
  }

  async function handleStop() {
    if (!session || stopping) return;
    let stoppedRunCode = effectiveActiveRunCode;
    if (
      !stoppedRunCode
      && activeAdapter.capabilities.supportsStop
      && activeAdapter.stopRun
    ) {
      stoppedRunCode = (await refreshActiveRun(session.sessionCode))?.runCode ?? "";
    }
    const runKey = stoppedRunCode
      ? `${session.sessionCode}:${stoppedRunCode}`
      : "";
    if (runKey) suppressedAutoDrainRunKeysRef.current.add(runKey);
    autoDrainEpochRef.current += 1;
    detachSteerObserver();
    const observedLocally = Boolean(abortRef.current);
    if (
      activeAdapter.capabilities.supportsStop
      && activeAdapter.stopRun
      && stoppedRunCode
    ) {
      setStopping(true);
      if (remoteActiveRunRef.current?.runCode === stoppedRunCode) {
        replaceRemoteActiveRun({
          ...remoteActiveRunRef.current,
          state: "FINISHING",
        });
      }
      try {
        await activeAdapter.stopRun(
          session,
          { expectedRunCode: stoppedRunCode },
        );
        detachRunObserver("stop");
        if (!observedLocally) {
          onInfo?.("Current response has been stopped", "Chat stopped");
        }
        await refreshActiveRun(session.sessionCode);
      } catch (error) {
        if (runKey) suppressedAutoDrainRunKeysRef.current.delete(runKey);
        onError(
          error instanceof Error ? error.message : "Chat stop failed",
          "Chat stop failed",
        );
        await refreshActiveRun(session.sessionCode);
      } finally {
        setStopping(false);
      }
      return;
    }
    detachRunObserver("stop");
  }

  function handleNewSession() {
    queueGenerationRef.current += 1;
    detachRunObserver("detach");
    detachSteerObserver();
    setSession(null);
    setMessages([]);
    setAguiEvents([]);
    setProtocolEvents([]);
    setLiveAguiEvents([]);
    setLiveAguiAnchorMessageCodes({});
    activeRunCodeRef.current = "";
    setActiveRunCode("");
    setActiveRunSupportsSteer(null);
    replaceRemoteActiveRun(null);
    replaceQueuedSubmissions([]);
  }

  const presentationDebugEvents = useMemo(
    () => buildTranscriptPresentation({
      debugOptions,
      liveAguiAnchorMessageCodes,
      liveAguiEvents,
      liveAguiPending,
      messages,
      protocolEvents,
      sessionCode: session?.sessionCode ?? "",
    }).debugEventsByRunCode,
    [debugOptions, liveAguiAnchorMessageCodes, liveAguiEvents, liveAguiPending, messages, protocolEvents, session?.sessionCode],
  );

  const resolvedDebugOptions = useMemo(() => ({
    ...debugOptions,
    debugEventTypes: debugOptions?.debugEventTypes?.length
      ? Array.from(new Set([
        ...debugOptions.debugEventTypes,
        ...Array.from(presentationDebugEvents.keys()).flatMap((runCode) => (
          mergeTranscriptAguiEvents(protocolEvents, liveAguiEvents)
            .filter((event) => (event.runId ?? event.runCode ?? "") === runCode)
            .map((event) => String(event.type).toUpperCase())
        )),
      ]))
      : debugOptions?.debugEventTypes,
  }), [debugOptions, liveAguiEvents, presentationDebugEvents, protocolEvents]);

  const panel = (
    <TronChatPanel
      createSessionLabel="Start chat"
      currentUserDisplayName={currentUserDisplayName}
      debugOptions={resolvedDebugOptions}
      disabledInput={Boolean(config.disabledInput)}
      draft={draft}
      emptyDescription={config.workspaceCode
        ? "Open a session and start talking to your configured agent."
        : "Set a workspace code before starting a session."}
      footerAddon={footerAddon}
      headerContent={headerContent}
      inputPlaceholder={config.inputPlaceholder}
      liveAguiAnchorMessageCodes={liveAguiAnchorMessageCodes}
      liveAguiEvents={liveAguiEvents}
      liveAguiPending={liveAguiPending}
      messages={messages}
      onCreateSession={() => {
        void ensureSession();
      }}
      onNewSession={handleNewSession}
      onDraftChange={setDraft}
      onEditMessage={setDraft}
      onEditQueuedMessage={handleEditQueuedMessage}
      onRemoveQueuedMessage={handleRemoveQueuedMessage}
      onSend={(mode) => { void handleSubmit(mode); }}
      onStop={() => { void handleStop(); }}
      protocolEvents={protocolEvents}
      queueDisabled={queueDisabled}
      queuedMessages={visibleQueuedSubmissions}
      runningSubmitModes={runningSubmitModes}
      sendDisabled={sendDisabled}
      sending={runActive}
      session={session}
      showAgui={aguiEnabled}
      showHeader={!config.floating}
      showStop={canStop}
      sidebarContent={sidebarContent}
      steerDisabled={steerDisabled}
      title={config.title || floatingTitle}
    />
  );

  if (config.floating !== false) {
    return (
      <TronChatWindow
        currentUserDisplayName={currentUserDisplayName}
        debugOptions={resolvedDebugOptions}
        disabledInput={Boolean(config.disabledInput)}
        draft={draft}
        emptyDescription={config.workspaceCode
          ? "Open a session and start talking to your configured agent."
          : "Set a workspace code before starting a session."}
        footerAddon={footerAddon}
        inputPlaceholder={config.inputPlaceholder}
        liveAguiAnchorMessageCodes={liveAguiAnchorMessageCodes}
        liveAguiEvents={liveAguiEvents}
        liveAguiPending={liveAguiPending}
        messages={messages}
        onCreateSession={() => { void ensureSession(); }}
        onNewSession={handleNewSession}
        onDraftChange={setDraft}
        onEditMessage={setDraft}
        onEditQueuedMessage={handleEditQueuedMessage}
        onRemoveQueuedMessage={handleRemoveQueuedMessage}
        onSend={(mode) => { void handleSubmit(mode); }}
        onStop={() => { void handleStop(); }}
        protocolEvents={protocolEvents}
        queueDisabled={queueDisabled}
        queuedMessages={visibleQueuedSubmissions}
        runningSubmitModes={runningSubmitModes}
        sendDisabled={sendDisabled}
        sending={runActive}
        session={session}
        showAgui={aguiEnabled}
        showStop={canStop}
        sidebarContent={sidebarContent}
        steerDisabled={steerDisabled}
        state={windowState}
        storageKey={config.storageKey || "tron.chat.runtime.window"}
        title={floatingTitle}
        titleSubtitle={floatingSubtitle}
        onStateChange={onWindowStateChange}
      />
    );
  }

  return panel;
}

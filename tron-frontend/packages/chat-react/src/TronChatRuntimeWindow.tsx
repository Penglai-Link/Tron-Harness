import {
  buildTranscriptPresentation,
  createMockChatAdapter,
  isRecentPendingUserTurn,
  mergeTranscriptAguiEvents,
  normalizeChatSession,
  openAguiProtocolRunCodes,
  type ChatFloatingState,
  type ChatAdapter,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatClientConfig,
  type ChatDebugOptions,
  type ChatMessage,
  type ChatSession,
  type ChatSendResult,
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

function dedupeMessages(messages: ChatMessage[]) {
  const seen = new Set<string>();
  return messages.filter((message) => {
    if (seen.has(message.messageCode)) return false;
    seen.add(message.messageCode);
    return true;
  });
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
  const activeAdapter = adapter ?? createMockChatAdapter();
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
  const [loading, setLoading] = useState(false);
  const [liveAguiPending, setLiveAguiPending] = useState(false);
  const activeSessionCodeRef = useRef("");
  const abortRef = useRef<AbortController | null>(null);
  const streamAbortRequestedRef = useRef(false);
  const currentLiveAguiRunCodesRef = useRef<Set<string>>(new Set());
  const aguiEnabled = config.showAgui !== false && activeAdapter.capabilities.supportsAgui;
  const protocolReplayEnabled = aguiEnabled
    && activeAdapter.capabilities.supportsProtocolReplay
    && Boolean(activeAdapter.loadProtocolEvents);

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
    activeSessionCodeRef.current = session?.sessionCode ?? "";
    onSessionChange?.(session);
  }, [onSessionChange, session]);

  useEffect(() => {
    let active = true;
    async function loadSessionState() {
      if (!session?.sessionCode) {
        setMessages([]);
        setAguiEvents([]);
        setProtocolEvents([]);
        return;
      }
      setLoading(true);
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents] = await Promise.all([
          activeAdapter.loadMessages(session.sessionCode),
          aguiEnabled ? activeAdapter.loadAguiEvents(session.sessionCode) : Promise.resolve([]),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(session.sessionCode)
            : Promise.resolve([]),
        ]);
        if (!active) return;
        setMessages(nextMessages);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
      } catch (error) {
        if (!active) return;
        onError(error instanceof Error ? error.message : "Chat data could not be loaded", "Chat load failed");
      } finally {
        if (active) setLoading(false);
      }
    }
    void loadSessionState();
    return () => {
      active = false;
    };
  }, [activeAdapter, aguiEnabled, onError, protocolReplayEnabled, session?.sessionCode]);

  const shouldPollData = Boolean(
    session?.sessionCode
    && !sending
    && (
      isRecentPendingUserTurn(messages)
      || (protocolReplayEnabled && openAguiProtocolRunCodes(protocolEvents, session.sessionCode).size > 0)
    ),
  );

  useEffect(() => {
    if (!session?.sessionCode || (!sending && !shouldPollData)) return;
    const intervalId = window.setInterval(async () => {
      if (!session?.sessionCode) return;
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents] = await Promise.all([
          activeAdapter.loadMessages(session.sessionCode),
          aguiEnabled ? activeAdapter.loadAguiEvents(session.sessionCode) : Promise.resolve([]),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(session.sessionCode)
            : Promise.resolve([]),
        ]);
        setMessages(nextMessages);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
      } catch {
        // Keep last successful snapshot until the next poll.
      }
    }, 1000);
    return () => window.clearInterval(intervalId);
  }, [activeAdapter, aguiEnabled, messages, protocolEvents, protocolReplayEnabled, sending, session?.sessionCode, shouldPollData]);

  const sendDisabled = useMemo(() => {
    if (loading || sending || config.disabledInput) return true;
    if (!draft.trim()) return true;
    return !session && !config.workspaceCode;
  }, [config.disabledInput, config.workspaceCode, draft, loading, sending, session]);

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
    setSession(created);
    setMessages([]);
    setAguiEvents([]);
    setProtocolEvents([]);
    return created;
  }

  async function handleSend() {
    if (sendDisabled) return;
    const activeSession = await ensureSession();
    if (!activeSession) return;
    const content = draft.trim();
    if (!content) return;
    const sendInput: SendChatMessageInput = {
      agentCode: normalizeOptionalValue(config.agentCode),
      appCode: normalizeOptionalValue(activeSession.appCode ?? config.appCode),
      content,
      taskCode: normalizeOptionalValue(activeSession.taskCode ?? config.taskCode),
      workspaceCode: activeSession.workspaceCode,
    };
    const abortController = new AbortController();
    abortRef.current = abortController;
    streamAbortRequestedRef.current = false;
    currentLiveAguiRunCodesRef.current.clear();
    const optimisticCreatedAt = new Date().toISOString();
    const optimisticUserCode = `LOCAL-${Date.now()}-USER`;
    const optimisticAssistantCode = `LOCAL-${Date.now()}-ASSISTANT`;
    setSending(true);
    setLiveAguiPending(aguiEnabled);
    setDraft("");
    setMessages((current) => [
      ...current,
      {
        actor: currentUserDisplayName || "User",
        channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
        content,
        createdAt: optimisticCreatedAt,
        id: -Date.now(),
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
        id: -Date.now() - 1,
        messageCode: optimisticAssistantCode,
        role: "ASSISTANT",
        runCode: null,
        sessionCode: activeSession.sessionCode,
      },
    ]);

    try {
      const result = await activeAdapter.streamMessage(
        activeSession,
        sendInput,
        {
          onAguiEvent: (event) => {
            if (!aguiEnabled) return;
            setLiveAguiEvents((current) => [...current, event]);
            const runCode = event.runId ?? "";
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
      setSession(result.session);
      setMessages((current) => dedupeMessages([
        ...current.filter((message) => (
          message.messageCode !== optimisticUserCode && message.messageCode !== optimisticAssistantCode
        )),
        ...result.messages,
      ]));
      if (aguiEnabled) {
        const [nextAguiEvents, nextProtocolEvents] = await Promise.all([
          activeAdapter.loadAguiEvents(result.session.sessionCode),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(result.session.sessionCode)
            : Promise.resolve([]),
        ]);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
      }
      setLiveAguiEvents([]);
      setLiveAguiAnchorMessageCodes({});
    } catch (error) {
      const stopped = streamAbortRequestedRef.current || (error instanceof Error && error.name === "AbortError");
      setMessages((current) => current.filter((message) => message.messageCode !== optimisticAssistantCode || Boolean(message.content.trim())));
      if (stopped) {
        onInfo?.("Current response has been stopped", "Chat stopped");
      } else {
        onError(error instanceof Error ? error.message : "Chat send failed", "Chat send failed");
      }
    } finally {
      setSending(false);
      setLiveAguiPending(false);
      abortRef.current = null;
      streamAbortRequestedRef.current = false;
    }
  }

  function handleStop() {
    streamAbortRequestedRef.current = true;
    abortRef.current?.abort();
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
      onDraftChange={setDraft}
      onEditMessage={setDraft}
      onSend={() => { void handleSend(); }}
      onStop={handleStop}
      protocolEvents={protocolEvents}
      sendDisabled={sendDisabled}
      sending={sending}
      session={session}
      showAgui={aguiEnabled}
      showHeader={!config.floating}
      sidebarContent={sidebarContent}
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
        onDraftChange={setDraft}
        onEditMessage={setDraft}
        onSend={() => { void handleSend(); }}
        onStop={handleStop}
        protocolEvents={protocolEvents}
        sendDisabled={sendDisabled}
        sending={sending}
        session={session}
        showAgui={aguiEnabled}
        sidebarContent={sidebarContent}
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

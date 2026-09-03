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
  type ChatAttachmentLabels,
  type ChatMessageAttachment,
  type ChatClientConfig,
  type ChatDebugOptions,
  type ChatMessage,
  type ChatQueuedSubmission,
  type ChatRunningSubmitMode,
  type ChatRuntimeRunState,
  type ChatSession,
  type ChatSendResult,
  type ChatSubmitMode,
  type ChatUserInputAnswer,
  type ChatUserInputRequest,
  type SendChatMessageInput,
} from "@tron/chat-core";
import { FileText, Paperclip, X } from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type Ref,
  type ReactNode,
  type SetStateAction,
} from "react";
import { TronChatPanel, type ChatMarkdownLinkRenderer } from "./TronChatPanel";
import { TronChatWindow } from "./TronChatWindow";
import { isImageAttachment, useAttachmentImageSource } from "./chatAttachments";

interface ToastLikeCallback {
  (message: string, title?: string): void;
}

export interface TronChatRuntimeWindowProps {
  adapter?: ChatAdapter;
  attachmentLabels?: ChatAttachmentLabels;
  autoFocusComposer?: boolean;
  config: ChatClientConfig;
  composerRef?: Ref<HTMLTextAreaElement>;
  composerHeader?: ReactNode;
  createSessionLabel?: string;
  currentUserDisplayName?: string;
  draft?: string;
  debugOptions?: ChatDebugOptions;
  emptyDescription?: string;
  emptyTitle?: string;
  floatingTitle?: string;
  floatingSubtitle?: string;
  formatAnnotationCount?: (count: number) => string;
  footerAddon?: ReactNode;
  headerContent?: ReactNode;
  initialSession?: ChatSession | null;
  onCreateSession?: () => void;
  onDraftChange?: (draft: string) => void;
  onError?: ToastLikeCallback;
  onInfo?: ToastLikeCallback;
  onPendingAttachmentsChange?: (attachments: ChatMessageAttachment[]) => void;
  onRunStateChange?: (state: ChatRuntimeRunState) => void;
  onSessionChange?: (session: ChatSession | null) => void;
  pendingAttachments?: ChatMessageAttachment[];
  createSendInput?: (
    session: ChatSession,
    content: string,
    attachments: ChatMessageAttachment[],
  ) => SendChatMessageInput | null;
  showAttachmentControl?: boolean;
  showHeader?: boolean;
  sidebarContent?: ReactNode;
  windowState?: ChatFloatingState;
  onWindowStateChange?: (state: ChatFloatingState) => void;
  renderMarkdownLink?: ChatMarkdownLinkRenderer;
}

function defaultErrorReporter(message: string) {
  console.error(message);
}

function normalizeOptionalValue(value: string | null | undefined) {
  const normalized = value?.trim();
  return normalized ? normalized : undefined;
}

function attachmentLabel(attachment: ChatMessageAttachment) {
  return attachment.fileName || attachment.title || attachment.summary || attachment.id;
}

function attachmentTypeLabel(attachment: ChatMessageAttachment) {
  const name = attachment.fileName || attachment.title || "";
  const extension = name.includes(".") ? name.split(".").pop() : undefined;
  return (extension || attachment.contentType?.split("/").pop() || "FILE").toUpperCase();
}

function PendingAttachmentCard({
  attachment,
  downloadAttachment,
  onRemove,
}: {
  attachment: ChatMessageAttachment;
  downloadAttachment?: (attachment: ChatMessageAttachment) => Promise<Blob>;
  onRemove: (attachmentId: string) => void;
}) {
  const label = attachmentLabel(attachment);
  const imageSource = useAttachmentImageSource(attachment, downloadAttachment);

  if (isImageAttachment(attachment)) {
    return (
      <div className="tron-chat__runtime-attachment-image">
        {imageSource ? <img alt={label} src={imageSource} /> : null}
        <button
          aria-label={`Remove ${label}`}
          className="tron-chat__runtime-attachment-remove tron-chat__runtime-attachment-remove--floating"
          type="button"
          onClick={() => onRemove(attachment.id)}
        >
          <X size={13} />
        </button>
      </div>
    );
  }

  return (
    <div className="tron-chat__runtime-attachment-file">
      <span className="tron-chat__runtime-attachment-file-icon" aria-hidden="true">
        <FileText size={22} />
      </span>
      <span className="tron-chat__runtime-attachment-file-info">
        <span className="tron-chat__runtime-attachment-file-name" title={label}>{label}</span>
        <span className="tron-chat__runtime-attachment-file-type">{attachmentTypeLabel(attachment)}</span>
      </span>
      <button
        aria-label={`Remove ${label}`}
        className="tron-chat__runtime-attachment-remove tron-chat__runtime-attachment-remove--floating"
        type="button"
        onClick={() => onRemove(attachment.id)}
      >
        <X size={13} />
      </button>
    </div>
  );
}

export function TronChatRuntimeWindow({
  adapter,
  attachmentLabels,
  autoFocusComposer = false,
  composerHeader,
  config,
  composerRef,
  createSessionLabel = "Start chat",
  currentUserDisplayName,
  draft: controlledDraft,
  debugOptions,
  emptyDescription,
  emptyTitle,
  floatingSubtitle = "Workspace assistant",
  floatingTitle = "Main Agent",
  formatAnnotationCount,
  footerAddon,
  headerContent,
  initialSession = null,
  onCreateSession,
  onDraftChange,
  onError = defaultErrorReporter,
  onInfo,
  onPendingAttachmentsChange,
  onRunStateChange,
  onSessionChange,
  pendingAttachments: controlledPendingAttachments,
  createSendInput: createSendInputOverride,
  showAttachmentControl = true,
  showHeader,
  sidebarContent,
  windowState,
  onWindowStateChange,
  renderMarkdownLink,
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
  const [userInputRequest, setUserInputRequest] = useState<ChatUserInputRequest | null>(null);
  const [userInputRequests, setUserInputRequests] = useState<ChatUserInputRequest[]>([]);
  const [userInputAnswerPending, setUserInputAnswerPending] = useState(false);
  const [aguiEvents, setAguiEvents] = useState<ChatAguiEvent[]>([]);
  const [protocolEvents, setProtocolEvents] = useState<ChatAguiStreamEvent[]>([]);
  const [liveAguiEvents, setLiveAguiEvents] = useState<ChatAguiStreamEvent[]>([]);
  const [liveAguiAnchorMessageCodes, setLiveAguiAnchorMessageCodes] = useState<Record<string, string>>({});
  const [liveAguiPendingAnchorMessageCode, setLiveAguiPendingAnchorMessageCode] = useState("");
  const [internalDraft, setInternalDraft] = useState("");
  const [internalPendingAttachments, setInternalPendingAttachments] = useState<ChatMessageAttachment[]>([]);
  const [attachmentUploadPending, setAttachmentUploadPending] = useState(false);
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
  const attachmentFileInputRef = useRef<HTMLInputElement | null>(null);
  const optimisticMessageSequenceRef = useRef(0);
  const currentLiveAguiRunCodesRef = useRef<Set<string>>(new Set());
  const observedRemoteRunRef = useRef<{ runCode: string; sessionCode: string } | null>(null);
  const drainedRemoteRunKeysRef = useRef<Set<string>>(new Set());
  const suppressedAutoDrainRunKeysRef = useRef<Set<string>>(new Set());
  const draftRef = useRef(controlledDraft ?? "");
  const pendingAttachmentsRef = useRef<ChatMessageAttachment[]>(controlledPendingAttachments ?? []);
  const draft = controlledDraft ?? internalDraft;
  const pendingAttachments = controlledPendingAttachments ?? internalPendingAttachments;

  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  useEffect(() => {
    pendingAttachmentsRef.current = pendingAttachments;
  }, [pendingAttachments]);

  function setDraft(nextValue: SetStateAction<string>) {
    const next = typeof nextValue === "function" ? nextValue(draftRef.current) : nextValue;
    draftRef.current = next;
    if (controlledDraft === undefined) setInternalDraft(next);
    onDraftChange?.(next);
  }

  function setPendingAttachments(nextValue: SetStateAction<ChatMessageAttachment[]>) {
    const next = typeof nextValue === "function"
      ? nextValue(pendingAttachmentsRef.current)
      : nextValue;
    pendingAttachmentsRef.current = next;
    if (controlledPendingAttachments === undefined) setInternalPendingAttachments(next);
    onPendingAttachmentsChange?.(next);
  }
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

  useEffect(() => {
    onRunStateChange?.({
      active: runActive,
      runCode: effectiveActiveRunCode,
      sessionCode: session?.sessionCode ?? "",
      supportsSteer: canSteer,
    });
  }, [
    canSteer,
    effectiveActiveRunCode,
    onRunStateChange,
    runActive,
    session?.sessionCode,
  ]);
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
    attachments: ChatMessageAttachment[] = [],
  ): SendChatMessageInput | null {
    if (createSendInputOverride) {
      return createSendInputOverride(activeSession, content, attachments);
    }
    const input: SendChatMessageInput = {
      agentCode: normalizeOptionalValue(config.agentCode),
      appCode: normalizeOptionalValue(activeSession.appCode ?? config.appCode),
      clientCapabilities: { userInputRequests: true },
      content,
      taskCode: normalizeOptionalValue(activeSession.taskCode ?? config.taskCode),
      workspaceCode: activeSession.workspaceCode,
    };
    if (attachments.length) input.attachments = attachments;
    return input;
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
      setPendingAttachments([]);
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
        setUserInputRequests([]);
        replaceRemoteActiveRun(null);
        return;
      }
      setLoading(true);
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents, nextActiveRun, nextUserInputRequests] = await Promise.all([
          activeAdapter.loadMessages(session.sessionCode),
          aguiEnabled ? activeAdapter.loadAguiEvents(session.sessionCode) : Promise.resolve([]),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(session.sessionCode)
            : Promise.resolve([]),
          loadActiveRunSnapshot(session.sessionCode, abortController.signal)
            .catch(() => undefined),
          activeAdapter.capabilities.supportsUserInputRequests && activeAdapter.loadUserInputRequests
            ? activeAdapter.loadUserInputRequests(session.sessionCode, abortController.signal)
            : Promise.resolve([]),
        ]);
        if (!active) return;
        setMessages(nextMessages);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
        setUserInputRequests(nextUserInputRequests);
        setUserInputRequest(nextUserInputRequests.find((request) => request.status === "Pending") ?? null);
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
      || Boolean(userInputRequest)
      || (protocolReplayEnabled && openAguiProtocolRunCodes(protocolEvents, session.sessionCode).size > 0)
    ),
  );

  useEffect(() => {
    if (!session?.sessionCode || !shouldPollData) return;
    const intervalId = window.setInterval(async () => {
      if (!session?.sessionCode) return;
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents, nextActiveRun, nextUserInputRequests] = await Promise.all([
          activeAdapter.loadMessages(session.sessionCode),
          aguiEnabled ? activeAdapter.loadAguiEvents(session.sessionCode) : Promise.resolve([]),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(session.sessionCode)
            : Promise.resolve([]),
          loadActiveRunSnapshot(session.sessionCode).catch(() => undefined),
          activeAdapter.capabilities.supportsUserInputRequests && activeAdapter.loadUserInputRequests
            ? activeAdapter.loadUserInputRequests(session.sessionCode)
            : Promise.resolve([]),
        ]);
        if (activeSessionCodeRef.current !== session.sessionCode) return;
        setMessages(nextMessages);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
        setUserInputRequests(nextUserInputRequests);
        setUserInputRequest(nextUserInputRequests.find((request) => request.status === "Pending") ?? null);
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
  }, [activeAdapter, aguiEnabled, messages, protocolEvents, protocolReplayEnabled, session?.sessionCode, shouldPollData, userInputRequest]);

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
    if (loading || config.disabledInput || userInputRequest || userInputAnswerPending) return true;
    if (!draft.trim() && !pendingAttachments.length) return true;
    return !session && !config.workspaceCode;
  }, [config.disabledInput, config.workspaceCode, draft, loading, pendingAttachments.length, session, userInputAnswerPending, userInputRequest]);
  const queueDisabled = useMemo(
    () => loading || Boolean(config.disabledInput) || Boolean(userInputRequest) || userInputAnswerPending || (!draft.trim() && !pendingAttachments.length) || !session,
    [config.disabledInput, draft, loading, pendingAttachments.length, session, userInputAnswerPending, userInputRequest],
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

  async function handleAttachmentUpload(file: File) {
    if (!file || !activeAdapter.uploadAttachment || attachmentUploadPending) return;
    const activeSession = await ensureSession();
    if (!activeSession) return;
    setAttachmentUploadPending(true);
    try {
      const attachment = await activeAdapter.uploadAttachment(activeSession, {
        file,
        kind: file.type.startsWith("image/") ? "image" : "file",
        summary: file.name,
      });
      if (activeSessionCodeRef.current === activeSession.sessionCode) {
        setPendingAttachments((current) => [...current, attachment]);
      }
    } catch (error) {
      onError(
        error instanceof Error ? error.message : "Chat attachment upload failed",
        "Attachment upload failed",
      );
    } finally {
      setAttachmentUploadPending(false);
    }
  }

  async function handleAttachmentFileInputChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.currentTarget.files?.item(0);
    event.currentTarget.value = "";
    if (!file) return;
    await handleAttachmentUpload(file);
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
    if (!content && !(sendInput.attachments?.length)) {
      restoreSourceSubmission();
      return false;
    }
    const submittedAttachments = sendInput.attachments ?? [];
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
        attachments: submittedAttachments,
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
      setUserInputRequest(result.userInputRequest?.status === "Pending" ? result.userInputRequest : null);
      const pendingUserInputRequest = result.userInputRequest;
      if (pendingUserInputRequest?.status === "Pending") {
        setUserInputRequests((current) => [
          ...current.filter((request) => request.requestCode !== pendingUserInputRequest.requestCode),
          pendingUserInputRequest,
        ]);
      }
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
        if (!sourceSubmission && submittedAttachments.length) {
          setPendingAttachments((current) => [...submittedAttachments, ...current]);
        }
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

  async function handleAnswerUserInput(answers: ChatUserInputAnswer[]) {
    if (!session || !userInputRequest || !activeAdapter.answerUserInputRequestStream) return;
    const continuationAnchorMessageCode = [...messages]
      .reverse()
      .find((message) => message.role === "ASSISTANT")?.messageCode ?? "";
    setUserInputAnswerPending(true);
    setSendingState(true);
    setLiveAguiPending(aguiEnabled);
    setLiveAguiPendingAnchorMessageCode(continuationAnchorMessageCode);
    setLiveAguiEvents([]);
    setLiveAguiAnchorMessageCodes({});
    try {
      const result = await activeAdapter.answerUserInputRequestStream(
        session,
        userInputRequest.requestCode,
        { answers },
        {
          onAguiEvent: (event) => {
            if (!aguiEnabled) return;
            setLiveAguiEvents((current) => [...current, event]);
            const runCode = String(event.runId ?? event.runCode ?? "");
            if (runCode && continuationAnchorMessageCode) {
              setLiveAguiAnchorMessageCodes((current) => current[runCode]
                ? current
                : { ...current, [runCode]: continuationAnchorMessageCode });
            }
          },
        },
      );
      setMessages(await activeAdapter.loadMessages(session.sessionCode).catch(() => result.messages));
      setSession(result.session);
      const nextUserInputRequests = activeAdapter.loadUserInputRequests
        ? await activeAdapter.loadUserInputRequests(session.sessionCode).catch(() => null)
        : null;
      if (nextUserInputRequests) {
        setUserInputRequests(nextUserInputRequests);
        setUserInputRequest(nextUserInputRequests.find((request) => request.status === "Pending") ?? null);
      } else {
        setUserInputRequest(result.userInputRequest?.status === "Pending" ? result.userInputRequest : null);
        setUserInputRequests((current) => current.map((request) => (
          request.requestCode === userInputRequest.requestCode
            ? { ...request, answers, status: "Answered" as const }
            : request
        )));
      }
      if (aguiEnabled) {
        const [nextAguiEvents, nextProtocolEvents] = await Promise.all([
          activeAdapter.loadAguiEvents(session.sessionCode).catch(() => aguiEvents),
          protocolReplayEnabled && activeAdapter.loadProtocolEvents
            ? activeAdapter.loadProtocolEvents(session.sessionCode).catch(() => protocolEvents)
            : Promise.resolve([]),
        ]);
        setAguiEvents(nextAguiEvents);
        setProtocolEvents(nextProtocolEvents);
      }
      setLiveAguiEvents([]);
      setLiveAguiPending(false);
      setLiveAguiPendingAnchorMessageCode("");
      setLiveAguiAnchorMessageCodes({});
      await loadActiveRunSnapshot(session.sessionCode).then(replaceRemoteActiveRun).catch(() => undefined);
    } catch (error) {
      onError(error instanceof Error ? error.message : "Chat answer failed", "Chat answer failed");
    } finally {
        setSendingState(false);
        setUserInputAnswerPending(false);
        setLiveAguiPending(false);
        setLiveAguiPendingAnchorMessageCode("");
    }
  }

  async function handleSend() {
    if (sendingRef.current || sendDisabled || userInputRequest || userInputAnswerPending) return;
    const activeSession = await ensureSession();
    if (!activeSession) return;
    const content = draft.trim();
    if (!content && !pendingAttachments.length) return;
    const sendInput = createSendInput(activeSession, content, pendingAttachments);
    if (!sendInput) return;
    setDraft("");
    setPendingAttachments([]);
    if (remoteActiveRunRef.current) {
      enqueueSubmission(activeSession, sendInput);
      return;
    }
    await runMessage(activeSession, sendInput);
  }

  async function handleQueue() {
    if (queueDisabled || userInputRequest || userInputAnswerPending) return;
    const activeSession = await ensureSession();
    if (!activeSession) return;
    const content = draft.trim();
    if (!content && !pendingAttachments.length) return;
    const sendInput = createSendInput(activeSession, content, pendingAttachments);
    if (!sendInput) return;
    setDraft("");
    setPendingAttachments([]);
    if (sendingRef.current || remoteActiveRunRef.current) {
      enqueueSubmission(activeSession, sendInput);
      return;
    }
    await runMessage(activeSession, sendInput);
  }

  async function handleSteer() {
    if (steeringRef.current || steerDisabled || userInputRequest || userInputAnswerPending) return;
    const activeSession = session;
    if (!activeSession) return;
    const content = draft.trim();
    if (!content && !pendingAttachments.length) return;
    const sendInput = createSendInput(activeSession, content, pendingAttachments);
    if (!sendInput) return;
    const expectedRunCode = activeRunCodeRef.current
      || remoteActiveRunRef.current?.runCode
      || "";
    const queueGeneration = queueGenerationRef.current;
    const autoDrainEpoch = autoDrainEpochRef.current;
    setDraft("");
    setPendingAttachments([]);
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
    const submission = queuedSubmissionsRef.current.find((item) => item.id === id);
    replaceQueuedSubmissions(
      removeQueuedChatSubmission(queuedSubmissionsRef.current, id),
    );
    setDraft(content);
    setPendingAttachments(submission?.input.attachments ?? []);
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
    setUserInputRequest(null);
    setAguiEvents([]);
    setProtocolEvents([]);
    setLiveAguiEvents([]);
    setLiveAguiPendingAnchorMessageCode("");
    setLiveAguiAnchorMessageCodes({});
    setPendingAttachments([]);
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
      liveAguiPendingAnchorMessageCode,
      liveAguiPending,
      messages,
      protocolEvents,
      sessionCode: session?.sessionCode ?? "",
    }).debugEventsByRunCode,
    [debugOptions, liveAguiAnchorMessageCodes, liveAguiEvents, liveAguiPendingAnchorMessageCode, liveAguiPending, messages, protocolEvents, session?.sessionCode],
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

  const attachmentDownloader = useMemo<
    ((attachment: ChatMessageAttachment) => Promise<Blob>) | undefined
  >(() => {
    if (!session || !activeAdapter.downloadAttachment) return undefined;
    return (attachment: ChatMessageAttachment) =>
      activeAdapter.downloadAttachment!(session, attachment);
  }, [activeAdapter, session]);
  const pendingAttachmentHeader = pendingAttachments.length ? (
    <div className="tron-chat__runtime-attachments tron-chat__runtime-attachments--pending">
      {pendingAttachments.map((attachment) => (
        <PendingAttachmentCard
          attachment={attachment}
          downloadAttachment={attachmentDownloader}
          key={attachment.id}
          onRemove={(attachmentId) => setPendingAttachments((current) => (
            current.filter((item) => item.id !== attachmentId)
          ))}
        />
      ))}
    </div>
  ) : null;
  const runtimeFooterAddon = (
    <div className="tron-chat__runtime-footer-addon">
      {showAttachmentControl && activeAdapter.uploadAttachment ? (
        <div className="tron-chat__runtime-attachments">
          <input
            className="tron-chat__runtime-attachment-input"
            ref={attachmentFileInputRef}
            type="file"
            onChange={(event) => { void handleAttachmentFileInputChange(event); }}
          />
          <button
            aria-label="Attach file"
            className="tron-chat__icon-button"
            disabled={Boolean(config.disabledInput) || attachmentUploadPending}
            title={attachmentUploadPending ? "Uploading attachment" : "Attach file"}
            type="button"
            onClick={() => attachmentFileInputRef.current?.click()}
          >
            <Paperclip size={16} />
          </button>
        </div>
      ) : null}
      {footerAddon}
    </div>
  );

  const panel = (
    <TronChatPanel
      attachmentLabels={attachmentLabels}
      attachmentDownloader={attachmentDownloader}
      autoFocusComposer={autoFocusComposer}
      composerRef={composerRef}
      composerHeader={composerHeader ?? pendingAttachmentHeader}
      createSessionLabel={createSessionLabel}
      currentUserDisplayName={currentUserDisplayName}
      debugOptions={resolvedDebugOptions}
      disabledInput={Boolean(config.disabledInput) || userInputRequest?.status === "Pending"}
      draft={draft}
      emptyDescription={emptyDescription ?? (config.workspaceCode
        ? "Open a session and start talking to your configured agent."
        : "Set a workspace code before starting a session.")}
      emptyTitle={emptyTitle}
      footerAddon={runtimeFooterAddon}
      formatAnnotationCount={formatAnnotationCount}
      headerContent={headerContent}
      inputPlaceholder={config.inputPlaceholder}
      liveAguiAnchorMessageCodes={liveAguiAnchorMessageCodes}
      liveAguiEvents={liveAguiEvents}
      liveAguiPendingAnchorMessageCode={liveAguiPendingAnchorMessageCode}
      liveAguiPending={liveAguiPending}
      messages={messages}
      onCreateSession={onCreateSession ?? (() => { void ensureSession(); })}
      onNewSession={handleNewSession}
      onDraftChange={setDraft}
      onPasteAttachment={handleAttachmentUpload}
      onEditMessage={setDraft}
      onEditQueuedMessage={handleEditQueuedMessage}
      onRemoveQueuedMessage={handleRemoveQueuedMessage}
      onSend={(mode) => { void handleSubmit(mode); }}
      onStop={() => { void handleStop(); }}
      protocolEvents={protocolEvents}
      queueDisabled={queueDisabled || Boolean(userInputRequest)}
      queuedMessages={visibleQueuedSubmissions}
      runningSubmitModes={runningSubmitModes}
      sendDisabled={sendDisabled || Boolean(userInputRequest)}
      sending={runActive}
      session={session}
      showAgui={aguiEnabled}
      showHeader={showHeader ?? !config.floating}
      showStop={canStop}
      sidebarContent={sidebarContent}
      steerDisabled={steerDisabled || Boolean(userInputRequest)}
      title={config.title || floatingTitle}
      userInputLabels={config.userInputLabels}
      userInputRequests={userInputRequests}
      workRailLabels={config.workRailLabels}
      userInputAnswerPending={userInputAnswerPending}
      userInputRequest={userInputRequest}
      onAnswerUserInput={(answers) => { void handleAnswerUserInput(answers); }}
      renderMarkdownLink={renderMarkdownLink}
    />
  );

  if (config.floating !== false) {
    return (
      <TronChatWindow
        attachmentLabels={attachmentLabels}
        attachmentDownloader={attachmentDownloader}
        autoFocusComposer={autoFocusComposer}
        composerRef={composerRef}
        composerHeader={composerHeader ?? pendingAttachmentHeader}
        currentUserDisplayName={currentUserDisplayName}
        debugOptions={resolvedDebugOptions}
        disabledInput={Boolean(config.disabledInput) || userInputRequest?.status === "Pending"}
        draft={draft}
        emptyDescription={emptyDescription ?? (config.workspaceCode
          ? "Open a session and start talking to your configured agent."
          : "Set a workspace code before starting a session.")}
        emptyTitle={emptyTitle}
        footerAddon={runtimeFooterAddon}
        formatAnnotationCount={formatAnnotationCount}
        inputPlaceholder={config.inputPlaceholder}
      liveAguiAnchorMessageCodes={liveAguiAnchorMessageCodes}
      liveAguiEvents={liveAguiEvents}
      liveAguiPendingAnchorMessageCode={liveAguiPendingAnchorMessageCode}
      liveAguiPending={liveAguiPending}
        messages={messages}
        onCreateSession={onCreateSession ?? (() => { void ensureSession(); })}
        onNewSession={handleNewSession}
        onDraftChange={setDraft}
        onPasteAttachment={handleAttachmentUpload}
        onEditMessage={setDraft}
        onEditQueuedMessage={handleEditQueuedMessage}
        onRemoveQueuedMessage={handleRemoveQueuedMessage}
        onSend={(mode) => { void handleSubmit(mode); }}
        onStop={() => { void handleStop(); }}
        protocolEvents={protocolEvents}
        queueDisabled={queueDisabled || Boolean(userInputRequest)}
        queuedMessages={visibleQueuedSubmissions}
        runningSubmitModes={runningSubmitModes}
        sendDisabled={sendDisabled || Boolean(userInputRequest)}
        sending={runActive}
        session={session}
        showAgui={aguiEnabled}
        showStop={canStop}
        sidebarContent={sidebarContent}
        steerDisabled={steerDisabled || Boolean(userInputRequest)}
        state={windowState}
        storageKey={config.storageKey || "tron.chat.runtime.window"}
        title={floatingTitle}
        titleSubtitle={floatingSubtitle}
        userInputLabels={config.userInputLabels}
        userInputRequests={userInputRequests}
        workRailLabels={config.workRailLabels}
        userInputAnswerPending={userInputAnswerPending}
        userInputRequest={userInputRequest}
        onAnswerUserInput={(answers) => { void handleAnswerUserInput(answers); }}
        onStateChange={onWindowStateChange}
        renderMarkdownLink={renderMarkdownLink}
      />
    );
  }

  return panel;
}

import {
  buildTranscriptPresentation,
  createMockChatAdapter,
  mergeTranscriptAguiEvents,
  normalizeChatSession,
  type ChatAdapter,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatClientConfig,
  type ChatDebugOptions,
  type ChatMessage,
  type ChatSession,
  type SendChatMessageInput,
  type ChatFloatingState,
  isRecentPendingUserTurn,
  openAguiProtocolRunCodes,
} from "@tron/chat-core";
import {
  computed,
  defineComponent,
  h,
  ref,
  type PropType,
  useSlots,
  watch,
} from "vue";
import { defaultErrorReporter, dedupeMessages, normalizeOptionalValue } from "./utils";
import { TronChatPanel } from "./TronChatPanel";
import { TronChatWindow } from "./TronChatWindow";

export type ToastLikeCallback = (message: string, title?: string) => void;

export interface TronChatRuntimeWindowProps {
  adapter?: ChatAdapter;
  config: ChatClientConfig;
  currentUserDisplayName?: string;
  debugOptions?: ChatDebugOptions;
  floatingSubtitle?: string;
  floatingTitle?: string;
  initialSession?: ChatSession | null;
  onError?: ToastLikeCallback;
  onInfo?: ToastLikeCallback;
  onSessionChange?: (session: ChatSession | null) => void;
  onWindowStateChange?: (state: ChatFloatingState) => void;
  windowState?: ChatFloatingState;
}

export const TronChatRuntimeWindow = defineComponent({
  name: "TronChatRuntimeWindow",
  props: {
    adapter: { default: undefined, type: Object as PropType<ChatAdapter | undefined> },
    config: { required: true, type: Object as PropType<ChatClientConfig> },
    currentUserDisplayName: { default: "", type: String },
    debugOptions: { default: undefined, type: Object as PropType<ChatDebugOptions | undefined> },
    floatingSubtitle: { default: "Workspace assistant", type: String },
    floatingTitle: { default: "Main Agent", type: String },
    initialSession: { default: null, type: Object as PropType<ChatSession | null> },
    onError: { default: defaultErrorReporter, type: Function as PropType<ToastLikeCallback> },
    onInfo: { default: undefined, type: Function as PropType<ToastLikeCallback | undefined> },
    onSessionChange: { default: undefined, type: Function as PropType<((session: ChatSession | null) => void) | undefined> },
    onWindowStateChange: { default: undefined, type: Function as PropType<((state: ChatFloatingState) => void) | undefined> },
    windowState: { default: undefined, type: Object as PropType<ChatFloatingState | undefined> },
  },
  emits: {
    error: (_message: string, _title?: string) => true,
    info: (_message: string, _title?: string) => true,
    "session-change": (_session: ChatSession | null) => true,
    "window-state-change": (_state: ChatFloatingState) => true,
  },
  setup(props, { emit }) {
    const slots = useSlots();
    const activeAdapter = computed(() => props.adapter ?? createMockChatAdapter());
    const normalizedInitialSession = computed(() => (props.initialSession ? normalizeChatSession(props.initialSession) : null));
    const session = ref<ChatSession | null>(normalizedInitialSession.value);
    const messages = ref<ChatMessage[]>([]);
    const aguiEvents = ref<ChatAguiEvent[]>([]);
    const protocolEvents = ref<ChatAguiStreamEvent[]>([]);
    const liveAguiEvents = ref<ChatAguiStreamEvent[]>([]);
    const liveAguiAnchorMessageCodes = ref<Record<string, string>>({});
    const draft = ref("");
    const sending = ref(false);
    const loading = ref(false);
    const liveAguiPending = ref(false);
    const abortRef = ref<AbortController | null>(null);
    const streamAbortRequestedRef = ref(false);

    const aguiEnabled = computed(() => props.config.showAgui !== false && activeAdapter.value.capabilities.supportsAgui);
    const protocolReplayEnabled = computed(() => (
      aguiEnabled.value
      && activeAdapter.value.capabilities.supportsProtocolReplay
      && Boolean(activeAdapter.value.loadProtocolEvents)
    ));

    watch(normalizedInitialSession, (nextSession) => {
      session.value = nextSession;
    }, { immediate: true });

    watch(session, (nextSession) => {
      props.onSessionChange?.(nextSession);
      emit("session-change", nextSession);
    }, { immediate: true });

    watch(session, async (nextSession, _, onCleanup) => {
      let active = true;
      onCleanup(() => {
        active = false;
      });

      if (!nextSession?.sessionCode) {
        messages.value = [];
        aguiEvents.value = [];
        protocolEvents.value = [];
        return;
      }

      loading.value = true;
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents] = await Promise.all([
          activeAdapter.value.loadMessages(nextSession.sessionCode),
          aguiEnabled.value ? activeAdapter.value.loadAguiEvents(nextSession.sessionCode) : Promise.resolve([]),
          protocolReplayEnabled.value && activeAdapter.value.loadProtocolEvents
            ? activeAdapter.value.loadProtocolEvents(nextSession.sessionCode)
            : Promise.resolve([]),
        ]);
        if (!active) return;
        messages.value = nextMessages;
        aguiEvents.value = nextAguiEvents;
        protocolEvents.value = nextProtocolEvents;
      } catch (error) {
        if (!active) return;
        const message = error instanceof Error ? error.message : "Chat data could not be loaded";
        props.onError?.(message, "Chat load failed");
        emit("error", message, "Chat load failed");
      } finally {
        if (active) loading.value = false;
      }
    }, { immediate: true });

    const shouldPollData = computed(() => Boolean(
      session.value?.sessionCode
      && !sending.value
      && (
        isRecentPendingUserTurn(messages.value)
        || (protocolReplayEnabled.value && openAguiProtocolRunCodes(protocolEvents.value, session.value.sessionCode).size > 0)
      ),
    ));

    watch([session, sending, shouldPollData], (_, __, onCleanup) => {
      if (!session.value?.sessionCode || (!sending.value && !shouldPollData.value)) return;
      const intervalId = window.setInterval(async () => {
        if (!session.value?.sessionCode) return;
        try {
          const [nextMessages, nextAguiEvents, nextProtocolEvents] = await Promise.all([
            activeAdapter.value.loadMessages(session.value.sessionCode),
            aguiEnabled.value ? activeAdapter.value.loadAguiEvents(session.value.sessionCode) : Promise.resolve([]),
            protocolReplayEnabled.value && activeAdapter.value.loadProtocolEvents
              ? activeAdapter.value.loadProtocolEvents(session.value.sessionCode)
              : Promise.resolve([]),
          ]);
          messages.value = nextMessages;
          aguiEvents.value = nextAguiEvents;
          protocolEvents.value = nextProtocolEvents;
        } catch {
          // Keep the last successful snapshot.
        }
      }, 1000);

      onCleanup(() => {
        window.clearInterval(intervalId);
      });
    }, { immediate: true });

    const sendDisabled = computed(() => {
      if (loading.value || sending.value || props.config.disabledInput) return true;
      if (!draft.value.trim()) return true;
      return !session.value && !props.config.workspaceCode;
    });

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
        reportError("Workspace code is required before starting a session.", "Session create failed");
        return null;
      }

      const created = await activeAdapter.value.createSession({
        agentCode: normalizeOptionalValue(props.config.agentCode),
        appCode: normalizeOptionalValue(props.config.appCode),
        taskCode: normalizeOptionalValue(props.config.taskCode),
        title: normalizeOptionalValue(props.config.title) ?? props.floatingTitle,
        workspaceCode: props.config.workspaceCode,
      });
      session.value = created;
      messages.value = [];
      aguiEvents.value = [];
      protocolEvents.value = [];
      return created;
    }

    async function handleSend() {
      if (sendDisabled.value) return;
      const activeSession = await ensureSession();
      if (!activeSession) return;
      const content = draft.value.trim();
      if (!content) return;

      const sendInput: SendChatMessageInput = {
        agentCode: normalizeOptionalValue(props.config.agentCode),
        appCode: normalizeOptionalValue(activeSession.appCode ?? props.config.appCode),
        content,
        taskCode: normalizeOptionalValue(activeSession.taskCode ?? props.config.taskCode),
        workspaceCode: activeSession.workspaceCode,
      };
      const abortController = new AbortController();
      abortRef.value = abortController;
      streamAbortRequestedRef.value = false;

      const optimisticCreatedAt = new Date().toISOString();
      const optimisticUserCode = `LOCAL-${Date.now()}-USER`;
      const optimisticAssistantCode = `LOCAL-${Date.now()}-ASSISTANT`;
      sending.value = true;
      liveAguiPending.value = aguiEnabled.value;
      draft.value = "";
      messages.value = [
        ...messages.value,
        {
          actor: props.currentUserDisplayName || "User",
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
          actor: activeSession.agentCode ?? props.config.agentCode ?? "Assistant",
          channel: activeSession.taskCode ? "TASK_AGENT" : "MAIN_AGENT",
          content: "",
          createdAt: optimisticCreatedAt,
          id: -Date.now() - 1,
          messageCode: optimisticAssistantCode,
          role: "ASSISTANT",
          runCode: null,
          sessionCode: activeSession.sessionCode,
        },
      ];

      try {
        const result = await activeAdapter.value.streamMessage(
          activeSession,
          sendInput,
          {
            onAguiEvent: (event) => {
              if (!aguiEnabled.value) return;
              liveAguiEvents.value = [...liveAguiEvents.value, event];
              const runCode = event.runId ?? "";
              if (!runCode) return;
              liveAguiAnchorMessageCodes.value = liveAguiAnchorMessageCodes.value[runCode]
                ? liveAguiAnchorMessageCodes.value
                : { ...liveAguiAnchorMessageCodes.value, [runCode]: optimisticUserCode };
            },
            onDelta: (delta) => {
              liveAguiPending.value = false;
              messages.value = messages.value.map((message) => (
                message.messageCode === optimisticAssistantCode
                  ? { ...message, content: `${message.content}${delta}` }
                  : message
              ));
            },
          },
          abortController.signal,
        );
        session.value = result.session;
        messages.value = dedupeMessages([
          ...messages.value.filter((message) => (
            message.messageCode !== optimisticUserCode && message.messageCode !== optimisticAssistantCode
          )),
          ...result.messages,
        ]);
        if (aguiEnabled.value) {
          const [nextAguiEvents, nextProtocolEvents] = await Promise.all([
            activeAdapter.value.loadAguiEvents(result.session.sessionCode),
            protocolReplayEnabled.value && activeAdapter.value.loadProtocolEvents
              ? activeAdapter.value.loadProtocolEvents(result.session.sessionCode)
              : Promise.resolve([]),
          ]);
          aguiEvents.value = nextAguiEvents;
          protocolEvents.value = nextProtocolEvents;
        }
        liveAguiEvents.value = [];
        liveAguiAnchorMessageCodes.value = {};
      } catch (error) {
        const stopped = streamAbortRequestedRef.value || (error instanceof Error && error.name === "AbortError");
        messages.value = messages.value.filter((message) => (
          message.messageCode !== optimisticAssistantCode || Boolean(message.content.trim())
        ));
        if (stopped) {
          reportInfo("Current response has been stopped", "Chat stopped");
        } else {
          const message = error instanceof Error ? error.message : "Chat send failed";
          reportError(message, "Chat send failed");
        }
      } finally {
        sending.value = false;
        liveAguiPending.value = false;
        abortRef.value = null;
        streamAbortRequestedRef.value = false;
      }
    }

    function handleStop() {
      streamAbortRequestedRef.value = true;
      abortRef.value?.abort();
    }

    const presentationDebugEvents = computed(() => buildTranscriptPresentation({
      debugOptions: props.debugOptions,
      liveAguiAnchorMessageCodes: liveAguiAnchorMessageCodes.value,
      liveAguiEvents: liveAguiEvents.value,
      liveAguiPending: liveAguiPending.value,
      messages: messages.value,
      protocolEvents: protocolEvents.value,
      sessionCode: session.value?.sessionCode ?? "",
    }).debugEventsByRunCode);

    const resolvedDebugOptions = computed(() => ({
      ...props.debugOptions,
      debugEventTypes: props.debugOptions?.debugEventTypes?.length
        ? Array.from(new Set([
          ...props.debugOptions.debugEventTypes,
          ...Array.from(presentationDebugEvents.value.keys()).flatMap((runCode) => (
            mergeTranscriptAguiEvents(protocolEvents.value, liveAguiEvents.value)
              .filter((event) => (event.runId ?? event.runCode ?? "") === runCode)
              .map((event) => String(event.type).toUpperCase())
          )),
        ]))
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
      disabledInput: Boolean(props.config.disabledInput),
      draft: draft.value,
      emptyDescription: props.config.workspaceCode
        ? "Open a session and start talking to your configured agent."
        : "Set a workspace code before starting a session.",
      inputPlaceholder: props.config.inputPlaceholder,
      liveAguiAnchorMessageCodes: liveAguiAnchorMessageCodes.value,
      liveAguiEvents: liveAguiEvents.value,
      liveAguiPending: liveAguiPending.value,
      messages: messages.value,
      onCreateSession: () => { void ensureSession(); },
      onDraftChange: handleDraftChange,
      onEditMessage: handleEditMessage,
      onSend: () => { void handleSend(); },
      onStop: handleStop,
      protocolEvents: protocolEvents.value,
      sendDisabled: sendDisabled.value,
      sending: sending.value,
      session: session.value,
      showAgui: aguiEnabled.value,
      stopLabel: "Stop",
      title: props.config.title || props.floatingTitle,
    }));

    return () => {
      if (props.config.floating !== false) {
        return h(
          TronChatWindow,
          {
            ...panelCommonProps.value,
            onStateChange: (state: ChatFloatingState) => {
              props.onWindowStateChange?.(state);
              emit("window-state-change", state);
            },
            state: props.windowState,
            storageKey: props.config.storageKey || "tron.chat.runtime.window",
            title: props.floatingTitle,
            titleSubtitle: props.floatingSubtitle,
          },
          slots,
        );
      }

      return h(
        TronChatPanel,
        {
          ...panelCommonProps.value,
          showHeader: true,
        },
        slots,
      );
    };
  },
});

export default TronChatRuntimeWindow;

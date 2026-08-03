<script setup lang="ts">
import {
  buildTranscriptPresentation,
  createMockChatAdapter,
  mergeTranscriptAguiEvents,
  normalizeChatSession,
  type ChatAguiEvent,
  type ChatAguiStreamEvent,
  type ChatMessage,
  type ChatSession,
  type SendChatMessageInput,
  type ChatFloatingState,
  isRecentPendingUserTurn,
  openAguiProtocolRunCodes,
} from "@tron/chat-core";
import { computed, ref, useSlots, watch } from "vue";
import {
  defaultErrorReporter,
  dedupeMessages,
  normalizeOptionalValue,
} from "../utils";
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

watch(
  normalizedInitialSession,
  (nextSession) => {
    session.value = nextSession;
  },
  { immediate: true },
);

watch(
  session,
  (nextSession) => {
    props.onSessionChange?.(nextSession);
    emit("session-change", nextSession);
  },
  { immediate: true },
);

watch(
  session,
  async (nextSession, _, onCleanup) => {
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
      const [nextMessages, nextAguiEvents, nextProtocolEvents] =
        await Promise.all([
          activeAdapter.value.loadMessages(nextSession.sessionCode),
          aguiEnabled.value
            ? activeAdapter.value.loadAguiEvents(nextSession.sessionCode)
            : Promise.resolve([]),
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
      const message =
        error instanceof Error
          ? error.message
          : "Chat data could not be loaded";
      props.onError?.(message, "Chat load failed");
      emit("error", message, "Chat load failed");
    } finally {
      if (active) loading.value = false;
    }
  },
  { immediate: true },
);

const shouldPollData = computed(() =>
  Boolean(
    session.value?.sessionCode &&
    !sending.value &&
    (isRecentPendingUserTurn(messages.value) ||
      (protocolReplayEnabled.value &&
        openAguiProtocolRunCodes(
          protocolEvents.value,
          session.value.sessionCode,
        ).size > 0)),
  ),
);

watch(
  [session, sending, shouldPollData],
  (_, __, onCleanup) => {
    if (
      !session.value?.sessionCode ||
      (!sending.value && !shouldPollData.value)
    )
      return;
    const intervalId = window.setInterval(async () => {
      if (!session.value?.sessionCode) return;
      try {
        const [nextMessages, nextAguiEvents, nextProtocolEvents] =
          await Promise.all([
            activeAdapter.value.loadMessages(session.value.sessionCode),
            aguiEnabled.value
              ? activeAdapter.value.loadAguiEvents(session.value.sessionCode)
              : Promise.resolve([]),
            protocolReplayEnabled.value &&
            activeAdapter.value.loadProtocolEvents
              ? activeAdapter.value.loadProtocolEvents(
                  session.value.sessionCode,
                )
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
  },
  { immediate: true },
);

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
    appCode: normalizeOptionalValue(
      activeSession.appCode ?? props.config.appCode,
    ),
    content,
    taskCode: normalizeOptionalValue(
      activeSession.taskCode ?? props.config.taskCode,
    ),
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
    session.value = result.session;
    messages.value = dedupeMessages([
      ...messages.value.filter(
        (message) =>
          message.messageCode !== optimisticUserCode &&
          message.messageCode !== optimisticAssistantCode,
      ),
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
    const stopped =
      streamAbortRequestedRef.value ||
      (error instanceof Error && error.name === "AbortError");
    messages.value = messages.value.filter(
      (message) =>
        message.messageCode !== optimisticAssistantCode ||
        Boolean(message.content.trim()),
    );
    if (stopped) {
      reportInfo("Current response has been stopped", "Chat stopped");
    } else {
      const message =
        error instanceof Error ? error.message : "Chat send failed";
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

function handleNewSession() {
  handleStop();
  session.value = null;
  messages.value = [];
  aguiEvents.value = [];
  protocolEvents.value = [];
  liveAguiEvents.value = [];
  liveAguiAnchorMessageCodes.value = {};
}

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
  onCreateSession: () => {
    void ensureSession();
  },
  onNewSession: handleNewSession,
  onDraftChange: handleDraftChange,
  onEditMessage: handleEditMessage,
  onSend: () => {
    void handleSend();
  },
  onStop: handleStop,
  protocolEvents: protocolEvents.value,
  sendDisabled: sendDisabled.value,
  sending: sending.value,
  session: session.value,
  showAgui: aguiEnabled.value,
  stopLabel: "Stop",
  title: props.config.title || props.floatingTitle,
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

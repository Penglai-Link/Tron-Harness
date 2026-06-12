<script setup lang="ts">
import { ArrowDown, Send, Square, Bot } from "lucide-vue-next";
import {
  buildTranscriptPresentation,
  describeDebugEvent,
  sessionAgentTagLabel,
  type ChatMessage,
} from "@tron/chat-core";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { cx, insertNewlineAtCursor } from "../utils";
import ChatBubble from "./ChatBubble.vue";
import WaitingRail from "./WaitingRail.vue";
import AguiRunWorkRail from "./AguiRunWorkRail.vue";
import { TronChatPanelProps } from "../types";

const props = withDefaults(defineProps<TronChatPanelProps>(), {
  createSessionLabel: "Create session",
  currentUserDisplayName: "",
  debugOptions: undefined,
  disabledInput: false,
  emptyDescription: "Create a session to start chatting.",
  emptyTitle: "No session selected",
  inputPlaceholder: "Enter to send, Shift+Enter for newline",
  liveAguiAnchorMessageCodes: () => ({}),
  liveAguiEvents: () => [],
  liveAguiPending: false,
  onCreateSession: undefined,
  onEditMessage: undefined,
  onStop: undefined,
  protocolEvents: () => [],
  sendDisabled: false,
  sendLabel: "Send",
  sending: false,
  session: null,
  showAgui: true,
  showHeader: true,
  stopLabel: "Stop",
  title: "",
});

const emit = defineEmits<{
  "create-session": [];
  "draft-change": [value: string];
  "edit-message": [value: string];
  send: [];
  stop: [];
}>();

const transcriptRef = ref<HTMLElement | null>(null);
const showJumpButton = ref(false);
const autoStickToBottom = ref(true);

const presentation = computed(() =>
  buildTranscriptPresentation({
    debugOptions: props.debugOptions,
    liveAguiAnchorMessageCodes: props.liveAguiAnchorMessageCodes,
    liveAguiEvents: props.liveAguiEvents,
    liveAguiPending: props.liveAguiPending,
    messages: props.messages,
    protocolEvents: props.protocolEvents,
    sessionCode: props.session?.sessionCode ?? "",
  }),
);

function scrollToBottom(behavior: ScrollBehavior = "auto") {
  const element = transcriptRef.value;
  if (!element) return;
  element.scrollTo({ behavior, top: element.scrollHeight });
  autoStickToBottom.value = true;
  showJumpButton.value = false;
}

function handleScroll() {
  const element = transcriptRef.value;
  if (!element) return;
  const distanceFromBottom =
    element.scrollHeight - element.scrollTop - element.clientHeight;
  autoStickToBottom.value = distanceFromBottom <= 96;
  showJumpButton.value = distanceFromBottom > 160;
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key !== "Enter" || event.isComposing) return;
  event.preventDefault();
  if (event.shiftKey) {
    emit(
      "draft-change",
      insertNewlineAtCursor(event.currentTarget as HTMLTextAreaElement),
    );
    return;
  }
  if (props.sending) {
    emit("stop");
    return;
  }
  emit("send");
}

function workRailsForMessage(message: ChatMessage) {
  return (
    presentation.value.workRailsByMessageCode.get(message.messageCode) ?? []
  );
}

function debugEventsForRun(runCode: string) {
  return (presentation.value.debugEventsByRunCode.get(runCode) ?? []).map(
    (event) => describeDebugEvent(event),
  );
}

watch([() => props.messages, presentation], async () => {
  if (autoStickToBottom.value) {
    await nextTick();
    scrollToBottom("auto");
  } else {
    handleScroll();
  }
});

watch(
  () => props.session?.sessionCode,
  () => {
    autoStickToBottom.value = true;
    showJumpButton.value = false;
  },
);

onMounted(() => {
  scrollToBottom("auto");
});
</script>

<template>
  <div class="tron-chat">
    <header v-if="showHeader" class="tron-chat__header">
      <div class="tron-chat__header-content">
        <slot name="header">
          <span class="tron-chat__title">{{
            title || session?.title || "Chat"
          }}</span>
          <span v-if="session" class="tron-chat__subtitle">
            {{ session.sessionCode }} ·
            {{ sessionAgentTagLabel(session.agentCode) }}
          </span>
        </slot>
      </div>
    </header>

    <template v-if="session">
      <div
        :class="
          cx(
            'tron-chat__content',
            $slots.sidebar && 'tron-chat__content--with-sidebar',
          )
        "
      >
        <section
          ref="transcriptRef"
          aria-label="Chat transcript"
          class="tron-chat__transcript"
          @scroll="handleScroll"
        >
          <div v-if="messages.length" class="tron-chat__transcript-inner">
            <template v-for="message in messages" :key="message.messageCode">
              <ChatBubble
                :current-user-display-name="currentUserDisplayName"
                :message="message"
                @edit-message="
                  (content: string) => emit('edit-message', content)
                "
              />
              <WaitingRail
                v-if="
                  showAgui &&
                  message.messageCode ===
                    presentation.waitingForModelAnchorMessageCode
                "
              />
              <template v-if="showAgui">
                <AguiRunWorkRail
                  v-for="run in workRailsForMessage(message)"
                  :key="run.runCode"
                  :debug-events="debugEventsForRun(run.runCode)"
                  :events="run.events"
                />
              </template>
            </template>
          </div>
          <Bot
            :size="40"
            :strokeWidth="1"
            class="tron-chat__transcript__bot"
            v-else
          />
        </section>
        <aside v-if="$slots.sidebar" class="tron-chat__sidebar">
          <slot name="sidebar" />
        </aside>
      </div>

      <footer class="tron-chat__footer">
        <button
          v-if="showJumpButton"
          aria-label="Jump to latest message"
          class="tron-chat__button tron-chat__jump"
          type="button"
          @click="scrollToBottom('smooth')"
        >
          <ArrowDown :size="16" />
        </button>
        <div class="tron-chat__composer">
          <textarea
            class="tron-chat__textarea"
            :disabled="disabledInput"
            :placeholder="inputPlaceholder"
            :value="draft"
            @input="
              (e: Event) =>
                emit('draft-change', (e.target as HTMLTextAreaElement).value)
            "
            @keydown="handleKeydown"
          />
          <div class="tron-chat__composer-row">
            <div class="tron-chat__composer-addon">
              <slot name="footerAddon" />
            </div>
            <button
              :class="
                cx(
                  'tron-chat__button tron-chat__button_send',
                  sending && 'tron-chat__button--danger',
                )
              "
              :disabled="disabledInput || (!sending && sendDisabled)"
              type="button"
              @click="sending ? emit('stop') : emit('send')"
            >
              <Square v-if="sending" :size="14" />
              <Send v-else :size="15" />
            </button>
          </div>
        </div>
      </footer>
    </template>

    <template v-else>
      <div
        :class="
          cx(
            'tron-chat__content',
            $slots.sidebar && 'tron-chat__content--with-sidebar',
          )
        "
      >
        <section class="tron-chat__transcript">
          <div class="tron-chat__empty">
            <div class="tron-chat__empty-inner">
              <div class="tron-chat__empty-title">{{ emptyTitle }}</div>
              <div class="tron-chat__empty-description">
                {{ emptyDescription }}
              </div>
              <div style="margin-top: 16px">
                <button
                  class="tron-chat__button"
                  type="button"
                  @click="emit('create-session')"
                >
                  {{ createSessionLabel }}
                </button>
              </div>
            </div>
          </div>
        </section>
        <aside v-if="$slots.sidebar" class="tron-chat__sidebar">
          <slot name="sidebar" />
        </aside>
      </div>
    </template>
  </div>
</template>

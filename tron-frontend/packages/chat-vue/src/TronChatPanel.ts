import {
  ArrowDown,
  Bot,
  BrainCircuit,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  LoaderCircle,
  Pencil,
  Send,
  Square,
  Terminal,
  UserRound,
} from "lucide-vue-next";
import {
  buildAguiUserWorkCards,
  buildTranscriptPresentation,
  describeDebugEvent,
  deriveReasoningPreview,
  formatJsonBlockValue,
  formatTime,
  sessionAgentTagLabel,
  toolSummaryLabel,
  type AguiUserWorkCard,
  type ChatAguiStreamEvent,
  type ChatDebugOptions,
  type ChatMessage,
  type ChatSession,
  type ToolGroupWorkCard,
  type ToolWorkCard,
} from "@tron/chat-core";
import {
  computed,
  defineComponent,
  Fragment,
  h,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  type PropType,
  useSlots,
  watch,
} from "vue";
import { cx, copyPlainText, insertNewlineAtCursor, renderMarkdownHtml } from "./utils";

function formatAguiDuration(milliseconds: number) {
  if (milliseconds < 1000) return "<1s";
  const totalSeconds = Math.max(1, Math.round(milliseconds / 1000));
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (totalMinutes < 60) return seconds ? `${totalMinutes}m ${seconds}s` : `${totalMinutes}m`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes ? `${hours}h ${minutes}m` : `${hours}h`;
}

function aguiWorkDuration(events: ChatAguiStreamEvent[]) {
  const times = events
    .map((event) => ("timestamp" in event && typeof event.timestamp === "number" ? event.timestamp : null))
    .filter((value): value is number => typeof value === "number");
  if (times.length < 2) return "<1s";
  return formatAguiDuration(Math.max(...times) - Math.min(...times));
}

function terminalAguiEvent(events: ChatAguiStreamEvent[]) {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const normalized = String(events[index]?.type ?? "").toUpperCase();
    if (normalized.includes("RUN_FINISHED") || normalized.includes("RUN_ERROR")) return events[index];
  }
  return null;
}

function hasToolParameterPayload(value: unknown) {
  if (value == null) return false;
  if (typeof value === "string") return Boolean(value.trim());
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object") return true;
  return true;
}

const MarkdownBlock = defineComponent({
  name: "MarkdownBlock",
  props: {
    content: {
      default: "",
      type: String,
    },
  },
  setup(props) {
    return () => h("div", {
      class: "tron-chat__markdown tron-chat__message-body",
      innerHTML: renderMarkdownHtml(props.content),
    });
  },
});

const CopyValueButton = defineComponent({
  name: "CopyValueButton",
  props: {
    ariaLabel: {
      required: true,
      type: String,
    },
    title: {
      required: true,
      type: String,
    },
    value: {
      required: true,
      type: String,
    },
  },
  setup(props) {
    const copied = ref(false);
    const timerId = ref<number | null>(null);

    onBeforeUnmount(() => {
      if (timerId.value !== null) window.clearTimeout(timerId.value);
    });

    async function handleCopy() {
      if (!props.value.trim()) return;
      await copyPlainText(props.value);
      copied.value = true;
      if (timerId.value !== null) window.clearTimeout(timerId.value);
      timerId.value = window.setTimeout(() => {
        copied.value = false;
      }, 1400);
    }

    return () => h(
      "button",
      {
        "aria-label": props.ariaLabel,
        class: "tron-chat__icon-button",
        disabled: !props.value.trim(),
        onClick: () => { void handleCopy(); },
        title: props.title,
        type: "button",
      },
      copied.value ? h(CheckCircle2, { size: 14 }) : h(Copy, { size: 14 }),
    );
  },
});

const ChatBubble = defineComponent({
  name: "ChatBubble",
  props: {
    currentUserDisplayName: {
      default: "",
      type: String,
    },
    message: {
      required: true,
      type: Object as PropType<ChatMessage>,
    },
  },
  emits: {
    editMessage: (_content: string) => true,
  },
  setup(props, { emit }) {
    const isUser = computed(() => props.message.role === "USER");
    const actorName = computed(() => (isUser.value ? (props.currentUserDisplayName || props.message.actor) : props.message.actor));

    return () => h(
      "article",
      { class: cx("tron-chat__message", isUser.value && "tron-chat__message--user") },
      [
        h("div", { class: "tron-chat__message-header" }, [
          h("div", { class: "tron-chat__message-actor" }, [
            isUser.value ? h(UserRound, { size: 16 }) : h(Bot, { size: 16 }),
            h("span", actorName.value),
          ]),
          h("div", { class: "tron-chat__message-actions" }, [
            h(CopyValueButton, {
              ariaLabel: "Copy raw markdown",
              title: "Copy raw markdown",
              value: props.message.content,
            }),
            isUser.value ? h(
              "button",
              {
                "aria-label": "Edit message",
                class: "tron-chat__icon-button",
                onClick: () => emit("editMessage", props.message.content),
                type: "button",
              },
              h(Pencil, { size: 14 }),
            ) : null,
            h("span", { class: "tron-chat__message-time" }, formatTime(props.message.createdAt)),
          ]),
        ]),
        isUser.value
          ? h("div", { class: "tron-chat__message-body" }, props.message.content)
          : h(MarkdownBlock, { content: props.message.content }),
      ],
    );
  },
});

const JsonBlock = defineComponent({
  name: "JsonBlock",
  props: {
    value: { required: true },
  },
  setup(props) {
    return () => {
      const preview = formatJsonBlockValue(props.value);
      if (!preview) return null;
      return h("pre", { class: "tron-chat__result-block" }, preview);
    };
  },
});

const ToolCard = defineComponent({
  name: "ToolCard",
  props: {
    card: {
      required: true,
      type: Object as PropType<ToolWorkCard>,
    },
    nested: {
      default: false,
      type: Boolean,
    },
    stepNumber: {
      default: undefined,
      type: Number,
    },
  },
  setup(props) {
    const expanded = ref(false);
    const parametersExpanded = ref(false);
    const logRef = ref<HTMLDivElement | null>(null);

    const complete = computed(() => props.card.ended || Boolean(props.card.result));
    const running = computed(() => props.card.started && !complete.value);
    const hasParameters = computed(() => hasToolParameterPayload(props.card.args));
    const parametersText = computed(() => (hasParameters.value ? formatJsonBlockValue(props.card.args) : ""));
    const resultText = computed(() => (props.card.result ? formatJsonBlockValue(props.card.result) : ""));
    const toolLabel = computed(() => toolSummaryLabel(props.card.toolName, props.card.args));
    const nestedToolLabel = computed(() => `${complete.value ? "Ran" : "Running"} ${toolLabel.value.replace(/^Run\s+/i, "")}`);

    watch([running, complete], () => {
      if (running.value) expanded.value = true;
      else if (complete.value) expanded.value = false;
    }, { immediate: true });

    watch([expanded, complete], async () => {
      if (!expanded.value || complete.value || !props.card.logs.length) return;
      await nextTick();
      const element = logRef.value;
      if (!element) return;
      element.scrollTop = element.scrollHeight;
    });

    return () => h(
      "div",
      { class: cx("tron-chat__tool-row", !props.nested && "tron-chat__tool-row--standalone") },
      [
        h(
          "button",
          {
            "aria-expanded": expanded.value,
            class: cx("tron-chat__tool-toggle", props.nested && "tron-chat__tool-toggle--nested"),
            onClick: () => {
              expanded.value = !expanded.value;
            },
            type: "button",
          },
          [
            h("span", { class: "tron-chat__tool-toggle-main" }, [
              h("span", { class: cx("tron-chat__tool-badge", props.nested ? "tron-chat__tool-badge--nested" : "tron-chat__tool-badge--standalone") }, [
                typeof props.stepNumber === "number"
                  ? String(props.stepNumber)
                  : complete.value
                    ? h(CheckCircle2, { size: 14 })
                    : running.value
                      ? h(LoaderCircle, { class: "animate-spin", size: 14 })
                      : h(Terminal, { size: 14 }),
              ]),
              h("span", { class: "tron-chat__tool-toggle-text" }, [
                h("span", { class: "tron-chat__tool-title-row" }, [
                  h("span", { class: cx("tron-chat__tool-title", props.nested && "tron-chat__tool-title--nested") }, props.nested ? nestedToolLabel.value : toolLabel.value),
                  props.nested ? null : h("span", { class: cx("tron-chat__tool-status", complete.value ? "tron-chat__tool-status--done" : "tron-chat__tool-status--running") }, complete.value ? "done" : "running"),
                ]),
              ]),
            ]),
            h("span", { class: "tron-chat__tool-chevron" }, expanded.value ? h(ChevronDown, { size: 14 }) : h(ChevronRight, { size: 14 })),
          ],
        ),
        expanded.value
          ? h("div", { class: cx("tron-chat__tool-body", props.nested ? "tron-chat__tool-body--nested" : "tron-chat__tool-body--standalone") }, [
            h("div", { class: "tron-chat__tool-sections" }, [
              hasParameters.value ? h("section", [
                h("div", { class: "tron-chat__tool-section-header", style: { marginBottom: parametersExpanded.value ? "8px" : "0" } }, [
                  h(
                    "button",
                    {
                      "aria-expanded": parametersExpanded.value,
                      class: "tron-chat__tool-subtoggle",
                      onClick: () => {
                        parametersExpanded.value = !parametersExpanded.value;
                      },
                      type: "button",
                    },
                    [
                      h("span", { class: "tron-chat__tool-subtoggle-label" }, [
                        parametersExpanded.value ? h(ChevronDown, { size: 13 }) : h(ChevronRight, { size: 13 }),
                        h("span", "Parameters"),
                      ]),
                    ],
                  ),
                  h(CopyValueButton, {
                    ariaLabel: "Copy parameters",
                    title: "Copy parameters",
                    value: parametersText.value,
                  }),
                ]),
                parametersExpanded.value ? h(JsonBlock, { value: props.card.args }) : null,
              ]) : null,
              complete.value ? h("section", [
                h("div", { class: "tron-chat__tool-section-header", style: { marginBottom: "8px" } }, [
                  h("h3", { class: "tron-chat__tool-section-title" }, "Result"),
                  h(CopyValueButton, {
                    ariaLabel: "Copy result",
                    title: "Copy result",
                    value: resultText.value,
                  }),
                ]),
                props.card.result
                  ? h("div", { style: { maxHeight: "256px", overflowY: "auto" } }, [h(JsonBlock, { value: props.card.result })])
                  : h("div", { class: "tron-chat__result-block" }, "No result payload."),
              ]) : h("section", [
                h("h3", { class: "tron-chat__tool-section-title", style: { marginBottom: "8px" } }, "Log"),
                h(
                  "div",
                  {
                    class: "tron-chat__code-block",
                    ref: logRef,
                    style: { maxHeight: "256px", overflowY: "auto" },
                  },
                  props.card.logs.length
                    ? props.card.logs.map((line, index) => h("p", {
                      key: `${props.card.id}-log-${index}`,
                      style: { margin: 0, whiteSpace: "pre-wrap" },
                    }, line))
                    : h("p", { style: { margin: 0 } }, "Waiting for tool activity..."),
                ),
              ]),
            ]),
          ]) : null,
      ],
    );
  },
});

const ToolGroupCard = defineComponent({
  name: "ToolGroupCard",
  props: {
    card: {
      required: true,
      type: Object as PropType<ToolGroupWorkCard>,
    },
  },
  setup(props) {
    const toolCards = computed(() => props.card.toolCards.filter((toolCard) => toolCard.started));
    const hasRunningTool = computed(() => toolCards.value.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result));
    const expanded = ref(false);

    watch(hasRunningTool, () => {
      expanded.value = hasRunningTool.value;
    }, { immediate: true });

    return () => {
      if (!toolCards.value.length) return null;
      return h("section", { "aria-label": "Tool run steps", class: "tron-chat__rail-entry" }, [
        h(
          "button",
          {
            "aria-expanded": expanded.value,
            class: "tron-chat__rail-entry-toggle",
            onClick: () => {
              expanded.value = !expanded.value;
            },
            type: "button",
          },
          [
            h("span", { class: "tron-chat__rail-entry-main" }, [
              h("span", { class: "tron-chat__rail-entry-icon" }, [
                h(Terminal, { class: hasRunningTool.value ? "animate-pulse" : undefined, size: 15 }),
              ]),
              h("span", { class: "tron-chat__rail-entry-summary tron-chat__rail-entry-summary--strong" }, `Ran ${toolCards.value.length} ${toolCards.value.length === 1 ? "command" : "commands"}`),
            ]),
            h("span", { class: "tron-chat__rail-entry-chevron" }, expanded.value ? h(ChevronDown, { size: 14 }) : h(ChevronRight, { size: 14 })),
          ],
        ),
        expanded.value ? h("ol", { class: "tron-chat__rail-entry-list" }, toolCards.value.map((toolCard, index) => h("li", { key: toolCard.id }, [h(ToolCard, {
          card: toolCard,
          nested: true,
          stepNumber: index + 1,
        })]))) : null,
      ]);
    };
  },
});

const ReasoningCard = defineComponent({
  name: "ReasoningCard",
  props: {
    card: {
      required: true,
      type: Object as PropType<Extract<AguiUserWorkCard, { kind: "reasoning" }>["value"]>,
    },
  },
  setup(props) {
    const hasRunningTool = computed(() => props.card.toolCards.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result));
    const shouldStayExpanded = computed(() => !props.card.ended || hasRunningTool.value);
    const expanded = ref(false);

    watch(shouldStayExpanded, () => {
      expanded.value = shouldStayExpanded.value;
    }, { immediate: true });

    const summary = computed(() => deriveReasoningPreview(props.card.content) || (props.card.ended ? "Reasoning captured." : "Thinking..."));

    return () => h("section", { "aria-label": "Thinking", class: "tron-chat__rail-entry" }, [
      h(
        "button",
        {
          "aria-expanded": expanded.value,
          class: "tron-chat__rail-entry-toggle",
          onClick: () => {
            expanded.value = !expanded.value;
          },
          type: "button",
        },
        [
          h("span", { class: "tron-chat__rail-entry-main" }, [
            h("span", { class: "tron-chat__rail-entry-icon" }, [
              h(BrainCircuit, { class: props.card.ended ? undefined : "animate-pulse", size: 15 }),
            ]),
            h("span", { class: cx("tron-chat__rail-entry-summary", !props.card.ended && "tron-chat__rail-entry-summary--pending") }, summary.value),
          ]),
          h("span", { class: "tron-chat__rail-entry-chevron" }, expanded.value ? h(ChevronDown, { size: 14 }) : h(ChevronRight, { size: 14 })),
        ],
      ),
      expanded.value && props.card.content
        ? h("div", { class: "tron-chat__rail-entry-body" }, [
          h("div", { class: "tron-chat__rail-detail" }, [h(MarkdownBlock, { content: props.card.content })]),
        ])
        : null,
    ]);
  },
});

const AguiRunWorkRail = defineComponent({
  name: "AguiRunWorkRail",
  props: {
    debugEvents: {
      default: () => [],
      type: Array as PropType<string[]>,
    },
    events: {
      default: () => [],
      type: Array as PropType<ChatAguiStreamEvent[]>,
    },
  },
  setup(props) {
    const userCards = computed(() => buildAguiUserWorkCards(props.events));
    const terminalEvent = computed(() => terminalAguiEvent(props.events));
    const expanded = ref(false);

    watch(terminalEvent, () => {
      expanded.value = !terminalEvent.value;
    }, { immediate: true });

    return () => {
      if (!userCards.value.length && !props.debugEvents.length) return null;
      return h("section", { "aria-label": "AG-UI run work", class: "tron-chat__rail" }, [
        terminalEvent.value
          ? h(
            "button",
            {
              "aria-expanded": expanded.value,
              class: "tron-chat__rail-summary",
              onClick: () => {
                expanded.value = !expanded.value;
              },
              type: "button",
            },
            [
              h("span", { class: "tron-chat__rail-summary-label" }, [`Worked for ${aguiWorkDuration(props.events)}`]),
              expanded.value ? h(ChevronDown, { size: 14 }) : h(ChevronRight, { size: 14 }),
            ],
          )
          : null,
        (!terminalEvent.value || expanded.value)
          ? h("div", { class: "tron-chat__rail-stack", style: { marginTop: terminalEvent.value ? "8px" : "0" } }, [
            userCards.value.length
              ? h("section", { "aria-label": "Agent work rail", class: "tron-chat__rail", style: { maxWidth: "none", paddingLeft: "0" } }, [
                h("div", { class: "tron-chat__rail-card-list" }, userCards.value.map((card) => (
                  card.kind === "reasoning"
                    ? h(ReasoningCard, { card: card.value, key: card.value.id })
                    : card.kind === "toolGroup"
                      ? h(ToolGroupCard, { card: card.value, key: card.value.id })
                      : h(ToolCard, { card: card.value, key: card.value.id })
                ))),
              ])
              : null,
            props.debugEvents.length
              ? h("section", { "aria-label": "AG-UI debug event cards", class: "tron-chat__rail", style: { maxWidth: "none", paddingLeft: "0" } }, [
                h("div", { class: "tron-chat__debug-list" }, props.debugEvents.map((event, index) => h("div", {
                  class: "tron-chat__debug-item",
                  key: `${event}-${index}`,
                }, [
                  h("div", { class: "tron-chat__debug-summary" }, event),
                ]))),
              ])
              : null,
          ])
          : null,
      ]);
    };
  },
});

const WaitingRail = defineComponent({
  name: "WaitingRail",
  setup() {
    return () => h("section", { "aria-label": "Waiting for model", class: "tron-chat__rail" }, [
      h("div", { class: "tron-chat__waiting" }, [
        h("span", { class: "tron-chat__waiting-line" }),
        h("span", { class: "tron-chat__rail-toggle-label" }, [
          h(LoaderCircle, { class: "animate-spin", size: 14 }),
          h("span", "Waiting for model..."),
        ]),
        h("span", { class: "tron-chat__waiting-line" }),
      ]),
    ]);
  },
});

export interface TronChatPanelProps {
  createSessionLabel?: string;
  currentUserDisplayName?: string;
  debugOptions?: ChatDebugOptions;
  disabledInput?: boolean;
  draft: string;
  emptyDescription?: string;
  emptyTitle?: string;
  inputPlaceholder?: string;
  liveAguiAnchorMessageCodes?: Record<string, string>;
  liveAguiEvents?: ChatAguiStreamEvent[];
  liveAguiPending?: boolean;
  messages: ChatMessage[];
  onCreateSession?: () => void;
  onDraftChange: (value: string) => void;
  onEditMessage?: (content: string) => void;
  onSend: () => void;
  onStop?: () => void;
  protocolEvents?: ChatAguiStreamEvent[];
  sendDisabled?: boolean;
  sendLabel?: string;
  sending?: boolean;
  session?: ChatSession | null;
  showAgui?: boolean;
  showHeader?: boolean;
  stopLabel?: string;
  title?: string;
}

export const TronChatPanel = defineComponent({
  name: "TronChatPanel",
  props: {
    createSessionLabel: { default: "Create session", type: String },
    currentUserDisplayName: { default: "", type: String },
    debugOptions: { default: undefined, type: Object as PropType<ChatDebugOptions | undefined> },
    disabledInput: { default: false, type: Boolean },
    draft: { required: true, type: String },
    emptyDescription: { default: "Create a session to start chatting.", type: String },
    emptyTitle: { default: "No session selected", type: String },
    inputPlaceholder: { default: "Enter to send, Shift+Enter for newline", type: String },
    liveAguiAnchorMessageCodes: { default: () => ({}), type: Object as PropType<Record<string, string>> },
    liveAguiEvents: { default: () => [], type: Array as PropType<ChatAguiStreamEvent[]> },
    liveAguiPending: { default: false, type: Boolean },
    messages: { required: true, type: Array as PropType<ChatMessage[]> },
    protocolEvents: { default: () => [], type: Array as PropType<ChatAguiStreamEvent[]> },
    sendDisabled: { default: false, type: Boolean },
    sendLabel: { default: "Send", type: String },
    sending: { default: false, type: Boolean },
    session: { default: null, type: Object as PropType<ChatSession | null> },
    showAgui: { default: true, type: Boolean },
    showHeader: { default: true, type: Boolean },
    stopLabel: { default: "Stop", type: String },
    title: { default: "", type: String },
  },
  emits: {
    "create-session": () => true,
    "draft-change": (_value: string) => true,
    "edit-message": (_value: string) => true,
    "send": () => true,
    "stop": () => true,
  },
  setup(props, { emit }) {
    const slots = useSlots();
    const transcriptRef = ref<HTMLElement | null>(null);
    const showJumpButton = ref(false);
    const autoStickToBottom = ref(true);

    const presentation = computed(() => buildTranscriptPresentation({
      debugOptions: props.debugOptions,
      liveAguiAnchorMessageCodes: props.liveAguiAnchorMessageCodes,
      liveAguiEvents: props.liveAguiEvents,
      liveAguiPending: props.liveAguiPending,
      messages: props.messages,
      protocolEvents: props.protocolEvents,
      sessionCode: props.session?.sessionCode ?? "",
    }));

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
      const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
      autoStickToBottom.value = distanceFromBottom <= 96;
      showJumpButton.value = distanceFromBottom > 160;
    }

    watch([() => props.messages, () => presentation.value], async () => {
      if (autoStickToBottom.value) {
        await nextTick();
        scrollToBottom("auto");
      } else {
        handleScroll();
      }
    });

    watch(() => props.session?.sessionCode, () => {
      autoStickToBottom.value = true;
      showJumpButton.value = false;
    });

    onMounted(() => {
      scrollToBottom("auto");
    });

    return () => h("div", { class: "tron-chat" }, [
      props.showHeader
        ? h("header", { class: "tron-chat__header" }, [
          h("div", { class: "tron-chat__header-content" }, slots.header?.() ?? [
            h("span", { class: "tron-chat__title" }, props.title || props.session?.title || "Chat"),
            props.session
              ? h("span", { class: "tron-chat__subtitle" }, `${props.session.sessionCode} · ${sessionAgentTagLabel(props.session.agentCode)}`)
              : null,
          ]),
        ])
        : null,
      props.session
        ? [
          h("div", { class: cx("tron-chat__content", Boolean(slots.sidebar) && "tron-chat__content--with-sidebar") }, [
            h("section", {
              "aria-label": "Chat transcript",
              class: "tron-chat__transcript",
              ref: transcriptRef,
              onScroll: handleScroll,
            }, [
              h("div", { class: "tron-chat__transcript-inner" }, props.messages.length ? props.messages.map((message) => h(Fragment, { key: message.messageCode }, [
                h(ChatBubble, {
                  currentUserDisplayName: props.currentUserDisplayName,
                  message,
                  onEditMessage: (content: string) => emit("edit-message", content),
                }),
                props.showAgui && message.messageCode === presentation.value.waitingForModelAnchorMessageCode
                  ? h(WaitingRail)
                  : null,
                props.showAgui
                  ? (presentation.value.workRailsByMessageCode.get(message.messageCode) ?? []).map((run) => h(AguiRunWorkRail, {
                    debugEvents: (presentation.value.debugEventsByRunCode.get(run.runCode) ?? []).map((event) => describeDebugEvent(event)),
                    events: run.events as ChatAguiStreamEvent[],
                    key: run.runCode,
                  }))
                  : null,
              ])) : null),
            ]),
            slots.sidebar ? h("aside", { class: "tron-chat__sidebar" }, slots.sidebar()) : null,
          ]),
          h("footer", { class: "tron-chat__footer" }, [
            showJumpButton.value
              ? h(
                "button",
                {
                  "aria-label": "Jump to latest message",
                  class: "tron-chat__button tron-chat__jump",
                  onClick: () => scrollToBottom("smooth"),
                  type: "button",
                },
                h(ArrowDown, { size: 16 }),
              )
              : null,
            h("div", { class: "tron-chat__composer" }, [
              h("textarea", {
                class: "tron-chat__textarea",
                disabled: props.disabledInput,
                onInput: (event: Event) => emit("draft-change", (event.target as HTMLTextAreaElement).value),
                onKeydown: (event: KeyboardEvent) => {
                  if (event.key !== "Enter" || event.isComposing) return;
                  event.preventDefault();
                  if (event.shiftKey) {
                    emit("draft-change", insertNewlineAtCursor(event.currentTarget as HTMLTextAreaElement));
                    return;
                  }
                  if (props.sending) {
                    emit("stop");
                    return;
                  }
                  emit("send");
                },
                placeholder: props.inputPlaceholder,
                value: props.draft,
              }),
              h("div", { class: "tron-chat__composer-row" }, [
                h("div", { class: "tron-chat__composer-addon" }, slots.footerAddon?.()),
                h(
                  "button",
                  {
                    class: cx("tron-chat__button", props.sending && "tron-chat__button--danger"),
                    disabled: props.disabledInput || (!props.sending && props.sendDisabled),
                    onClick: () => {
                      if (props.sending) {
                        emit("stop");
                        return;
                      }
                      emit("send");
                    },
                    type: "button",
                  },
                  [
                    props.sending ? h(Square, { size: 14 }) : h(Send, { size: 15 }),
                    h("span", props.sending ? props.stopLabel : props.sendLabel),
                  ],
                ),
              ]),
            ]),
          ]),
        ]
        : h("div", { class: cx("tron-chat__content", Boolean(slots.sidebar) && "tron-chat__content--with-sidebar") }, [
          h("section", { class: "tron-chat__transcript" }, [
            h("div", { class: "tron-chat__empty" }, [
              h("div", { class: "tron-chat__empty-inner" }, [
                h("div", { class: "tron-chat__empty-title" }, props.emptyTitle),
                h("div", { class: "tron-chat__empty-description" }, props.emptyDescription),
                h("div", { style: { marginTop: "16px" } }, [
                  h(
                    "button",
                    {
                      class: "tron-chat__button",
                      onClick: () => emit("create-session"),
                      type: "button",
                    },
                    props.createSessionLabel,
                  ),
                ]),
              ]),
            ]),
          ]),
          slots.sidebar ? h("aside", { class: "tron-chat__sidebar" }, slots.sidebar()) : null,
        ]),
    ]);
  },
});

export type { AguiUserWorkCard, ChatAguiStreamEvent, ChatDebugOptions, ChatMessage, ChatSession, ToolGroupWorkCard, ToolWorkCard };

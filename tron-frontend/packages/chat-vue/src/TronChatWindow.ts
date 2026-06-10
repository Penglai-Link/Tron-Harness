import {
  Bot,
  Grip,
  Minus,
  X,
} from "lucide-vue-next";
import {
  clampFloatingStateToViewport,
  defaultFloatingState,
  floatingStatesEqual,
  readFloatingState,
  reduceFloatingState,
  resolveFloatingViewportBounds,
  writeFloatingState,
  type ChatFloatingState,
} from "@tron/chat-core";
import {
  computed,
  defineComponent,
  h,
  onBeforeUnmount,
  ref,
  type PropType,
  useSlots,
  watch,
} from "vue";
import { cx } from "./utils";
import { TronChatPanel, type TronChatPanelProps } from "./TronChatPanel";

export interface TronChatWindowProps extends Omit<TronChatPanelProps, "showHeader"> {
  onStateChange?: (state: ChatFloatingState) => void;
  state?: ChatFloatingState;
  storageKey?: string;
  title?: string;
  titleSubtitle?: string;
}

export const TronChatWindow = defineComponent({
  name: "TronChatWindow",
  props: {
    createSessionLabel: { default: "Create session", type: String },
    currentUserDisplayName: { default: "", type: String },
    debugOptions: { default: undefined, type: Object as PropType<TronChatPanelProps["debugOptions"]> },
    disabledInput: { default: false, type: Boolean },
    draft: { required: true, type: String },
    emptyDescription: { default: "Create a session to start chatting.", type: String },
    emptyTitle: { default: "No session selected", type: String },
    inputPlaceholder: { default: "Enter to send, Shift+Enter for newline", type: String },
    liveAguiAnchorMessageCodes: { default: () => ({}), type: Object as PropType<Record<string, string>> },
    liveAguiEvents: { default: () => [], type: Array as PropType<TronChatPanelProps["liveAguiEvents"]> },
    liveAguiPending: { default: false, type: Boolean },
    messages: { required: true, type: Array as PropType<TronChatPanelProps["messages"]> },
    onStateChange: { default: undefined, type: Function as PropType<(state: ChatFloatingState) => void> },
    protocolEvents: { default: () => [], type: Array as PropType<TronChatPanelProps["protocolEvents"]> },
    sendDisabled: { default: false, type: Boolean },
    sendLabel: { default: "Send", type: String },
    sending: { default: false, type: Boolean },
    session: { default: null, type: Object as PropType<TronChatPanelProps["session"]> },
    showAgui: { default: true, type: Boolean },
    state: { default: undefined, type: Object as PropType<ChatFloatingState | undefined> },
    stopLabel: { default: "Stop", type: String },
    storageKey: { default: "tron.chat.window", type: String },
    title: { default: "Chat", type: String },
    titleSubtitle: { default: "Workspace assistant", type: String },
  },
  emits: {
    "create-session": () => true,
    "draft-change": (_value: string) => true,
    "edit-message": (_value: string) => true,
    "send": () => true,
    "stop": () => true,
    "state-change": (_state: ChatFloatingState) => true,
  },
  setup(props, { emit }) {
    const slots = useSlots();
    const localState = ref(readFloatingState(props.storageKey, defaultFloatingState));
    const viewportRevision = ref(0);
    const normalizedStateRef = ref<string | null>(null);
    const dragRef = ref<{ pointerId: number; startX: number; startY: number; x: number; y: number } | null>(null);
    const resizeRef = ref<{ pointerId: number; startX: number; startY: number; width: number; height: number } | null>(null);

    const isControlled = computed(() => Boolean(props.state && props.onStateChange));
    const activeState = computed(() => (isControlled.value ? (props.state as ChatFloatingState) : localState.value));

    function isInteractiveTarget(target: EventTarget | null) {
      return target instanceof Element
        && Boolean(target.closest("button, a, input, textarea, select, [role='button']"));
    }

    function buildViewportState(next: ChatFloatingState) {
      if (typeof window === "undefined") return next;
      return clampFloatingStateToViewport(next, {
        ...resolveFloatingViewportBounds(window.innerWidth),
        viewportHeight: window.innerHeight,
        viewportWidth: window.innerWidth,
      });
    }

    function applyState(next: ChatFloatingState, normalize = true) {
      const resolvedState = normalize ? buildViewportState(next) : next;
      if (isControlled.value) {
        props.onStateChange?.(resolvedState);
        emit("state-change", resolvedState);
        return;
      }
      localState.value = resolvedState;
      emit("state-change", resolvedState);
    }

    const visibleState = computed(() => {
      void viewportRevision.value;
      return buildViewportState(activeState.value);
    });

    watch([activeState, visibleState, viewportRevision], () => {
      if (typeof window === "undefined" || !visibleState.value.open) {
        normalizedStateRef.value = null;
        return;
      }

      const normalizationKey = [
        window.innerWidth,
        window.innerHeight,
        activeState.value.collapsed,
        activeState.value.height,
        activeState.value.open,
        activeState.value.width,
        activeState.value.x,
        activeState.value.y,
        visibleState.value.collapsed,
        visibleState.value.height,
        visibleState.value.open,
        visibleState.value.width,
        visibleState.value.x,
        visibleState.value.y,
      ].join(":");

      if (floatingStatesEqual(activeState.value, visibleState.value)) {
        normalizedStateRef.value = null;
        return;
      }

      if (normalizedStateRef.value === normalizationKey) return;
      normalizedStateRef.value = normalizationKey;
      applyState(visibleState.value, false);
    }, { immediate: true });

    watch(() => props.storageKey, (nextStorageKey) => {
      if (!isControlled.value) localState.value = readFloatingState(nextStorageKey, defaultFloatingState);
    });

    function handleViewportResize() {
      viewportRevision.value += 1;
    }

    watch(() => visibleState.value.open, (open) => {
      if (typeof window === "undefined") return;
      if (open) window.addEventListener("resize", handleViewportResize);
      else window.removeEventListener("resize", handleViewportResize);
    }, { immediate: true });

    onBeforeUnmount(() => {
      if (typeof window === "undefined") return;
      window.removeEventListener("resize", handleViewportResize);
    });

    watch(localState, (nextState) => {
      if (!isControlled.value) writeFloatingState(props.storageKey, nextState);
    }, { deep: true });

    const panelStyle = computed(() => ({
      height: visibleState.value.collapsed ? 72 : visibleState.value.height,
      transform: `translate(${-visibleState.value.x}px, ${-visibleState.value.y}px)`,
      width: visibleState.value.width,
    }));

    function startDrag(event: PointerEvent) {
      if (visibleState.value.collapsed || isInteractiveTarget(event.target)) return;
      dragRef.value = {
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        x: visibleState.value.x,
        y: visibleState.value.y,
      };
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    }

    function handleDrag(event: PointerEvent) {
      if (!dragRef.value || dragRef.value.pointerId !== event.pointerId) return;
      const next = reduceFloatingState(visibleState.value, {
        type: "move",
        x: dragRef.value.x - (event.clientX - dragRef.value.startX),
        y: dragRef.value.y - (event.clientY - dragRef.value.startY),
      });
      applyState(next);
    }

    function endDrag(event: PointerEvent) {
      if (!dragRef.value || dragRef.value.pointerId !== event.pointerId) return;
      dragRef.value = null;
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    }

    function startResize(event: PointerEvent) {
      resizeRef.value = {
        height: visibleState.value.height,
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        width: visibleState.value.width,
      };
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    }

    function handleResize(event: PointerEvent) {
      if (!resizeRef.value || resizeRef.value.pointerId !== event.pointerId) return;
      const next = reduceFloatingState(visibleState.value, {
        type: "resize",
        height: resizeRef.value.height - (event.clientY - resizeRef.value.startY),
        width: resizeRef.value.width - (event.clientX - resizeRef.value.startX),
      });
      applyState(next);
    }

    function endResize(event: PointerEvent) {
      if (!resizeRef.value || resizeRef.value.pointerId !== event.pointerId) return;
      resizeRef.value = null;
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    }

    return () => {
      if (!visibleState.value.open) {
        return h(
          "div",
          { class: "tron-chat-window" },
          h(
            "button",
            {
              class: "tron-chat__button tron-chat-window__fab",
              onClick: () => applyState(reduceFloatingState(visibleState.value, { type: "toggle-open", open: true })),
              type: "button",
            },
            [
              h(Bot, { size: 16 }),
              h("span", props.title),
            ],
          ),
        );
      }

      return h("div", { class: "tron-chat-window" }, [
        h(
          "div",
          {
            class: cx("tron-chat-window__panel", visibleState.value.collapsed && "tron-chat-window__panel--collapsed"),
            style: panelStyle.value,
          },
          [
            h(
              "div",
              {
                class: "tron-chat-window__header",
                onPointerdown: startDrag,
                onPointermove: handleDrag,
                onPointerup: endDrag,
                onPointercancel: endDrag,
              },
              [
                h("div", { class: "tron-chat-window__title" }, [
                  h(Bot, { size: 18 }),
                  h("div", { class: "tron-chat-window__title-text" }, [
                    h("span", { class: "tron-chat-window__title-main" }, props.title),
                    h("span", { class: "tron-chat-window__title-sub" }, props.titleSubtitle),
                  ]),
                ]),
                h("div", { class: "tron-chat-window__actions" }, [
                  h(
                    "button",
                    {
                      "aria-label": "Collapse chat",
                      class: "tron-chat-window__control",
                      onClick: () => applyState(reduceFloatingState(visibleState.value, { type: "toggle-collapsed" })),
                      type: "button",
                    },
                    h(Minus, { size: 16 }),
                  ),
                  h(
                    "button",
                    {
                      "aria-label": "Close chat",
                      class: "tron-chat-window__control",
                      onClick: () => applyState(reduceFloatingState(visibleState.value, { type: "toggle-open", open: false })),
                      type: "button",
                    },
                    h(X, { size: 16 }),
                  ),
                ]),
              ],
            ),
            visibleState.value.collapsed ? null : h("div", { class: "tron-chat-window__body" }, [
                h(TronChatPanel, {
                  createSessionLabel: props.createSessionLabel,
                  currentUserDisplayName: props.currentUserDisplayName,
                  debugOptions: props.debugOptions,
                  disabledInput: props.disabledInput,
                  draft: props.draft,
                  emptyDescription: props.emptyDescription,
                  emptyTitle: props.emptyTitle,
                  inputPlaceholder: props.inputPlaceholder,
                  liveAguiAnchorMessageCodes: props.liveAguiAnchorMessageCodes,
                  liveAguiEvents: props.liveAguiEvents,
                  liveAguiPending: props.liveAguiPending,
                  messages: props.messages,
                  onCreateSession: () => emit("create-session"),
                  onDraftChange: (value: string) => emit("draft-change", value),
                  onEditMessage: (value: string) => emit("edit-message", value),
                  onSend: () => emit("send"),
                  onStop: () => emit("stop"),
                  protocolEvents: props.protocolEvents,
                  sendDisabled: props.sendDisabled,
                  sendLabel: props.sendLabel,
                  sending: props.sending,
                  session: props.session,
                  showAgui: props.showAgui,
                  showHeader: false,
                  stopLabel: props.stopLabel,
                  title: props.title,
                }, slots),
              ]),
            visibleState.value.collapsed ? null : h(
              "button",
              {
                "aria-label": "Resize chat window",
                class: "tron-chat-window__control tron-chat-window__resize",
                onPointerdown: startResize,
                onPointermove: handleResize,
                onPointerup: endResize,
                onPointercancel: endResize,
                type: "button",
              },
              h(Grip, { size: 12 }),
            ),
          ],
        ),
      ]);
    };
  },
});

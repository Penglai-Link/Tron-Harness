import { Bot, Grip, Minus, X } from "lucide-react";
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
  useEffect,
  useCallback,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { TronChatPanel, type TronChatPanelProps } from "./TronChatPanel";

interface TronChatWindowProps extends Omit<
  TronChatPanelProps,
  "headerContent" | "showHeader"
> {
  state?: ChatFloatingState;
  storageKey?: string;
  title?: string;
  titleSubtitle?: string;
  onStateChange?: (state: ChatFloatingState) => void;
}

export function TronChatWindow({
  onStateChange,
  state,
  storageKey = "tron.chat.window",
  title = "Chat",
  titleSubtitle = "Workspace assistant",
  ...panelProps
}: TronChatWindowProps) {
  const isControlled = Boolean(state && onStateChange);
  const [localState, setLocalState] = useState(() =>
    readFloatingState(storageKey, defaultFloatingState),
  );
  const [viewportRevision, setViewportRevision] = useState(0);
  const activeState = isControlled ? (state as ChatFloatingState) : localState;
  const panelRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    x: number;
    y: number;
  } | null>(null);
  const normalizedStateRef = useRef<string | null>(null);
  const resizeRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    width: number;
    height: number;
  } | null>(null);

  function isInteractiveTarget(target: EventTarget | null) {
    return (
      target instanceof Element &&
      Boolean(
        target.closest("button, a, input, textarea, select, [role='button']"),
      )
    );
  }

  useEffect(() => {
    if (!isControlled) writeFloatingState(storageKey, localState);
  }, [isControlled, localState, storageKey]);

  const buildViewportState = useCallback((next: ChatFloatingState) => {
    if (typeof window === "undefined") {
      return next;
    }

    return clampFloatingStateToViewport(next, {
      ...resolveFloatingViewportBounds(window.innerWidth),
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
    });
  }, []);

  const applyState = useCallback(
    (next: ChatFloatingState, normalize = true) => {
      const resolvedState = normalize ? buildViewportState(next) : next;
      if (isControlled) {
        onStateChange?.(resolvedState);
        return;
      }
      setLocalState(resolvedState);
    },
    [buildViewportState, isControlled, onStateChange],
  );

  const visibleState = useMemo(
    () => buildViewportState(activeState),
    [activeState, buildViewportState, viewportRevision],
  );

  useEffect(() => {
    if (typeof window === "undefined" || !visibleState.open) {
      normalizedStateRef.current = null;
      return;
    }

    const normalizationKey = [
      window.innerWidth,
      window.innerHeight,
      activeState.collapsed,
      activeState.height,
      activeState.open,
      activeState.width,
      activeState.x,
      activeState.y,
      visibleState.collapsed,
      visibleState.height,
      visibleState.open,
      visibleState.width,
      visibleState.x,
      visibleState.y,
    ].join(":");

    if (floatingStatesEqual(activeState, visibleState)) {
      normalizedStateRef.current = null;
      return;
    }

    if (normalizedStateRef.current === normalizationKey) {
      return;
    }

    normalizedStateRef.current = normalizationKey;
    applyState(visibleState, false);
  }, [activeState, applyState, visibleState]);

  useEffect(() => {
    if (typeof window === "undefined" || !visibleState.open) {
      return;
    }

    function handleViewportResize() {
      setViewportRevision((current) => current + 1);
    }

    window.addEventListener("resize", handleViewportResize);
    return () => window.removeEventListener("resize", handleViewportResize);
  }, [visibleState.open]);

  const panelStyle = useMemo(
    () => ({
      height: visibleState.collapsed ? 72 : visibleState.height,
      transform: `translate(${-visibleState.x}px, ${-visibleState.y}px)`,
      width: visibleState.width,
    }),
    [visibleState],
  );

  function startDrag(event: ReactPointerEvent<HTMLDivElement>) {
    if (visibleState.collapsed || isInteractiveTarget(event.target)) return;
    dragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      x: visibleState.x,
      y: visibleState.y,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function handleDrag(event: ReactPointerEvent<HTMLDivElement>) {
    if (!dragRef.current || dragRef.current.pointerId !== event.pointerId)
      return;
    const next = reduceFloatingState(visibleState, {
      type: "move",
      x: dragRef.current.x - (event.clientX - dragRef.current.startX),
      y: dragRef.current.y - (event.clientY - dragRef.current.startY),
    });
    applyState(next);
  }

  function endDrag(event: ReactPointerEvent<HTMLDivElement>) {
    if (!dragRef.current || dragRef.current.pointerId !== event.pointerId)
      return;
    dragRef.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  }

  function startResize(event: ReactPointerEvent<HTMLButtonElement>) {
    resizeRef.current = {
      height: visibleState.height,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      width: visibleState.width,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function handleResize(event: ReactPointerEvent<HTMLButtonElement>) {
    if (!resizeRef.current || resizeRef.current.pointerId !== event.pointerId)
      return;
    const next = reduceFloatingState(visibleState, {
      type: "resize",
      height:
        resizeRef.current.height - (event.clientY - resizeRef.current.startY),
      width:
        resizeRef.current.width - (event.clientX - resizeRef.current.startX),
    });
    applyState(next);
  }

  function endResize(event: ReactPointerEvent<HTMLButtonElement>) {
    if (!resizeRef.current || resizeRef.current.pointerId !== event.pointerId)
      return;
    resizeRef.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  }

  return (
    <div className="tron-chat-window">
      {!visibleState.open ? (
        <button
          className="tron-chat__button tron-chat-window__fab"
          type="button"
          onClick={() =>
            applyState(
              reduceFloatingState(visibleState, {
                type: "toggle-open",
                open: true,
              }),
            )
          }
        >
          <Bot size={16} />
          <span>{title}</span>
        </button>
      ) : (
        <div
          className={
            visibleState.collapsed
              ? "tron-chat-window__panel tron-chat-window__panel--collapsed"
              : "tron-chat-window__panel"
          }
          ref={panelRef}
          style={panelStyle}
        >
          <div
            className="tron-chat-window__header"
            onPointerDown={startDrag}
            onPointerMove={handleDrag}
            onPointerUp={endDrag}
            onPointerCancel={endDrag}
          >
            <div className="tron-chat-window__title">
              <Bot size={18} />
              <div className="tron-chat-window__title-text">
                <span className="tron-chat-window__title-main">{title}</span>
              </div>
            </div>
            <div className="tron-chat-window__actions">
              <button
                className="tron-chat-window__control"
                type="button"
                aria-label="Close chat"
                onClick={() =>
                  applyState(
                    reduceFloatingState(visibleState, {
                      type: "toggle-open",
                      open: false,
                    }),
                  )
                }
              >
                <X size={16} />
              </button>
            </div>
          </div>
          {!visibleState.collapsed ? (
            <div className="tron-chat-window__body">
              <TronChatPanel {...panelProps} showHeader={false} />
            </div>
          ) : null}
          {!visibleState.collapsed ? (
            <button
              className="tron-chat-window__control tron-chat-window__resize"
              type="button"
              aria-label="Resize chat window"
              onPointerDown={startResize}
              onPointerMove={handleResize}
              onPointerUp={endResize}
              onPointerCancel={endResize}
            ></button>
          ) : null}
        </div>
      )}
    </div>
  );
}

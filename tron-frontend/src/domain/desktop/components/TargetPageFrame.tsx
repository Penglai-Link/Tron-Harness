import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";

import { DesktopBridge, sanitizeInjectionEventPayload, type DesktopInjectionEvent, type DesktopInjectionEventRequest, type DesktopBackendStatus } from "@/shared/desktop/DesktopBridge";

interface FrameHandshake {
  frameId: string;
}

interface TargetPageFrameProps {
  pageId: string;
  targetUrl: string;
  desktopBackendStatus: DesktopBackendStatus;
  injectionEnabled: boolean;
  onHandshake?: (handshake: FrameHandshake) => void;
  onFrameBlocked?: (reason: string) => void;
  onInjectionEvent?: (event: DesktopInjectionEvent) => void;
}

const FRAME_TIMEOUT_MS = 3500;

export function TargetPageFrame({
  pageId,
  targetUrl,
  desktopBackendStatus,
  injectionEnabled,
  onHandshake,
  onFrameBlocked,
  onInjectionEvent,
}: TargetPageFrameProps) {
  const bridge = useMemo(() => new DesktopBridge(), []);
  const frameRef = useRef<HTMLIFrameElement | null>(null);
  const [readyState, setReadyState] = useState<"pending" | "ready" | "blocked">("pending");
  const [frameReady, setFrameReady] = useState(false);
  const [frameId, setFrameId] = useState("");
  const fallbackDispatchedRef = useRef(false);
  const timeoutRef = useRef<number | null>(null);

  const clearTimeoutRef = useCallback(() => {
    if (timeoutRef.current !== null) {
      window.clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  const dispatchBlocked = useCallback((reason: string) => {
    if (fallbackDispatchedRef.current) {
      return;
    }

    fallbackDispatchedRef.current = true;
    setReadyState("blocked");
    onFrameBlocked?.(reason);
  }, [onFrameBlocked]);

  useEffect(() => {
    if (!injectionEnabled || !desktopBackendStatus.ready) {
      clearTimeoutRef();
      return;
    }

    const timer = window.setTimeout(() => {
      if (!frameReady) {
        dispatchBlocked("frame-handshake-timeout");
      }
    }, FRAME_TIMEOUT_MS);

    timeoutRef.current = timer;

    return () => {
      clearTimeoutRef();
    };
  }, [desktopBackendStatus.ready, dispatchBlocked, frameReady, clearTimeoutRef, injectionEnabled]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const raw = event.data;
      const sanitized = sanitizeInjectionEventPayload(raw as DesktopInjectionEventRequest);

      if (!sanitized) {
        return;
      }

      if (sanitized.pageId !== pageId) {
        return;
      }

      if (sanitized.eventType === "tron:frame-ready") {
        clearTimeoutRef();
        setReadyState("ready");
        setFrameReady(true);
        if (frameId !== sanitized.frameId) {
          setFrameId(sanitized.frameId);
        }
        onHandshake?.({ frameId: sanitized.frameId });
        return;
      }

      onInjectionEvent?.(sanitized);
    };

    window.addEventListener("message", handler);
    return () => {
      window.removeEventListener("message", handler);
    };
  }, [clearTimeoutRef, frameId, onHandshake, onInjectionEvent, pageId]);

  const iframeStyle: CSSProperties = {
    height: "100%",
    width: "100%",
    border: "0",
  };

  const handleLoad = () => {
    clearTimeoutRef();
    if (!frameReady && injectionEnabled) {
      const timer = window.setTimeout(() => {
        if (!frameReady) {
          dispatchBlocked("frame-load-complete-without-ready");
        }
      }, 1500);

      timeoutRef.current = timer;
    }
  };

  const handleError = () => {
    dispatchBlocked("iframe-load-error");
  };

  return (
    <section className="grid min-h-[500px] rounded-lg border border-[var(--color-outline-variant)]">
      <div className="h-8 border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] px-3 text-[12px] leading-8 text-[var(--color-on-surface-variant)]">
        {readyState === "ready" ? "frame handshake ok" : readyState === "blocked" ? "frame blocked or handshake timeout" : "waiting for frame handshake"}
      </div>
      <iframe
        key={pageId}
        ref={frameRef}
        onLoad={handleLoad}
        onError={handleError}
        src={targetUrl}
        style={iframeStyle}
        referrerPolicy="no-referrer"
        sandbox="allow-forms allow-modals allow-pointer-lock allow-popups allow-same-origin allow-scripts"
        title={`Tron target ${pageId}`}
      />
    </section>
  );
}

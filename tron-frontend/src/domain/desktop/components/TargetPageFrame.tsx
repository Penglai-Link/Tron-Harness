import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { ArrowLeft, ArrowRight, Check, RefreshCw, ScanSearch } from "lucide-react";

import { sanitizeInjectionEventPayload, sanitizeUrlInput, type DesktopInjectionEvent, type DesktopInjectionEventRequest, type DesktopBackendStatus } from "@/shared/desktop/DesktopBridge";

interface FrameHandshake {
  frameId: string;
}

interface TargetPageFrameProps {
  pageId: string;
  targetUrl: string;
  desktopBackendStatus: DesktopBackendStatus;
  injectionEnabled: boolean;
  onInjectionEnabledChange?: (enabled: boolean) => void;
  onTargetUrlChange?: (url: string) => void;
  onHandshake?: (handshake: FrameHandshake) => void;
  onFrameBlocked?: (reason: string) => void;
  onInjectionEvent?: (event: DesktopInjectionEvent) => void;
}

const FRAME_TIMEOUT_MS = 3500;
const BRIDGE_CHANNEL = "tron:frame-event";

export function TargetPageFrame({
  pageId,
  targetUrl,
  desktopBackendStatus,
  injectionEnabled,
  onInjectionEnabledChange,
  onTargetUrlChange,
  onHandshake,
  onFrameBlocked,
  onInjectionEvent,
}: TargetPageFrameProps) {
  const frameRef = useRef<HTMLIFrameElement | null>(null);
  const [frameReady, setFrameReady] = useState(false);
  const [frameId, setFrameId] = useState("");
  const [frameVersion, setFrameVersion] = useState(0);
  const [historyEntries, setHistoryEntries] = useState<string[]>([targetUrl]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [addressDraft, setAddressDraft] = useState(targetUrl);
  const [addressEditing, setAddressEditing] = useState(false);
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
    onFrameBlocked?.(reason);
  }, [onFrameBlocked]);

  const postInjectionToggle = useCallback(() => {
    frameRef.current?.contentWindow?.postMessage({
      channel: BRIDGE_CHANNEL,
      type: "tron:toggle-injection",
      pageId,
      enabled: injectionEnabled,
    }, "*");
  }, [injectionEnabled, pageId]);

  useEffect(() => {
    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    setFrameVersion((current) => current + 1);
    setHistoryEntries([targetUrl]);
    setHistoryIndex(0);
    setAddressDraft(targetUrl);
    setAddressEditing(false);
  }, [clearTimeoutRef, pageId, targetUrl]);

  useEffect(() => {
    postInjectionToggle();
  }, [postInjectionToggle]);

  useEffect(() => {
    if (injectionEnabled) {
      return;
    }

    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
  }, [clearTimeoutRef, injectionEnabled]);

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

  const activeUrl = historyEntries[historyIndex] ?? targetUrl;
  const activeHost = useMemo(() => {
    try {
      return new URL(activeUrl).host || activeUrl;
    } catch {
      return activeUrl;
    }
  }, [activeUrl]);

  const canGoBack = historyIndex > 0;
  const canGoForward = historyIndex < historyEntries.length - 1;

  useEffect(() => {
    if (!addressEditing) {
      setAddressDraft(activeHost);
    }
  }, [activeHost, addressEditing]);

  const handleLoad = () => {
    clearTimeoutRef();
    postInjectionToggle();
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

  const handleBack = () => {
    if (!canGoBack) {
      return;
    }

    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    setHistoryIndex((current) => Math.max(0, current - 1));
    setFrameVersion((current) => current + 1);
  };

  const handleForward = () => {
    if (!canGoForward) {
      return;
    }

    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    setHistoryIndex((current) => Math.min(historyEntries.length - 1, current + 1));
    setFrameVersion((current) => current + 1);
  };

  const handleRefresh = () => {
    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    setFrameVersion((current) => current + 1);
  };

  const handleInjectionToggle = () => {
    onInjectionEnabledChange?.(!injectionEnabled);
  };

  const loadTargetAddress = () => {
    const normalized = sanitizeUrlInput(addressDraft);
    if (!normalized.startsWith("http://") && !normalized.startsWith("https://")) {
      setAddressDraft(activeUrl);
      setAddressEditing(false);
      return;
    }

    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    setHistoryEntries((current) => [...current.slice(0, historyIndex + 1), normalized]);
    setHistoryIndex((current) => current + 1);
    setFrameVersion((current) => current + 1);
    setAddressDraft(new URL(normalized).host || normalized);
    setAddressEditing(false);
    onTargetUrlChange?.(normalized);
  };

  const handleAddressFocus = () => {
    setAddressDraft(activeUrl);
    setAddressEditing(true);
  };

  const handleAddressKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      loadTargetAddress();
      return;
    }

    if (event.key === "Escape") {
      setAddressDraft(activeHost);
      setAddressEditing(false);
    }
  };

  return (
    <section className="overflow-hidden">
      <div className="flex h-14 items-center justify-between border-b border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface-strong)] px-4">
        <div className="flex items-center gap-5 text-[var(--color-on-surface-variant)]">
          <button
            aria-label="Go back"
            className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)] disabled:opacity-40"
            disabled={!canGoBack}
            type="button"
            onClick={handleBack}
          >
            <ArrowLeft className="size-[18px]" />
          </button>
          <button
            aria-label="Go forward"
            className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)] disabled:opacity-40"
            disabled={!canGoForward}
            type="button"
            onClick={handleForward}
          >
            <ArrowRight className="size-[18px]" />
          </button>
          <button
            aria-label="Refresh frame"
            className="inline-flex items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]"
            type="button"
            onClick={handleRefresh}
          >
            <RefreshCw className="size-[18px]" />
          </button>
        </div>
        <form
          className={`${addressEditing ? 'border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface-strong)]' : 'justify-center'} flex h-9 min-w-[220px] max-w-[420px] flex-1 items-center gap-2 rounded-full px-4 text-sm text-[var(--color-on-surface)] md:flex-none`}
          onSubmit={(event) => {
            event.preventDefault();
            loadTargetAddress();
          }}
        >
          <input
            aria-label="Target address"
            className={`${addressEditing ? 'text-left' : ''} min-w-0 flex-1 bg-transparent text-center text-sm text-[var(--color-on-surface)] outline-none placeholder:text-[var(--color-on-surface-variant)]`}
            spellCheck={false}
            value={addressDraft}
            onChange={(event) => setAddressDraft(event.target.value)}
            onFocus={handleAddressFocus}
            onKeyDown={handleAddressKeyDown}
          />
          {addressEditing ? (
            <button
              aria-label="Load target address"
              className="inline-flex shrink-0 items-center justify-center text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]"
              type="submit"
            >
              <Check className="size-[16px]" />
            </button>
          ) : null}
        </form>
        <div className="flex items-center text-[var(--color-on-surface-variant)]">
          <button
            aria-label={injectionEnabled ? "Disable injection" : "Enable injection"}
            aria-pressed={injectionEnabled}
            className={`inline-flex items-center justify-center transition-colors hover:text-[var(--color-on-surface)] ${
              injectionEnabled ? "text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)]"
            }`}
            type="button"
            onClick={handleInjectionToggle}
          >
            <ScanSearch className="size-[18px]" />
          </button>
        </div>
      </div>
      <iframe
        key={`${pageId}-${frameVersion}-${historyIndex}`}
        data-frame-key={`${frameVersion}`}
        ref={frameRef}
        onLoad={handleLoad}
        onError={handleError}
        name={pageId}
        src={activeUrl}
        style={iframeStyle}
        referrerPolicy="no-referrer"
        title={`Tron target ${pageId}`}
        className="block min-h-[560px] bg-[var(--color-elevated-surface-strong)]"
      />
    </section>
  );
}

import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { ArrowLeft, ArrowRight, Check, ExternalLink, MessageCirclePlus, RefreshCw } from "lucide-react";

import {
  DesktopBridge,
  sanitizeInjectionEventPayload,
  sanitizeUrlInput,
  type DesktopBackendStatus,
  type DesktopInjectionEvent,
  type DesktopInjectionEventRequest,
} from "@/shared/desktop/DesktopBridge";

export interface TargetPageFrameHandle {
  open: (url: string) => void;
}

export interface TargetPageFrameInspectionResult {
  dom: Record<string, unknown>;
  event: DesktopInjectionEvent;
  remark: string;
}

interface TargetPageFrameProps {
  onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
}

const FRAME_TIMEOUT_MS = 3500;
const BRIDGE_CHANNEL = "tron:frame-event";
const DEFAULT_PAGE_ID_PREFIX = "desktop";

function createPageId() {
  return `${DEFAULT_PAGE_ID_PREFIX}-${Date.now()}`;
}

function describeOpenError(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  if (typeof error === "string" && error.trim()) {
    return error.trim();
  }

  return "unknown error";
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
}

function readString(payload: Record<string, unknown> | null, key: string): string {
  const value = payload?.[key];

  return typeof value === "string" ? value : "";
}

function displayAddress(url: string): string {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

function preserveUrlHash(originalUrl: string, resolvedUrl: string): string {
  try {
    const original = new URL(originalUrl);
    const resolved = new URL(resolvedUrl || originalUrl);
    if (!resolved.hash && original.hash) {
      resolved.hash = original.hash;
    }

    return resolved.href;
  } catch {
    return resolvedUrl || originalUrl;
  }
}

export const TargetPageFrame = forwardRef<TargetPageFrameHandle, TargetPageFrameProps>(function TargetPageFrame({
  onInspectionConfirm,
}, ref) {
  const bridge = useMemo(() => new DesktopBridge(), []);
  const frameRef = useRef<HTMLIFrameElement | null>(null);
  const [visible, setVisible] = useState(false);
  const [pageId, setPageId] = useState(createPageId);
  const [injectionEnabled, setInjectionEnabled] = useState(false);
  const [desktopBackendStatus, setDesktopBackendStatus] = useState<DesktopBackendStatus>({
    backendUrl: "",
    backendPort: 0,
    ready: false,
    injectionEnabled: false,
  });
  const [errorMessage, setErrorMessage] = useState("");
  const [frameReady, setFrameReady] = useState(false);
  const [frameId, setFrameId] = useState("");
  const [frameVersion, setFrameVersion] = useState(0);
  const [historyEntries, setHistoryEntries] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [addressDraft, setAddressDraft] = useState("");
  const [addressEditing, setAddressEditing] = useState(false);
  const [unsupportedPageActive, setUnsupportedPageActive] = useState(false);
  const fallbackDispatchedRef = useRef(false);
  const embeddingCheckSequenceRef = useRef(0);
  const timeoutRef = useRef<number | null>(null);
  const activeUrl = historyEntries[historyIndex] ?? "";
  const activeDisplayAddress = displayAddress(activeUrl);

  const clearTimeoutRef = useCallback(() => {
    if (timeoutRef.current !== null) {
      window.clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  const openInSystemBrowser = useCallback((url: string, targetPageId = pageId) => {
    void (async () => {
      setErrorMessage("");

      let result;
      try {
        result = await bridge.openTargetPage({
          mode: "browser",
          pageId: targetPageId,
          url,
        });
      } catch (error) {
        setErrorMessage(`系统浏览器打开失败：${describeOpenError(error)}`);
        return;
      }

      if (result.opened) {
        return;
      }

      if (result.reason === "running-in-browser") {
        window.open(url, "_blank");
        return;
      }

      setErrorMessage(`系统浏览器打开失败：${result.reason ?? "not available"}`);
    })();
  }, [bridge, pageId]);

  const dispatchBlocked = useCallback((reason: string) => {
    if (fallbackDispatchedRef.current) {
      return;
    }

    fallbackDispatchedRef.current = true;
    setUnsupportedPageActive(true);
    setFrameReady(false);
    setFrameId("");
    setErrorMessage(`Target page likely blocks iframe embedding (${reason}).`);
  }, []);

  const checkEmbeddingSupport = useCallback(async (url: string) => {
    const result = await bridge.checkTargetPageEmbedding({ url });

    return {
      blocked: result.blocked,
      reason: result.reason,
      url: preserveUrlHash(url, result.url || url),
    };
  }, [bridge]);

  const applyEmbeddingResult = useCallback((url: string, index: number) => {
    const requestSequence = embeddingCheckSequenceRef.current + 1;
    embeddingCheckSequenceRef.current = requestSequence;

    void (async () => {
      const result = await checkEmbeddingSupport(url);
      if (requestSequence !== embeddingCheckSequenceRef.current) {
        return;
      }

      setHistoryEntries((current) => {
        const next = [...current];
        if (next[index] === url) {
          next[index] = result.url;
        }
        return next;
      });
      setAddressDraft(displayAddress(result.url));
      setAddressEditing(false);

      if (result.blocked) {
        setUnsupportedPageActive(true);
        setErrorMessage(`Target page does not support iframe embedding (${result.reason ?? "response-policy"}).`);
        return;
      }

      setUnsupportedPageActive(false);
      setErrorMessage("");
      setFrameVersion((current) => current + 1);
    })();
  }, [checkEmbeddingSupport]);

  const postInjectionToggle = useCallback(() => {
    frameRef.current?.contentWindow?.postMessage({
      channel: BRIDGE_CHANNEL,
      type: "tron:toggle-injection",
      pageId,
      enabled: injectionEnabled,
    }, "*");
  }, [injectionEnabled, pageId]);

  useEffect(() => {
    let isMounted = true;

    void (async () => {
      const status = await bridge.getBackendStatus();
      if (isMounted) {
        setDesktopBackendStatus(status);
      }
    })();

    return () => {
      isMounted = false;
    };
  }, [bridge]);

  useEffect(() => {
    void (async () => {
      const enabled = await bridge.setInjectionEnabled(injectionEnabled);
      setDesktopBackendStatus((current) => ({
        ...current,
        injectionEnabled: enabled,
      }));
    })();
  }, [bridge, injectionEnabled]);

  const open = useCallback((url: string) => {
    const normalized = sanitizeUrlInput(url);
    if (!normalized.startsWith("http://") && !normalized.startsWith("https://")) {
      return;
    }

    const nextPageId = createPageId();
    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setPageId(nextPageId);
    setInjectionEnabled(false);
    setErrorMessage("");
    setFrameReady(false);
    setFrameId("");
    setVisible(true);
    setUnsupportedPageActive(false);
    setHistoryEntries([normalized]);
    setHistoryIndex(0);
    setAddressDraft(normalized);
    setAddressEditing(false);
    applyEmbeddingResult(normalized, 0);
  }, [applyEmbeddingResult, clearTimeoutRef]);

  useImperativeHandle(ref, () => ({
    open,
  }), [open]);

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
        setErrorMessage("");
        if (frameId !== sanitized.frameId) {
          setFrameId(sanitized.frameId);
        }
        return;
      }

      if (sanitized.eventType === "tron:inspection-exited") {
        clearTimeoutRef();
        setFrameReady(false);
        setInjectionEnabled(false);
        return;
      }

      if (sanitized.eventType === "tron:navigation-requested") {
        const requestedUrl = sanitizeUrlInput(readString(asRecord(sanitized.payload), "url"));
        if (!requestedUrl.startsWith("http://") && !requestedUrl.startsWith("https://")) {
          return;
        }

        if (activeUrl === requestedUrl) {
          return;
        }

        const nextIndex = historyIndex + 1;
        fallbackDispatchedRef.current = false;
        setUnsupportedPageActive(false);
        setErrorMessage("");
        setFrameReady(false);
        setFrameId("");
        setHistoryEntries((current) => {
          const currentUrl = current[historyIndex] ?? "";
          if (currentUrl === requestedUrl) {
            return current;
          }

          return [...current.slice(0, historyIndex + 1), requestedUrl];
        });
        setHistoryIndex(nextIndex);
        setAddressDraft(displayAddress(requestedUrl));
        setAddressEditing(false);
        applyEmbeddingResult(requestedUrl, nextIndex);
        return;
      }

      if (sanitized.eventType === "tron:node-inspected") {
        const dom = asRecord(sanitized.payload) ?? {};
        void bridge.emitInjectionEvent(sanitized);

        onInspectionConfirm?.({
          dom,
          event: sanitized,
          remark: readString(dom, "remark"),
        });
      }
    };

    window.addEventListener("message", handler);
    return () => {
      window.removeEventListener("message", handler);
    };
  }, [activeUrl, applyEmbeddingResult, bridge, clearTimeoutRef, frameId, historyIndex, onInspectionConfirm, pageId]);

  const iframeStyle: CSSProperties = {
    height: "100%",
    width: "100%",
    border: "0",
  };

  const canGoBack = historyIndex > 0;
  const canGoForward = historyIndex < historyEntries.length - 1;
  const addressValue = addressEditing ? addressDraft : activeDisplayAddress;

  useEffect(() => {
    if (!addressEditing) {
      setAddressDraft(activeDisplayAddress);
    }
  }, [activeDisplayAddress, addressEditing]);

  const handleLoad = () => {
    clearTimeoutRef();
    postInjectionToggle();
    window.setTimeout(postInjectionToggle, 0);
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
    const nextIndex = Math.max(0, historyIndex - 1);
    setUnsupportedPageActive(false);
    setHistoryIndex(nextIndex);
    applyEmbeddingResult(historyEntries[nextIndex] ?? "", nextIndex);
  };

  const handleForward = () => {
    if (!canGoForward) {
      return;
    }

    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    const nextIndex = Math.min(historyEntries.length - 1, historyIndex + 1);
    setUnsupportedPageActive(false);
    setHistoryIndex(nextIndex);
    applyEmbeddingResult(historyEntries[nextIndex] ?? "", nextIndex);
  };

  const handleRefresh = () => {
    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    if (unsupportedPageActive) {
      return;
    }

    setFrameVersion((current) => current + 1);
  };

  const handleOpenInSystemBrowser = () => {
    openInSystemBrowser(activeUrl);
  };

  const handleInjectionToggle = () => {
    setInjectionEnabled((current) => !current);
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
    setUnsupportedPageActive(false);
    const nextIndex = historyIndex + 1;
    setHistoryEntries((current) => [...current.slice(0, historyIndex + 1), normalized]);
    setHistoryIndex(nextIndex);
    setAddressDraft(displayAddress(normalized));
    setAddressEditing(false);
    applyEmbeddingResult(normalized, nextIndex);
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
      setAddressDraft(activeDisplayAddress);
      setAddressEditing(false);
    }
  };

  if (!visible || !activeUrl) {
    return null;
  }

  return (
    <section className="overflow-hidden">
      {errorMessage ? <p className="mb-2 text-sm text-[var(--color-error)]">{errorMessage}</p> : null}
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
            value={addressValue}
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
            aria-label={injectionEnabled ? "Disable inspection" : "Enable inspection"}
            aria-pressed={injectionEnabled}
            className={`inline-flex items-center justify-center transition-colors hover:text-[var(--color-on-surface)] ${
              injectionEnabled ? "text-[var(--color-on-surface)]" : "text-[var(--color-on-surface-variant)]"
            }`}
            type="button"
            onClick={handleInjectionToggle}
          >
            <MessageCirclePlus className="size-[18px]" />
          </button>
        </div>
      </div>
      {unsupportedPageActive ? (
        <div className="flex min-h-[560px] items-center justify-center bg-[var(--color-elevated-surface-strong)] text-[var(--color-on-surface-variant)]">
          <div className="flex max-w-[420px] flex-col items-center gap-4 text-center">
            <ExternalLink className="size-8" />
            <div className="space-y-1">
              <p className="text-sm text-[var(--color-on-surface)]">当前页面不支持在内置浏览器中打开</p>
              <p className="text-xs">请使用系统浏览器打开该页面。</p>
            </div>
            <button
              className="inline-flex h-9 items-center gap-2 rounded-full border border-[var(--color-outline-variant)] px-4 text-sm text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-elevated-surface)]"
              type="button"
              onClick={handleOpenInSystemBrowser}
            >
              <ExternalLink className="size-4" />
              使用系统浏览器打开
            </button>
          </div>
        </div>
      ) : (
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
          sandbox="allow-forms allow-modals allow-pointer-lock allow-same-origin allow-scripts allow-top-navigation-by-user-activation"
          title={`Tron target ${pageId}`}
          className="block min-h-[560px] bg-[var(--color-elevated-surface-strong)]"
        />
      )}
    </section>
  );
});

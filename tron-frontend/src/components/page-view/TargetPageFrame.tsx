import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type FocusEvent,
  type KeyboardEvent,
} from "react";

import {
  DesktopBridge,
  sanitizeInjectionEventPayload,
  sanitizeUrlInput,
  type DesktopBackendStatus,
  type DesktopInjectionEvent,
  type DesktopInjectionEventRequest,
  type DesktopTargetBounds,
} from "@/shared/desktop/DesktopBridge";
import { useTabVisibility } from "@/components/editable-tabs";
import { TargetPageChrome } from "./TargetPageChrome";
import { TargetPageEmpty } from "./TargetPageEmpty";
import { TargetPageUnsupported } from "./TargetPageUnsupported";

export interface TargetPageFrameHandle {
  open: (url: string) => void;
}

export interface TargetPageFrameInspectionResult {
  dom: Record<string, unknown>;
  event: DesktopInjectionEvent;
  remark: string;
}

interface TargetPageFrameProps {
  inspectionMode?: boolean;
  onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
  onUrlChange?: (url: string) => void;
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

function readString(
  payload: Record<string, unknown> | null,
  key: string,
): string {
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

export const TargetPageFrame = forwardRef<
  TargetPageFrameHandle,
  TargetPageFrameProps
>(function TargetPageFrame(
  { inspectionMode = false, onInspectionConfirm, onUrlChange },
  ref,
) {
  const bridge = useMemo(() => new DesktopBridge(), []);
  const frameRef = useRef<HTMLIFrameElement | null>(null);
  const targetViewportRef = useRef<HTMLDivElement | null>(null);
  const addressFormRef = useRef<HTMLFormElement | null>(null);
  const addressInputRef = useRef<HTMLInputElement | null>(null);
  const addressSubmitPendingRef = useRef(false);
  const [visible, setVisible] = useState(false);
  const [pageId, setPageId] = useState(createPageId);
  const [injectionEnabled, setInjectionEnabled] = useState(false);
  const [desktopBackendStatus, setDesktopBackendStatus] =
    useState<DesktopBackendStatus>({
      backendUrl: "",
      backendPort: 0,
      ready: false,
      injectionEnabled: false,
    });
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
  const isDesktopHost = bridge.isDesktop();
  const panelVisible = useTabVisibility();

  useEffect(() => {
    if (!inspectionMode) {
      setInjectionEnabled(false);
    }
  }, [inspectionMode]);

  const clearTimeoutRef = useCallback(() => {
    if (timeoutRef.current !== null) {
      window.clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  const openInSystemBrowser = useCallback(
    (url: string, targetPageId = pageId) => {
      void (async () => {
        let result;
        try {
          result = await bridge.openTargetPage({
            mode: "browser",
            pageId: targetPageId,
            url,
          });
        } catch (error) {
          return;
        }

        if (result.opened) {
          return;
        }

        if (result.reason === "running-in-browser") {
          window.open(url, "_blank");
          return;
        }
      })();
    },
    [bridge, pageId],
  );

  const dispatchBlocked = useCallback((reason: string) => {
    if (fallbackDispatchedRef.current) {
      return;
    }

    fallbackDispatchedRef.current = true;
    setUnsupportedPageActive(true);
    setFrameReady(false);
    setFrameId("");
  }, []);

  const checkEmbeddingSupport = useCallback(
    async (url: string) => {
      const result = await bridge.checkTargetPageEmbedding({ url });

      return {
        blocked: result.blocked,
        reason: result.reason,
        url: preserveUrlHash(url, result.url || url),
      };
    },
    [bridge],
  );

  const readTargetViewportBounds = useCallback(():
    | DesktopTargetBounds
    | undefined => {
    const viewport = targetViewportRef.current;
    if (!viewport) {
      return undefined;
    }

    const rect = viewport.getBoundingClientRect();
    const visibleWidth =
      Math.min(rect.right, window.innerWidth) - Math.max(rect.left, 0);
    const visibleHeight =
      Math.min(rect.bottom, window.innerHeight) - Math.max(rect.top, 0);
    if (visibleWidth <= 0 || visibleHeight <= 0) {
      return {
        height: 0,
        width: 0,
        x: 0,
        y: 0,
      };
    }

    return {
      height: Math.max(0, Math.round(rect.height)),
      width: Math.max(0, Math.round(rect.width)),
      x: Math.round(rect.left),
      y: Math.round(rect.top),
    };
  }, []);

  const syncTargetViewBounds = useCallback(() => {
    if (
      !isDesktopHost ||
      !panelVisible ||
      !visible ||
      unsupportedPageActive ||
      !pageId
    ) {
      return;
    }

    const bounds = readTargetViewportBounds();
    if (!bounds) {
      return;
    }

    void bridge.resizeTargetPageView({
      bounds,
      pageId,
    });
  }, [
    bridge,
    isDesktopHost,
    pageId,
    panelVisible,
    readTargetViewportBounds,
    unsupportedPageActive,
    visible,
  ]);

  const applyEmbeddingResult = useCallback(
    (url: string, index: number, targetPageId = pageId) => {
      const requestSequence = embeddingCheckSequenceRef.current + 1;
      embeddingCheckSequenceRef.current = requestSequence;

      void (async () => {
        if (bridge.isDesktop()) {
          const result = await bridge.openTargetPage({
            bounds: readTargetViewportBounds(),
            mode: "view",
            pageId: targetPageId,
            url,
          });
          if (requestSequence !== embeddingCheckSequenceRef.current) {
            return;
          }

          setAddressDraft(displayAddress(url));
          setAddressEditing(false);
          setUnsupportedPageActive(!result.opened);
          // The visibility effect can run before the main process creates the
          // WebContentsView. Re-apply it after openTargetPage resolves so a
          // hidden tab stays detached and a visible tab is attached promptly.
          const shouldBeVisible = panelVisible && result.opened;
          void bridge.setTargetPageViewVisibility({
            bounds: shouldBeVisible ? readTargetViewportBounds() : undefined,
            pageId: targetPageId,
            visible: shouldBeVisible,
          });
          window.requestAnimationFrame(syncTargetViewBounds);
          return;
        }

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
          return;
        }

        setUnsupportedPageActive(false);
        setFrameVersion((current) => current + 1);
      })();
    },
    [
      bridge,
      checkEmbeddingSupport,
      pageId,
      panelVisible,
      readTargetViewportBounds,
      syncTargetViewBounds,
    ],
  );

  const postInjectionToggle = useCallback(() => {
    frameRef.current?.contentWindow?.postMessage(
      {
        channel: BRIDGE_CHANNEL,
        type: "tron:toggle-injection",
        pageId,
        enabled: injectionEnabled,
      },
      "*",
    );
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

  const open = useCallback(
    (url: string) => {
      const normalized = sanitizeUrlInput(url);

      if (!normalized) {
        if (isDesktopHost && visible) {
          void bridge.closeTargetPage({ pageId });
        }
        clearTimeoutRef();
        fallbackDispatchedRef.current = false;
        setPageId(createPageId());
        setInjectionEnabled(false);
        setFrameReady(false);
        setFrameId("");
        setVisible(true);
        setUnsupportedPageActive(false);
        setHistoryEntries([]);
        setHistoryIndex(0);
        setAddressDraft("");
        setAddressEditing(true);
        addressInputRef.current?.focus();
        return;
      }

      if (
        !normalized.startsWith("http://") &&
        !normalized.startsWith("https://")
      ) {
        return;
      }

      const nextPageId = createPageId();
      if (isDesktopHost && visible) {
        void bridge.closeTargetPage({ pageId });
      }
      clearTimeoutRef();
      fallbackDispatchedRef.current = false;
      setPageId(nextPageId);
      setInjectionEnabled(false);
      setFrameReady(false);
      setFrameId("");
      setVisible(true);
      setUnsupportedPageActive(false);
      setHistoryEntries([normalized]);
      setHistoryIndex(0);
      setAddressDraft(normalized);
      setAddressEditing(false);
      applyEmbeddingResult(normalized, 0, nextPageId);
    },
    [
      applyEmbeddingResult,
      bridge,
      clearTimeoutRef,
      isDesktopHost,
      pageId,
      visible,
    ],
  );

  useImperativeHandle(
    ref,
    () => ({
      open,
    }),
    [open],
  );

  useEffect(() => {
    postInjectionToggle();
  }, [postInjectionToggle]);

  useEffect(() => {
    if (!isDesktopHost || !visible) {
      return undefined;
    }

    const currentPageId = pageId;

    return () => {
      void bridge.closeTargetPage({ pageId: currentPageId });
    };
  }, [bridge, isDesktopHost, pageId, visible]);

  useEffect(() => {
    if (!isDesktopHost || !visible) {
      return undefined;
    }

    let animationFrame = 0;
    const scheduleSync = () => {
      if (animationFrame) {
        window.cancelAnimationFrame(animationFrame);
      }
      animationFrame = window.requestAnimationFrame(syncTargetViewBounds);
    };

    const observer = new ResizeObserver(scheduleSync);
    const viewport = targetViewportRef.current;
    if (viewport) {
      observer.observe(viewport);
    }

    scheduleSync();
    window.addEventListener("resize", scheduleSync);
    window.addEventListener("scroll", scheduleSync, true);

    return () => {
      if (animationFrame) {
        window.cancelAnimationFrame(animationFrame);
      }
      observer.disconnect();
      window.removeEventListener("resize", scheduleSync);
      window.removeEventListener("scroll", scheduleSync, true);
    };
  }, [isDesktopHost, syncTargetViewBounds, visible]);

  useEffect(() => {
    if (!isDesktopHost || !visible || !pageId) {
      return;
    }

    const shouldBeVisible = panelVisible && !unsupportedPageActive;
    void bridge.setTargetPageViewVisibility({
      bounds: shouldBeVisible ? readTargetViewportBounds() : undefined,
      pageId,
      visible: shouldBeVisible,
    });
  }, [
    bridge,
    isDesktopHost,
    pageId,
    panelVisible,
    readTargetViewportBounds,
    unsupportedPageActive,
    visible,
  ]);

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
  }, [
    desktopBackendStatus.ready,
    dispatchBlocked,
    frameReady,
    clearTimeoutRef,
    injectionEnabled,
  ]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const raw = event.data;
      const sanitized = sanitizeInjectionEventPayload(
        raw as DesktopInjectionEventRequest,
      );

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
        return;
      }

      if (sanitized.eventType === "tron:inspection-exited") {
        clearTimeoutRef();
        setFrameReady(false);
        setInjectionEnabled(false);
        return;
      }

      if (sanitized.eventType === "tron:navigation-requested") {
        const requestedUrl = sanitizeUrlInput(
          readString(asRecord(sanitized.payload), "url"),
        );
        if (
          !requestedUrl.startsWith("http://") &&
          !requestedUrl.startsWith("https://")
        ) {
          return;
        }

        if (activeUrl === requestedUrl) {
          return;
        }

        const nextIndex = historyIndex + 1;
        fallbackDispatchedRef.current = false;
        setUnsupportedPageActive(false);
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
        onUrlChange?.(requestedUrl);
        applyEmbeddingResult(requestedUrl, nextIndex);
        return;
      }

      if (inspectionMode && sanitized.eventType === "tron:node-inspected") {
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
  }, [
    activeUrl,
    applyEmbeddingResult,
    bridge,
    clearTimeoutRef,
    frameId,
    historyIndex,
    inspectionMode,
    onInspectionConfirm,
    onUrlChange,
    pageId,
  ]);

  const canGoBack = historyIndex > 0;
  const canGoForward = historyIndex < historyEntries.length - 1;
  const addressValue = addressEditing ? addressDraft : activeDisplayAddress;

  useEffect(() => {
    if (document.activeElement === addressInputRef.current) {
      return;
    }

    if (!addressEditing) {
      setAddressDraft(activeDisplayAddress);
    }
  }, [activeDisplayAddress, addressEditing]);

  useEffect(() => {
    if (!visible || activeUrl || !addressEditing) {
      return;
    }

    const frame = window.requestAnimationFrame(() => {
      addressInputRef.current?.focus();
    });

    return () => window.cancelAnimationFrame(frame);
  }, [activeUrl, addressEditing, visible]);

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
    setInjectionEnabled(false);
    if (unsupportedPageActive) {
      return;
    }

    if (isDesktopHost) {
      applyEmbeddingResult(activeUrl, historyIndex, pageId);
      return;
    }

    setFrameVersion((current) => current + 1);
  };

  const handleOpenInSystemBrowser = () => {
    openInSystemBrowser(activeUrl);
  };

  const handleInjectionToggle = () => {
    if (!inspectionMode) {
      return;
    }

    setInjectionEnabled((current) => !current);
  };

  const blurAddressInput = () => {
    addressInputRef.current?.blur();
  };

  const loadTargetAddress = (rawAddress = addressDraft) => {
    const normalized = sanitizeUrlInput(rawAddress);
    if (
      !normalized.startsWith("http://") &&
      !normalized.startsWith("https://")
    ) {
      setAddressDraft(activeUrl);
      setAddressEditing(false);
      blurAddressInput();
      return;
    }

    clearTimeoutRef();
    fallbackDispatchedRef.current = false;
    setFrameReady(false);
    setFrameId("");
    setInjectionEnabled(false);
    setUnsupportedPageActive(false);
    const nextIndex = historyEntries.length === 0 ? 0 : historyIndex + 1;
    setHistoryEntries((current) =>
      current.length === 0
        ? [normalized]
        : [...current.slice(0, historyIndex + 1), normalized],
    );
    setHistoryIndex(nextIndex);
    setAddressDraft(displayAddress(normalized));
    setAddressEditing(false);
    onUrlChange?.(normalized);
    blurAddressInput();
    applyEmbeddingResult(normalized, nextIndex);
  };

  const handleAddressFocus = () => {
    if (addressInputRef.current) {
      addressInputRef.current.value = activeUrl;
    }
    setAddressDraft(activeUrl);
    setAddressEditing(true);
  };

  const handleAddressBlur = (event: FocusEvent<HTMLFormElement>) => {
    if (addressSubmitPendingRef.current) {
      return;
    }

    const nextTarget = event.relatedTarget;
    if (
      nextTarget instanceof Node &&
      addressFormRef.current?.contains(nextTarget)
    ) {
      return;
    }

    window.setTimeout(() => {
      if (addressFormRef.current?.contains(document.activeElement)) {
        return;
      }

      setAddressDraft(activeDisplayAddress);
      setAddressEditing(false);
    }, 0);
  };

  const handleAddressKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      loadTargetAddress(event.currentTarget.value);
      return;
    }

    if (event.key === "Escape") {
      setAddressDraft(activeDisplayAddress);
      setAddressEditing(false);
    }
  };

  if (!visible) {
    return null;
  }

  const renderEmbeddedFrame = !isDesktopHost;

  return (
    <section className="overflow-hidden h-full flex flex-col">
      <TargetPageChrome
        addressEditing={addressEditing}
        addressFormRef={addressFormRef}
        addressInputRef={addressInputRef}
        addressValue={addressValue}
        canGoBack={canGoBack}
        canGoForward={canGoForward}
        inspectionMode={inspectionMode}
        injectionEnabled={injectionEnabled}
        isDesktopHost={isDesktopHost}
        onAddressBlur={handleAddressBlur}
        onAddressChange={setAddressDraft}
        onAddressFocus={handleAddressFocus}
        onAddressKeyDown={handleAddressKeyDown}
        onAddressSubmit={(event) => {
          event.preventDefault();
          const submittedAddress = new FormData(event.currentTarget).get(
            "targetAddress",
          );
          addressSubmitPendingRef.current = false;
          loadTargetAddress(
            typeof submittedAddress === "string"
              ? submittedAddress
              : addressDraft,
          );
        }}
        onBack={handleBack}
        onForward={handleForward}
        onInjectionToggle={handleInjectionToggle}
        onRefresh={handleRefresh}
        onSubmitPointerDown={() => {
          addressSubmitPendingRef.current = true;
          window.setTimeout(() => {
            addressSubmitPendingRef.current = false;
          }, 0);
        }}
      />
      {unsupportedPageActive ? (
        <TargetPageUnsupported
          onOpenInSystemBrowser={handleOpenInSystemBrowser}
        />
      ) : !activeUrl ? (
        <TargetPageEmpty targetViewportRef={targetViewportRef} />
      ) : renderEmbeddedFrame ? (
        <iframe
          key={`${pageId}-${frameVersion}-${historyIndex}`}
          data-frame-key={`${frameVersion}`}
          ref={frameRef}
          onLoad={handleLoad}
          onError={handleError}
          name={pageId}
          src={activeUrl}
          referrerPolicy="no-referrer"
          sandbox="allow-forms allow-modals allow-pointer-lock allow-same-origin allow-scripts allow-top-navigation-by-user-activation"
          title={`Tron target ${pageId}`}
          className="block border-0 w-full h-full  min-h-[560px] flex-auto bg-[var(--color-elevated-surface-strong)]"
        />
      ) : (
        <div
          ref={targetViewportRef}
          aria-label={`Tron target ${pageId}`}
          className="block min-h-[560px] flex-auto bg-[var(--color-elevated-surface-strong)]"
        >
          <div className="flex min-h-[560px] flex-auto items-center justify-center text-[var(--color-on-surface-variant)]">
            <p className="text-sm text-[var(--color-on-surface)]">
              目标页面正在当前窗口中加载
            </p>
          </div>
        </div>
      )}
    </section>
  );
});

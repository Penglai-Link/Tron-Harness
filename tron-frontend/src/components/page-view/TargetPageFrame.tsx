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

import { useTabVisibility } from "@/components/editable-tabs";
import {
  DesktopBridge,
  resolveDesktopPageId,
  resolveTargetWebviewPreloadUrl,
  sanitizeInjectionEventPayload,
  sanitizeUrlInput,
  type DesktopInjectionEvent,
  type DesktopInjectionEventRequest,
} from "@/shared/desktop/DesktopBridge";
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
  initialUrl?: string;
  inspectionMode?: boolean;
  onInspectionConfirm?: (result: TargetPageFrameInspectionResult) => void;
  onUrlChange?: (url: string) => void;
  pageId?: string;
  partition?: string;
}

interface TronWebviewElement extends HTMLElement {
  canGoBack: () => boolean;
  canGoForward: () => boolean;
  getURL: () => string;
  goBack: () => void;
  goForward: () => void;
  reload: () => void;
  send: (channel: string, ...args: unknown[]) => Promise<void>;
}

type WebviewNavigationEvent = Event & {
  errorCode?: number;
  isMainFrame?: boolean;
  url?: string;
};

const BRIDGE_CHANNEL = "tron:frame-event";

function createPageId() {
  const suffix =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

  return `desktop-${suffix}`;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
}

function readString(payload: Record<string, unknown> | null, key: string) {
  const value = payload?.[key];
  return typeof value === "string" ? value : "";
}

function displayAddress(url: string) {
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}

function preserveUrlHash(originalUrl: string, resolvedUrl: string) {
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

function safelyReadWebviewNavigation(webview: TronWebviewElement) {
  try {
    return {
      canGoBack: webview.canGoBack(),
      canGoForward: webview.canGoForward(),
      url: webview.getURL(),
    };
  } catch {
    return { canGoBack: false, canGoForward: false, url: "" };
  }
}

export const TargetPageFrame = forwardRef<
  TargetPageFrameHandle,
  TargetPageFrameProps
>(function TargetPageFrame(
  {
    initialUrl = "",
    inspectionMode = false,
    onInspectionConfirm,
    onUrlChange,
    pageId: requestedPageId,
    partition,
  },
  ref,
) {
  const hasInitialUrl = Boolean(initialUrl.trim());
  const bridge = useMemo(() => new DesktopBridge(), []);
  const generatedPageIdRef = useRef(createPageId());
  const pageId = useMemo(
    () => resolveDesktopPageId(requestedPageId || generatedPageIdRef.current),
    [requestedPageId],
  );
  const frameRef = useRef<HTMLIFrameElement | null>(null);
  const emptyViewportRef = useRef<HTMLDivElement | null>(null);
  const addressFormRef = useRef<HTMLFormElement | null>(null);
  const addressInputRef = useRef<HTMLInputElement | null>(null);
  const addressSubmitPendingRef = useRef(false);
  const injectionEnabledRef = useRef(false);
  const webviewReadyRef = useRef(false);
  const [webview, setWebview] = useState<TronWebviewElement | null>(null);
  const [visible, setVisible] = useState(!hasInitialUrl);
  const [activeUrl, setActiveUrl] = useState("");
  const [injectionEnabled, setInjectionEnabled] = useState(false);
  const [frameVersion, setFrameVersion] = useState(0);
  const [historyEntries, setHistoryEntries] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [navigation, setNavigation] = useState({
    canGoBack: false,
    canGoForward: false,
  });
  const [addressDraft, setAddressDraft] = useState("");
  const [addressEditing, setAddressEditing] = useState(!hasInitialUrl);
  const isDesktopHost = bridge.isDesktop();
  const targetPreloadUrl = resolveTargetWebviewPreloadUrl();
  const panelVisible = useTabVisibility();
  const activeDisplayAddress = displayAddress(activeUrl);

  injectionEnabledRef.current = injectionEnabled && panelVisible;

  useEffect(() => {
    if (!inspectionMode) {
      setInjectionEnabled(false);
    }
  }, [inspectionMode]);

  useEffect(() => {
    if (hasInitialUrl) {
      return;
    }

    setVisible(true);
    setAddressEditing(true);
  }, [hasInitialUrl]);

  useEffect(() => {
    if (!visible || activeUrl || !addressEditing) {
      return undefined;
    }

    const frameId = window.requestAnimationFrame(() => {
      addressInputRef.current?.focus();
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [activeUrl, addressEditing, visible]);

  const setCurrentUrl = useCallback(
    (url: string, notify = true) => {
      const normalized = sanitizeUrlInput(url);
      if (
        !normalized.startsWith("http://") &&
        !normalized.startsWith("https://")
      ) {
        return;
      }

      setActiveUrl(normalized);
      setAddressDraft(displayAddress(normalized));
      setAddressEditing(false);
      if (notify) {
        onUrlChange?.(normalized);
      }
    },
    [onUrlChange],
  );

  const checkEmbeddingSupport = useCallback(
    async (url: string) => {
      const result = await bridge.checkTargetPageEmbedding({ url });
      return {
        blocked: result.blocked,
        url: preserveUrlHash(url, result.url || url),
      };
    },
    [bridge],
  );

  const open = useCallback(
    (url: string) => {
      const normalized = sanitizeUrlInput(url);
      setVisible(true);
      setInjectionEnabled(false);
      setNavigation({ canGoBack: false, canGoForward: false });

      if (!normalized) {
        setActiveUrl("");
        setAddressDraft("");
        setAddressEditing(true);
        setHistoryEntries([]);
        setHistoryIndex(0);
        addressInputRef.current?.focus();
        return;
      }

      if (
        !normalized.startsWith("http://") &&
        !normalized.startsWith("https://")
      ) {
        return;
      }

      setActiveUrl(normalized);
      setAddressDraft(normalized);
      setAddressEditing(false);

      if (isDesktopHost) {
        return;
      }

      setHistoryEntries([normalized]);
      setHistoryIndex(0);
      void checkEmbeddingSupport(normalized).then((result) => {
        if (!result.blocked) {
          setActiveUrl(result.url);
          setHistoryEntries([result.url]);
          setFrameVersion((current) => current + 1);
        }
      });
    },
    [checkEmbeddingSupport, isDesktopHost],
  );

  useImperativeHandle(ref, () => ({ open }), [open]);

  const handleWebviewRef = useCallback((element: HTMLElement | null) => {
    webviewReadyRef.current = false;
    setWebview(element as TronWebviewElement | null);
  }, []);

  const syncWebviewInspection = useCallback(
    (configure = false) => {
      if (!webview || !webviewReadyRef.current) {
        return;
      }

      const channel = configure
        ? "tron-desktop:configure-target"
        : "tron-desktop:toggle-injection";
      try {
        void webview
          .send(channel, {
            enabled: injectionEnabledRef.current,
            pageId,
          })
          .catch(() => undefined);
      } catch {
        webviewReadyRef.current = false;
      }
    },
    [pageId, webview],
  );

  useEffect(() => {
    if (!webview) {
      return undefined;
    }

    const syncNavigation = (event?: WebviewNavigationEvent) => {
      if (event?.isMainFrame === false) {
        return;
      }
      const current = safelyReadWebviewNavigation(webview);
      const nextUrl = sanitizeUrlInput(event?.url || current.url);
      setNavigation({
        canGoBack: current.canGoBack,
        canGoForward: current.canGoForward,
      });
      if (nextUrl) {
        setCurrentUrl(nextUrl);
      }
    };
    const handleDomReady = () => {
      webviewReadyRef.current = true;
      syncNavigation();
      syncWebviewInspection(true);
    };
    const handleDidStartLoading = () => {
      webviewReadyRef.current = false;
    };
    const handleDidFailLoad = (event: WebviewNavigationEvent) => {
      if (event.isMainFrame === false || event.errorCode === -3) {
        return;
      }
    };

    webview.addEventListener("dom-ready", handleDomReady);
    webview.addEventListener("did-start-loading", handleDidStartLoading);
    webview.addEventListener("did-navigate", syncNavigation as EventListener);
    webview.addEventListener(
      "did-navigate-in-page",
      syncNavigation as EventListener,
    );
    webview.addEventListener(
      "did-fail-load",
      handleDidFailLoad as EventListener,
    );

    return () => {
      webview.removeEventListener("dom-ready", handleDomReady);
      webview.removeEventListener("did-start-loading", handleDidStartLoading);
      webview.removeEventListener(
        "did-navigate",
        syncNavigation as EventListener,
      );
      webview.removeEventListener(
        "did-navigate-in-page",
        syncNavigation as EventListener,
      );
      webview.removeEventListener(
        "did-fail-load",
        handleDidFailLoad as EventListener,
      );
    };
  }, [setCurrentUrl, syncWebviewInspection, webview]);

  useEffect(() => {
    syncWebviewInspection(false);
  }, [injectionEnabled, panelVisible, syncWebviewInspection]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const sanitized = sanitizeInjectionEventPayload(
        event.data as DesktopInjectionEventRequest,
      );
      if (!sanitized || sanitized.pageId !== pageId) {
        return;
      }

      if (sanitized.eventType === "tron:inspection-exited") {
        setInjectionEnabled(false);
        return;
      }

      if (sanitized.eventType === "tron:navigation-requested") {
        const requestedUrl = sanitizeUrlInput(
          readString(asRecord(sanitized.payload), "url"),
        );
        if (!isDesktopHost && requestedUrl) {
          setCurrentUrl(requestedUrl);
        }
        return;
      }

      if (inspectionMode && sanitized.eventType === "tron:node-inspected") {
        const dom = asRecord(sanitized.payload) ?? {};
        onInspectionConfirm?.({
          dom,
          event: sanitized,
          remark: readString(dom, "remark"),
        });
      }
    };

    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [
    inspectionMode,
    isDesktopHost,
    onInspectionConfirm,
    pageId,
    setCurrentUrl,
  ]);

  const openInSystemBrowser = useCallback(() => {
    if (!activeUrl) {
      return;
    }
    void bridge
      .openTargetPage({ mode: "browser", pageId, url: activeUrl })
      .then((result) => {
        if (!result.opened && result.reason === "running-in-browser") {
          window.open(activeUrl, "_blank", "noopener,noreferrer");
        }
      });
  }, [activeUrl, bridge, pageId]);

  const handleBack = () => {
    if (isDesktopHost) {
      if (webview?.canGoBack()) webview.goBack();
      return;
    }
    if (historyIndex <= 0) return;
    const nextIndex = historyIndex - 1;
    setHistoryIndex(nextIndex);
    setActiveUrl(historyEntries[nextIndex] ?? "");
  };

  const handleForward = () => {
    if (isDesktopHost) {
      if (webview?.canGoForward()) webview.goForward();
      return;
    }
    if (historyIndex >= historyEntries.length - 1) return;
    const nextIndex = historyIndex + 1;
    setHistoryIndex(nextIndex);
    setActiveUrl(historyEntries[nextIndex] ?? "");
  };

  const handleRefresh = () => {
    if (isDesktopHost) {
      webview?.reload();
      return;
    }
    setFrameVersion((current) => current + 1);
  };

  const submitAddress = (value = addressDraft) => {
    const normalized = sanitizeUrlInput(value);
    if (
      !normalized.startsWith("http://") &&
      !normalized.startsWith("https://")
    ) {
      setAddressDraft(activeUrl);
      setAddressEditing(false);
      return;
    }

    setInjectionEnabled(false);
    setCurrentUrl(normalized);
    if (!isDesktopHost) {
      const nextIndex = historyIndex + 1;
      setHistoryEntries((current) => [
        ...current.slice(0, nextIndex),
        normalized,
      ]);
      setHistoryIndex(nextIndex);
      void checkEmbeddingSupport(normalized).then((result) => {
        if (!result.blocked) {
          setCurrentUrl(result.url);
          setFrameVersion((current) => current + 1);
        }
      });
    }
    addressInputRef.current?.blur();
  };

  const handleAddressBlur = (event: FocusEvent<HTMLFormElement>) => {
    if (addressSubmitPendingRef.current) return;
    const nextTarget = event.relatedTarget;
    if (
      nextTarget instanceof Node &&
      addressFormRef.current?.contains(nextTarget)
    )
      return;
    window.setTimeout(() => {
      if (!addressFormRef.current?.contains(document.activeElement)) {
        setAddressDraft(activeDisplayAddress);
        setAddressEditing(false);
      }
    }, 0);
  };

  const handleAddressKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      submitAddress(event.currentTarget.value);
    } else if (event.key === "Escape") {
      setAddressDraft(activeDisplayAddress);
      setAddressEditing(false);
    }
  };

  if (!visible) {
    return null;
  }

  return (
    <section className="overflow-hidden h-full flex flex-col">
      <TargetPageChrome
        addressEditing={addressEditing}
        addressFormRef={addressFormRef}
        addressInputRef={addressInputRef}
        addressValue={addressEditing ? addressDraft : activeDisplayAddress}
        canGoBack={isDesktopHost ? navigation.canGoBack : historyIndex > 0}
        canGoForward={
          isDesktopHost
            ? navigation.canGoForward
            : historyIndex < historyEntries.length - 1
        }
        inspectionMode={inspectionMode}
        injectionEnabled={injectionEnabled}
        isDesktopHost={isDesktopHost}
        onAddressBlur={handleAddressBlur}
        onAddressChange={setAddressDraft}
        onAddressFocus={() => {
          setAddressDraft(activeUrl);
          setAddressEditing(true);
        }}
        onAddressKeyDown={handleAddressKeyDown}
        onAddressSubmit={(event) => {
          event.preventDefault();
          const value = new FormData(event.currentTarget).get("targetAddress");
          addressSubmitPendingRef.current = false;
          submitAddress(typeof value === "string" ? value : addressDraft);
        }}
        onBack={handleBack}
        onForward={handleForward}
        onInjectionToggle={() => {
          if (inspectionMode) setInjectionEnabled((current) => !current);
        }}
        onRefresh={handleRefresh}
        onSubmitPointerDown={() => {
          addressSubmitPendingRef.current = true;
          window.setTimeout(() => {
            addressSubmitPendingRef.current = false;
          }, 0);
        }}
      />
      {activeUrl ? (
        isDesktopHost && targetPreloadUrl ? (
          <webview
            key={pageId}
            ref={handleWebviewRef}
            style={{ display: "inline-flex", height: "100%" }}
            partition={partition}
            preload={targetPreloadUrl}
            src={activeUrl}
          />
        ) : (
          <iframe
            key={`${pageId}-${frameVersion}-${historyIndex}`}
            ref={frameRef}
            className="block min-h-[560px] w-full flex-auto border-0 bg-[var(--color-elevated-surface-strong)]"
            name={pageId}
            referrerPolicy="no-referrer"
            sandbox="allow-forms allow-modals allow-pointer-lock allow-same-origin allow-scripts allow-top-navigation-by-user-activation"
            src={activeUrl}
            title={`Tron target ${pageId}`}
            onLoad={(e) => {
              frameRef.current?.contentWindow?.postMessage(
                {
                  channel: BRIDGE_CHANNEL,
                  enabled: injectionEnabled,
                  pageId,
                  type: "tron:toggle-injection",
                },
                "*",
              );
            }}
          />
        )
      ) : (
        <TargetPageEmpty targetViewportRef={emptyViewportRef} />
      )}
    </section>
  );
});

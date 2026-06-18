import { useEffect, useMemo, useState } from "react";

import { Button } from "@/shared";
import {
  DesktopBridge,
  type DesktopBackendStatus,
  sanitizeInjectionEventPayload,
  type DesktopInjectionEvent,
  type DesktopInjectionEventRequest,
} from "@/shared/desktop/DesktopBridge";

import { TargetPageFrame } from "../components/TargetPageFrame";

const DEFAULT_TARGET_URL = "https://example.com";

export function TargetPagePage() {
  const bridge = useMemo(() => new DesktopBridge(), []);
  const [activeUrl, setActiveUrl] = useState(DEFAULT_TARGET_URL);
  const [pageId, setPageId] = useState(`desktop-${Date.now()}`);
  const [injectionEnabled, setInjectionEnabled] = useState(false);
  const [frameVisible, setFrameVisible] = useState(false);
  const [backendStatus, setBackendStatus] = useState<DesktopBackendStatus>({
    backendUrl: "",
    backendPort: 0,
    ready: false,
    injectionEnabled: false,
    databasePath: "",
    graphPath: "",
  });
  const [events, setEvents] = useState<DesktopInjectionEvent[]>([]);
  const [errorMessage, setErrorMessage] = useState<string>("");
  const [frameReady, setFrameReady] = useState(false);

  useEffect(() => {
    let isMounted = true;

    void (async () => {
      const status = await bridge.getBackendStatus();
      if (isMounted) {
        setBackendStatus(status);
      }
    })();

    return () => {
      isMounted = false;
    };
  }, [bridge]);

  useEffect(() => {
    void (async () => {
      const enabled = await bridge.setInjectionEnabled(injectionEnabled);
      setBackendStatus((current) => ({
        ...current,
        injectionEnabled: enabled,
      }));
    })();
  }, [bridge, injectionEnabled]);

  useEffect(() => {
    if (!injectionEnabled || !frameVisible) {
      setFrameReady(false);
    }
  }, [frameVisible, injectionEnabled]);

  const handleLoadTarget = () => {
    setErrorMessage("");
    setPageId(`desktop-${Date.now()}`);
    setActiveUrl(DEFAULT_TARGET_URL);
    setInjectionEnabled(false);
    setFrameVisible(true);
    setFrameReady(false);
  };

  const handleFallback = async (reason: string) => {
    setErrorMessage(`Target page likely blocked (${reason}), open with separate window.`);
    const result = await bridge.openTargetPage({
      url: activeUrl,
      mode: "window",
      pageId,
    });

    if (!result.opened) {
      setErrorMessage(`Fallback launch skipped: ${result.reason ?? "not available"}`);
    }
  };

  const handleIncomingEvent = (raw: unknown) => {
    const sanitized = sanitizeInjectionEventPayload(raw as DesktopInjectionEventRequest);
    if (!sanitized) {
      return;
    }

    void bridge.emitInjectionEvent(sanitized);
    setEvents((current) => [
      {
        ...sanitized,
      },
      ...current,
    ].slice(0, 20));
  };

  return (
    <main className="min-h-screen bg-[var(--color-background)] px-6 py-8 text-[var(--color-on-surface)]">
      <section className="mx-auto flex w-full max-w-[1200px] flex-col gap-4">
        <div className="flex items-center justify-end">
          <Button onClick={handleLoadTarget}>Open target</Button>
        </div>
        {errorMessage ? <p className="text-sm text-[var(--color-error)]">{errorMessage}</p> : null}
        {frameVisible ? (
          <section>
            <TargetPageFrame
              pageId={pageId}
              targetUrl={activeUrl}
              desktopBackendStatus={backendStatus}
              injectionEnabled={injectionEnabled}
              onInjectionEnabledChange={setInjectionEnabled}
              onTargetUrlChange={setActiveUrl}
              onHandshake={() => {
                setFrameReady(true);
                setErrorMessage("");
              }}
              onFrameBlocked={handleFallback}
              onInjectionEvent={handleIncomingEvent}
            />
          </section>
        ) : null}
        <section className="sr-only">
          <p>Frame handshake: {frameReady ? "ready" : "waiting"}</p>
          <p>Injection event count: {events.length}</p>
        </section>
      </section>
    </main>
  );
}

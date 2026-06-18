import { useEffect, useMemo, useState } from "react";

import { Button, SurfaceCard, TextInput, ToggleSwitch } from "@/shared";
import {
  DesktopBridge,
  type DesktopBackendStatus,
  sanitizeInjectionEventPayload,
  type DesktopInjectionEvent,
  type DesktopInjectionEventRequest,
  sanitizeUrlInput,
} from "@/shared/desktop/DesktopBridge";

import { TargetPageFrame } from "../components/TargetPageFrame";

const DEFAULT_TARGET_URL = "https://example.com";

export function TargetPagePage() {
  const bridge = useMemo(() => new DesktopBridge(), []);
  const [targetInput, setTargetInput] = useState(DEFAULT_TARGET_URL);
  const [activeUrl, setActiveUrl] = useState(DEFAULT_TARGET_URL);
  const [pageId, setPageId] = useState(`desktop-${Date.now()}`);
  const [injectionEnabled, setInjectionEnabled] = useState(true);
  const [backendStatus, setBackendStatus] = useState<DesktopBackendStatus>({
    backendUrl: "",
    backendPort: 0,
    ready: false,
    injectionEnabled: true,
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
    void bridge.setInjectionEnabled(injectionEnabled);
  }, [bridge, injectionEnabled]);

  const handleLoadTarget = () => {
    const normalized = sanitizeUrlInput(targetInput);
    if (!normalized.startsWith("http://") && !normalized.startsWith("https://")) {
      setErrorMessage("Please enter an http/https URL.");
      return;
    }

    setErrorMessage("");
    setPageId(`desktop-${Date.now()}`);
    setActiveUrl(normalized);
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
        <header className="space-y-1">
          <h1 className="text-2xl font-semibold text-[var(--color-on-surface)]">Desktop Target Page</h1>
          <p className="text-sm text-[var(--color-on-surface-variant)]">Inject into iframe frames and auto fallback to standalone view when blocked.</p>
        </header>
        <SurfaceCard>
          <div className="grid gap-4 md:grid-cols-[1fr_auto_auto] md:items-end">
            <div className="space-y-2">
              <label className="text-xs font-medium text-[var(--color-on-surface-variant)]" htmlFor="desktop-target-url">
                Target URL
              </label>
              <TextInput
                id="desktop-target-url"
                value={targetInput}
                onChange={(event) => setTargetInput(event.target.value)}
                placeholder="https://example.com"
                spellCheck={false}
              />
            </div>
            <Button onClick={handleLoadTarget}>Open target</Button>
            <ToggleSwitch
              ariaLabel="Enable DOM bridge injection"
              checked={injectionEnabled}
              label="Inject"
              onCheckedChange={setInjectionEnabled}
            />
          </div>
          <div className="mt-3 space-y-1 text-xs text-[var(--color-on-surface-variant)]">
            <p>
              Backend: <span className="font-mono">{backendStatus.backendUrl || "not ready"}</span>
            </p>
            <p>Injection enabled: {backendStatus.injectionEnabled ? "yes" : "no"}</p>
          </div>
          {errorMessage ? <p className="mt-2 text-sm text-[var(--color-error)]">{errorMessage}</p> : null}
        </SurfaceCard>
        <section>
          <TargetPageFrame
            pageId={pageId}
            targetUrl={activeUrl}
            desktopBackendStatus={backendStatus}
            injectionEnabled={injectionEnabled && backendStatus.ready}
            onHandshake={() => {
              setFrameReady(true);
              setErrorMessage("");
            }}
            onFrameBlocked={handleFallback}
            onInjectionEvent={handleIncomingEvent}
          />
        </section>
        <section className="grid gap-2 md:grid-cols-2">
          <SurfaceCard>
            <h2 className="mb-2 text-sm font-semibold text-[var(--color-on-surface)]">Frame status</h2>
            <p className="text-xs text-[var(--color-on-surface-variant)]">
              Frame handshake: {frameReady ? "ready" : "waiting"}
            </p>
          </SurfaceCard>
          <SurfaceCard className="max-h-[220px] overflow-auto">
            <h2 className="mb-2 text-sm font-semibold text-[var(--color-on-surface)]">Injection events</h2>
            {events.length === 0 ? (
              <p className="text-xs text-[var(--color-on-surface-variant)]">No events yet.</p>
            ) : (
              <ul className="space-y-2">
                {events.map((event, index) => (
                  <li className="border-t border-[var(--color-outline-variant)] pt-2 text-xs text-[var(--color-on-surface-variant)]" key={`${event.frameId}-${event.timestamp}-${index}`}>
                    <p className="font-mono">{event.eventType}</p>
                    <p className="mt-1">page: {event.pageId} frame: {event.frameId}</p>
                    <p className="mt-1 break-all">{JSON.stringify(event.payload ?? {})}</p>
                  </li>
                ))}
              </ul>
            )}
          </SurfaceCard>
        </section>
      </section>
    </main>
  );
}


/** Share long-lived SSE connections across tabs when SharedWorker is available. */
export function subscribeSharedEvent(url: string, eventName: string, onChange: () => void): () => void {
  if (typeof SharedWorker !== "undefined") {
    try {
      const worker = new SharedWorker(new URL("./eventStream.worker.ts", import.meta.url), { type: "module", name: "tron-event-streams" });
      const port = worker.port;
      port.onmessage = (event: MessageEvent<{ type: string }>) => {
        if (event.data.type === "event") onChange();
      };
      port.start();
      port.postMessage({ type: "subscribe", url: new URL(url, window.location.href).href, eventName });
      let closed = false;
      const close = () => {
        if (closed) return;
        closed = true;
        port.postMessage({ type: "unsubscribe" });
        port.close();
        window.removeEventListener("pagehide", onPageHide);
      };
      const onPageHide = (event: PageTransitionEvent) => {
        if (!event.persisted) close();
      };
      window.addEventListener("pagehide", onPageHide);
      return close;
    } catch {
      // Fall back for browsers that expose SharedWorker but cannot instantiate it.
    }
  }
  const source = new EventSource(url, { withCredentials: true });
  source.addEventListener(eventName, onChange);
  return () => source.close();
}

export {};

// Share persistent event streams between pages of the same browser profile.
type Subscription = { source: EventSource; ports: Set<MessagePort> };
const streams = new Map<string, Subscription>();

self.addEventListener("connect", ((event: MessageEvent) => {
  const port = event.ports[0];
  let key: string | undefined;
  const unsubscribe = () => {
    if (!key) return;
    const stream = streams.get(key);
    stream?.ports.delete(port);
    if (stream && stream.ports.size === 0) {
      stream.source.close();
      streams.delete(key);
    }
    key = undefined;
  };
  port.onmessage = (message: MessageEvent<{ type: "subscribe" | "unsubscribe"; url: string; eventName: string }>) => {
    if (message.data.type === "unsubscribe") { unsubscribe(); return; }
    unsubscribe();
    const { url, eventName } = message.data;
    key = JSON.stringify([url, eventName]);
    let stream = streams.get(key);
    if (!stream) {
      const ports = new Set<MessagePort>();
      const source = new EventSource(url, { withCredentials: true });
      source.addEventListener(eventName, () => {
        for (const listener of ports) listener.postMessage({ type: "event" });
      });
      stream = { source, ports };
      streams.set(key, stream);
    }
    stream.ports.add(port);
  };
  port.start();
}) as EventListener);

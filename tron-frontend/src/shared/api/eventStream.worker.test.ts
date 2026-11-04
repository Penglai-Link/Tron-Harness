import { afterEach, expect, it, vi } from "vitest";

afterEach(() => { vi.unstubAllGlobals(); vi.resetModules(); });

it("shares an event stream and closes it after the last page unsubscribes", async () => {
  let connect: (event: { ports: FakePort[] }) => void = () => {};
  const sources: FakeEventSource[] = [];
  class FakeEventSource {
    listeners = new Map<string, () => void>();
    close = vi.fn();
    constructor(public url: string) { sources.push(this); }
    addEventListener(name: string, callback: () => void) { this.listeners.set(name, callback); }
  }
  class FakePort {
    onmessage: ((event: { data: unknown }) => void) | null = null;
    postMessage = vi.fn();
    start = vi.fn();
    send(data: unknown) { this.onmessage?.({ data }); }
  }
  vi.stubGlobal("self", { addEventListener: (_type: string, callback: typeof connect) => { connect = callback; } });
  vi.stubGlobal("EventSource", FakeEventSource);
  await import("./eventStream.worker");
  const first = new FakePort();
  const second = new FakePort();
  connect({ ports: [first] });
  connect({ ports: [second] });
  const subscription = { type: "subscribe", url: "/events", eventName: "changed" };
  first.send(subscription);
  second.send(subscription);
  expect(sources).toHaveLength(1);
  sources[0].listeners.get("changed")?.();
  expect(first.postMessage).toHaveBeenCalledOnce();
  expect(second.postMessage).toHaveBeenCalledOnce();
  first.send({ type: "unsubscribe" });
  expect(sources[0].close).not.toHaveBeenCalled();
  second.send({ type: "unsubscribe" });
  expect(sources[0].close).toHaveBeenCalledOnce();
});

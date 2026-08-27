// @vitest-environment jsdom

import type { IDisposable, Terminal as XTerminal } from "@xterm/xterm";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ModelSessionAddon } from "./modelSession";

let animationFrames: FrameRequestCallback[];

function resizeEntry(target: HTMLElement, width: number) {
  return { contentRect: { width }, target } as unknown as ResizeObserverEntry;
}

class TerminalStub {
  readonly buffer = { active: { baseY: 0, cursorY: 0, viewportY: 0 } };
  readonly element = document.createElement("div");
  readonly markers: Array<{ dispose: ReturnType<typeof vi.fn>; line: number }> = [];
  private resizeHandler?: () => void;

  onScroll(): IDisposable {
    return { dispose: vi.fn() };
  }

  onResize(handler: () => void): IDisposable {
    this.resizeHandler = handler;
    return { dispose: vi.fn() };
  }

  registerMarker(cursorYOffset = 0) {
    const marker = { dispose: vi.fn(), line: cursorYOffset };
    this.markers.push(marker);
    return marker;
  }

  emitResize() {
    this.resizeHandler?.();
  }

  write = vi.fn((_data: string, callback?: () => void) => callback?.());
}

describe("ModelSessionAddon card resize tracking", () => {
  let observerCallback: ResizeObserverCallback;
  const observe = vi.fn();

  beforeEach(() => {
    observe.mockClear();
    animationFrames = [];
    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: ResizeObserverCallback) {
          observerCallback = callback;
        }

        observe = observe;
        unobserve = vi.fn();
        disconnect = vi.fn();
      },
    );
    vi.stubGlobal(
      "requestAnimationFrame",
      vi.fn((callback: FrameRequestCallback) => {
        animationFrames.push(callback);
        return animationFrames.length;
      }),
    );
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("refreshes the display when any observed card changes width", () => {
    const onLayoutResize = vi.fn();
    const addon = new ModelSessionAddon({ onLayoutResize });
    const terminal = new TerminalStub();
    addon.activate(terminal as unknown as XTerminal);

    addon.appendMessageChunk({ conversationId: "one", requestId: "one", response: "one" });
    addon.appendMessageChunk({ conversationId: "two", requestId: "two", response: "two" });

    const cards = [...terminal.element.querySelectorAll<HTMLElement>(".ai-terminal-model-session-host")];
    expect(observe).toHaveBeenCalledTimes(2);

    observerCallback([resizeEntry(cards[0], 420)], {} as ResizeObserver);
    observerCallback([resizeEntry(cards[1], 640)], {} as ResizeObserver);

    expect(onLayoutResize).toHaveBeenCalledTimes(2);
    addon.dispose();
  });

  it("ignores detail toggles while the session is running", () => {
    const onLayoutResize = vi.fn();
    const addon = new ModelSessionAddon({ onLayoutResize });
    const terminal = new TerminalStub();
    addon.activate(terminal as unknown as XTerminal);
    addon.appendMessageChunk({ conversationId: "one", requestId: "one", response: "one" });

    const host = terminal.element.querySelector<HTMLElement>(".ai-terminal-model-session-host");
    const detail = document.createElement("details");
    host?.appendChild(detail);
    detail.dispatchEvent(new Event("toggle"));

    expect(onLayoutResize).not.toHaveBeenCalled();
    addon.dispose();
  });

  it("anchors a streaming card only after the command output has finished", () => {
    const addon = new ModelSessionAddon();
    const terminal = new TerminalStub();
    addon.activate(terminal as unknown as XTerminal);
    addon.appendMessageChunk({
      conversationId: "one",
      requestId: "one",
      response: "one",
    });

    expect(terminal.markers).toHaveLength(0);

    addon.anchorPendingSessions();
    expect(terminal.markers).toHaveLength(1);
    addon.dispose();
  });

  it("shows a prepared session immediately below the submitted natural-language line", () => {
    const addon = new ModelSessionAddon();
    const terminal = new TerminalStub();
    addon.activate(terminal as unknown as XTerminal);

    addon.prepareNextSessionAnchor();
    addon.startSession({ conversationId: "one", requestId: "one" });
    flushAnimationFrames();

    const host = terminal.element.querySelector<HTMLElement>(
      ".ai-terminal-model-session-host",
    );
    expect(host?.hidden).toBe(false);
    expect(terminal.markers).toHaveLength(1);
    expect(terminal.markers[0].line).toBe(1);
    expect(terminal.write).toHaveBeenCalledWith("\r\n", expect.any(Function));
    addon.dispose();
  });

  it("reserves additional terminal rows as a streaming card grows", () => {
    const addon = new ModelSessionAddon();
    const terminal = new TerminalStub();
    addon.activate(terminal as unknown as XTerminal);
    addon.appendMessageChunk({
      conversationId: "one",
      requestId: "one",
      response: "one",
    });
    addon.anchorPendingSessions();
    flushAnimationFrames();

    const host = terminal.element.querySelector<HTMLElement>(
      ".ai-terminal-model-session-host",
    );
    const card = host?.firstElementChild as HTMLElement;
    Object.defineProperty(card, "scrollHeight", { configurable: true, value: 72 });

    addon.appendMessageChunk({
      conversationId: "one",
      requestId: "one",
      response: " more",
    });
    flushAnimationFrames();

    expect(terminal.markers).toHaveLength(1);
    expect(terminal.write).toHaveBeenLastCalledWith(
      "\r\n".repeat(3),
      expect.any(Function),
    );
    addon.dispose();
  });

  it("does not complete until every reserved terminal row has been written", async () => {
    const addon = new ModelSessionAddon();
    const terminal = new TerminalStub();
    let releaseWrite: (() => void) | undefined;
    terminal.write.mockImplementation((_data, callback) => {
      releaseWrite = callback;
    });
    addon.activate(terminal as unknown as XTerminal);
    addon.prepareNextSessionAnchor();
    addon.startSession({ conversationId: "one", requestId: "one" });
    flushAnimationFrames();

    let completed = false;
    const completion = addon.complete("one").then(() => {
      completed = true;
    });
    await Promise.resolve();

    expect(completed).toBe(false);
    releaseWrite?.();
    await completion;
    expect(completed).toBe(true);
    addon.dispose();
  });

  it("repositions a card from its reflowed marker line after terminal resize", () => {
    const addon = new ModelSessionAddon();
    const terminal = new TerminalStub();
    addon.activate(terminal as unknown as XTerminal);
    addon.appendMessageChunk({
      conversationId: "one",
      requestId: "one",
      response: "one",
    });
    addon.anchorPendingSessions();
    flushAnimationFrames();

    terminal.markers[0].line = 4;
    terminal.emitResize();
    flushAnimationFrames();

    const host = terminal.element.querySelector<HTMLElement>(
      ".ai-terminal-model-session-host",
    );
    expect(host?.style.transform).toBe("translate3d(0, 72px, 0)");

    terminal.markers[0].line = 6;
    terminal.emitResize();
    flushAnimationFrames();
    expect(host?.style.transform).toBe("translate3d(0, 108px, 0)");

    addon.dispose();
    expect(terminal.markers[0].dispose).toHaveBeenCalledOnce();
  });
});

function flushAnimationFrames() {
  while (animationFrames.length > 0) {
    animationFrames.shift()?.(performance.now());
  }
}

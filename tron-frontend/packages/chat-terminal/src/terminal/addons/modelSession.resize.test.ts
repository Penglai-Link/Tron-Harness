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

  registerMarker() {
    const marker = { dispose: vi.fn(), line: 0 };
    this.markers.push(marker);
    return marker;
  }

  emitResize() {
    this.resizeHandler?.();
  }

  write() {}
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

  it("repositions a card from its reflowed marker line after terminal resize", () => {
    const addon = new ModelSessionAddon();
    const terminal = new TerminalStub();
    addon.activate(terminal as unknown as XTerminal);
    addon.appendMessageChunk({
      conversationId: "one",
      requestId: "one",
      response: "one",
    });
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

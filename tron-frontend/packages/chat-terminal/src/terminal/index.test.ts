// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const fit = vi.fn();
const refresh = vi.fn();

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit = fit;
  },
}));

vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    cols = 80;
    rows = 24;
    element?: HTMLElement;
    options = {};

    loadAddon() {}

    refresh = refresh;

    open(container: HTMLElement) {
      this.element = document.createElement("div");
      container.appendChild(this.element);
    }

    dispose() {}
  },
}));

vi.mock("./utils/themes", () => ({
  applyTerminalThemeTokens: vi.fn(),
  clearTerminalThemeTokens: vi.fn(),
  syncTerminalTheme: vi.fn(),
}));

import { Terminal } from "./index";

describe("Terminal resize tracking", () => {
  let observerCallback: ResizeObserverCallback;
  let animationFrames: FrameRequestCallback[];
  const disconnect = vi.fn();
  const observe = vi.fn();

  beforeEach(() => {
    fit.mockReset();
    refresh.mockReset();
    disconnect.mockClear();
    observe.mockClear();

    vi.stubGlobal(
      "ResizeObserver",
      class {
        constructor(callback: ResizeObserverCallback) {
          observerCallback = callback;
        }

        disconnect = disconnect;
        observe = observe;
      },
    );
    animationFrames = [];
    vi.stubGlobal("requestAnimationFrame", vi.fn((callback: FrameRequestCallback) => {
      animationFrames.push(callback);
      return animationFrames.length;
    }));
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.replaceChildren();
  });

  it("observes both the host and the FitAddon measurement container", () => {
    const host = document.createElement("div");
    document.body.appendChild(host);

    const terminal = new Terminal(host);
    const viewport = host.querySelector(".ai-terminal-viewport");

    expect(viewport).toBeInstanceOf(HTMLElement);
    expect(observe).toHaveBeenNthCalledWith(1, host);
    expect(observe).toHaveBeenNthCalledWith(2, viewport);

    observerCallback([], {} as ResizeObserver);
    terminal.dispose();

    expect(disconnect).toHaveBeenCalledOnce();
  });

  it("suppresses transport resize notifications for inline card layout refreshes", () => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const terminal = new Terminal(host);
    const viewport = host.querySelector<HTMLElement>(".ai-terminal-viewport");

    expect(viewport).toBeInstanceOf(HTMLElement);
    vi.spyOn(viewport!, "getBoundingClientRect").mockReturnValue({
      width: 640,
      height: 320,
    } as DOMRect);

    animationFrames.splice(0).forEach((callback) => callback(0));
    fit.mockClear();
    fit.mockImplementation(() => {
      expect(terminal.shouldNotifyTransportResize()).toBe(false);
    });

    terminal.refreshForContentLayout();
    animationFrames.shift()?.(0);

    expect(fit).toHaveBeenCalledOnce();
    expect(refresh).toHaveBeenCalledWith(0, 23);
  });
});

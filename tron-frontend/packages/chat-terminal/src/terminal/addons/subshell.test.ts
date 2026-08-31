// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SubshellBootstrapAddon } from "./subshell";

describe("SubshellBootstrapAddon", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("automatically bootstraps a detected interactive shell", () => {
    const write = vi.fn();
    const terminal = { clear: vi.fn() };
    const addon = new SubshellBootstrapAddon({
      write,
      getBootstrap: () => "eval bootstrap",
      isNestedShellActive: () => true,
    });
    addon.activate(terminal as never);

    addon.handleDetection({ command: "ssh host", host: "host" });

    expect(write).toHaveBeenCalledWith(
      "\x15\x0b eval bootstrap && printf '\\033[H\\033[2J'\r",
    );
    vi.advanceTimersByTime(150);
    expect(terminal.clear).toHaveBeenCalledOnce();
  });

  it("does not write after the nested shell has exited", () => {
    const write = vi.fn();
    const addon = new SubshellBootstrapAddon({
      write,
      getBootstrap: () => "eval bootstrap",
      isNestedShellActive: () => false,
    });
    addon.activate({ clear: vi.fn() } as never);

    addon.handleDetection({ command: "ssh host", host: "host" });

    expect(write).not.toHaveBeenCalled();
  });
});

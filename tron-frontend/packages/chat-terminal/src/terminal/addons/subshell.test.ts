// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";

import { SubshellBootstrapAddon } from "./subshell";

describe("SubshellBootstrapAddon", () => {
  it("automatically bootstraps a detected interactive shell", () => {
    const write = vi.fn();
    const addon = new SubshellBootstrapAddon({
      write,
      getBootstrap: () => "eval bootstrap",
      isNestedShellActive: () => true,
    });
    addon.activate({} as never);

    addon.handleDetection({ command: "ssh host", host: "host" });

    expect(write).toHaveBeenCalledWith(
      "\x15\x0b eval bootstrap && printf '\\033]133;A\\007\\033]133;B\\007\\033[H\\033[2J\\033[3J'\r",
    );
  });

  it("defers rendering while bootstrapping a Docker Bash shell", () => {
    const write = vi.fn();
    const terminal = {
      write: vi.fn((_data: string, callback: () => void) => callback()),
    };
    const addon = new SubshellBootstrapAddon({
      write,
      getBootstrap: () => "eval bootstrap",
      isNestedShellActive: () => true,
    });
    addon.activate(terminal as never);

    addon.handleDetection({ command: "docker exec -it api bash", host: "api" });

    expect(terminal.write).toHaveBeenCalledWith("\x1b[?2026h", expect.any(Function));
    expect(write).toHaveBeenCalledWith(
      "\x15\x0b __TRON_SYNCED_BOOTSTRAP=1; if eval bootstrap; then printf '\\033]133;A\\007\\033]133;B\\007\\033[H\\033[2J\\033[3J'; else printf '\\033[?2026l'; fi\r",
    );
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

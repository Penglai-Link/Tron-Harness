import type { ITerminalAddon, Terminal as XTerminal } from "@xterm/xterm";

import type { ChatTerminalI18n } from "../i18n";
import type { SubshellDetection } from "../types/shellIntegration";

export interface SubshellAddonOptions {
  /** @deprecated Automatic bootstrap no longer shows a consent prompt. */
  i18n?: ChatTerminalI18n;
  /** Writes to the PTY, which at this point is the nested shell. */
  write: (data: string) => void;
  /** Kept for API compatibility; recursive helpers install the bootstrap. */
  getBootstrap: () => string;
  /** Whether the nested shell still owns the PTY. */
  isNestedShellActive?: () => boolean;
}

/** Readline: kill to start of line, then kill to end of line. */
const CLEAR_LINE = "\x15\x0b";

/**
 * Tracks nested-shell detection while the shell helper installs integration.
 *
 * Bootstrap is automatic. If a nested shell cannot be wrapped, it remains a
 * plain terminal and prompt-aware AI stays disabled by the input router.
 */
export class SubshellBootstrapAddon implements ITerminalAddon {
  private terminal?: XTerminal;

  constructor(private readonly options: SubshellAddonOptions) {}

  activate(terminal: XTerminal) {
    this.terminal = terminal;
  }

  dispose() {
    this.terminal = undefined;
  }

  refreshLocale() {}

  hide() {}

  handleDetection(_detection: SubshellDetection) {
    // Detection is the point at which the nested process is known to be an
    // interactive shell. Install integration immediately; there is no consent
    // prompt, and AI remains disabled until the shell reports OSC 133 again.
    this.install();
  }

  private install() {
    const bootstrap = this.options.getBootstrap();
    if (!bootstrap || this.options.isNestedShellActive?.() === false) {
      return;
    }

    // Keep the injected command out of history and clear it from the visible
    // terminal once the shell has evaluated it.
    this.options.write(
      `${CLEAR_LINE} ${bootstrap} && printf '\\033[H\\033[2J'\r`,
    );
    setTimeout(() => {
      this.terminal?.clear();
    }, 150);
  }
}

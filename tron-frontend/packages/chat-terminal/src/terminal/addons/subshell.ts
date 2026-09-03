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
/** DEC private mode: defer rendering until the nested prompt is ready. */
const SYNCHRONIZED_OUTPUT_BEGIN = "\x1b[?2026h";
const SYNCHRONIZED_OUTPUT_END = "\\033[?2026l";
const SYNCHRONIZED_SHELLS = new Set(["bash", "zsh"]);

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

  handleDetection(detection: SubshellDetection) {
    // Detection is the point at which the nested process is known to be an
    // interactive shell. Install integration immediately; there is no consent
    // prompt, and AI remains disabled until the shell reports OSC 133 again.
    this.install(canSynchronizeBootstrap(detection.command));
  }

  private install(synchronizeRender: boolean) {
    const bootstrap = this.options.getBootstrap();
    if (!bootstrap || this.options.isNestedShellActive?.() === false) {
      return;
    }

    const command = synchronizeRender
      ? `${CLEAR_LINE} __TRON_SYNCED_BOOTSTRAP=1; if ${bootstrap}; then printf '\\033]133;A\\007\\033]133;B\\007\\033[H\\033[2J\\033[3J'; else printf '${SYNCHRONIZED_OUTPUT_END}'; fi\r`
      : `${CLEAR_LINE} ${bootstrap} && printf '\\033]133;A\\007\\033]133;B\\007\\033[H\\033[2J\\033[3J'\r`;

    // The Docker Bash command is echoed before it is evaluated. Start xterm's
    // synchronized-output mode locally first, then let the hook close it from
    // the first prompt. The user sees only the finished prompt, not a flash of
    // the injected command followed by a full-screen clear. If source fails,
    // the fallback close sequence renders the error instead.
    if (synchronizeRender && this.terminal) {
      this.terminal.write(SYNCHRONIZED_OUTPUT_BEGIN, () => {
        this.options.write(command);
      });
      return;
    }

    this.options.write(command);
  }
}

function canSynchronizeBootstrap(command: string) {
  return command
    .trim()
    .split(/\s+/)
    .some((token) => SYNCHRONIZED_SHELLS.has(token.split("/").pop() ?? ""));
}

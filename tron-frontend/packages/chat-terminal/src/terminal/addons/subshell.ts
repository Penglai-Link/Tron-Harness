import type { ITerminalAddon, Terminal as XTerminal } from "@xterm/xterm";

import { createChatTerminalI18n, type ChatTerminalI18n } from "../i18n";
import type { SubshellDetection } from "../types/shellIntegration";
import {
  readSubshellConsent,
  writeSubshellConsent,
} from "../utils/subshellConsent";
import { createSubshellPrompt, type SubshellPrompt } from "../ui/subshellPrompt";

export interface SubshellAddonOptions {
  i18n?: ChatTerminalI18n;
  /** Writes to the PTY, which at this point is the nested shell. */
  write: (data: string) => void;
  /** The bootstrap one-liner reported by the server for this session. */
  getBootstrap: () => string;
  /**
   * Whether a nested shell still holds the PTY.
   *
   * Checked immediately before writing: between detection and the write the
   * nested shell can die (a dropped SSH connection, a rejected credential), and
   * the payload would then land on the local prompt instead.
   */
  isNestedShellActive?: () => boolean;
}

/** Readline: kill to start of line, then kill to end of line. */
const CLEAR_LINE = "\x15\x0b";

/**
 * Installs the shell integration into a nested shell, with consent.
 *
 * When the user runs `ssh host`, the remote shell knows nothing about our
 * markers, so command blocks and prompt-aware AI stop working there. Warp solves
 * this by "warpifying" the subshell; we do the same, except that typing a command
 * into a shell the user opened is only done after they agree — per host, and
 * optionally remembered.
 */
export class SubshellBootstrapAddon implements ITerminalAddon {
  private readonly i18n: ChatTerminalI18n;
  private prompt?: SubshellPrompt;
  private terminal?: XTerminal;
  private pendingHost?: string;

  constructor(private readonly options: SubshellAddonOptions) {
    this.i18n = options.i18n ?? createChatTerminalI18n();
  }

  activate(terminal: XTerminal) {
    this.terminal = terminal;
    this.prompt = createSubshellPrompt({
      i18n: this.i18n,
      onEnable: (remember) => {
        const host = this.pendingHost ?? "";
        this.pendingHost = undefined;
        if (remember) {
          writeSubshellConsent(host, "allow");
        }
        this.install();
      },
      onDismiss: (remember) => {
        const host = this.pendingHost ?? "";
        this.pendingHost = undefined;
        if (remember) {
          writeSubshellConsent(host, "deny");
        }
      },
    });
    terminal.element?.appendChild(this.prompt.element);
  }

  dispose() {
    this.prompt?.element.remove();
    this.prompt = undefined;
    this.pendingHost = undefined;
    this.terminal = undefined;
  }

  refreshLocale() {
    this.prompt?.refreshLocale();
  }

  /** Hides the prompt, e.g. once the nested shell exits. */
  hide() {
    this.prompt?.hide();
    this.pendingHost = undefined;
  }

  handleDetection(detection: SubshellDetection) {
    // Backend now handles SSH command wrapping automatically.
    // This detection is kept for monitoring purposes only.
    // No bootstrap installation needed from the frontend.
    return;
  }

  private install() {
    const bootstrap = this.options.getBootstrap();
    if (!bootstrap) {
      return;
    }

    // Last check before typing: the consent dialog may have been open for a
    // while, and a nested shell that has since exited would leave this payload
    // sitting on the local prompt instead.
    if (this.options.isNestedShellActive?.() === false) {
      this.hide();
      return;
    }

    // Clear whatever is on the nested shell's input line first: injecting into a
    // half-typed line would produce a broken command.
    //
    // The nested shell echoes the line as it is typed, and the payload is ~1.5 KB
    // of base64. To hide it:
    // 1. Prefix with a space so shells with HISTCONTROL=ignorespace won't save it to history
    // 2. Use setopt hist_ignore_space for zsh / HISTCONTROL=ignorespace for bash
    // 3. Clear screen after execution
    // 4. Clear client-side terminal buffer
    // `&&` deliberately: if the bootstrap failed, its error message must stay on screen.
    const hiddenBootstrap = ` ${bootstrap}`;
    this.options.write(
      `${CLEAR_LINE}${hiddenBootstrap} && printf '\\033[H\\033[2J'\r`,
    );

    // After a short delay, clear the terminal buffer on the client side to
    // remove the bootstrap command from scrollback
    setTimeout(() => {
      if (this.terminal) {
        this.terminal.clear();
      }
    }, 150);
  }
}

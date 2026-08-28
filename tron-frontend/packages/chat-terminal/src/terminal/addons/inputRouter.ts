import type { IDisposable, ITerminalAddon, Terminal as XTerminal } from "@xterm/xterm";

import type {
  InputRouterAddonOptions,
  InputRouterInputState,
} from "../types/inputRouter";
import { TerminalCommandDetector } from "../utils/terminalCommandDetector";

const DEFAULT_AGENT_PREFIX = "#";
const ENTER_KEYS = new Set(["\r", "\n", "\r\n"]);
/** Readline redraw, used to clear the screen the way the shell expects. */
const REDRAW = "\x0c";

/**
 * Routes keystrokes between the shell and the agent.
 *
 * Unlike the local-echo addon it replaces, this does not echo anything: the PTY
 * is authoritative, so the shell renders the prompt and the typed line itself.
 * The only thing decided here is where a submitted line goes.
 *
 * Routing is only attempted while the shell reports a prompt. That single check
 * is what fixes AI over SSH: a remote shell with the integration installed
 * reports its own prompts, so the agent stays reachable there, while a
 * full-screen program like `vim` keeps every byte to itself.
 */
export class InputRouterAddon implements ITerminalAddon {
  private terminal?: XTerminal;
  private dataDisposable?: IDisposable;
  private agentRequestInFlight = false;

  constructor(private readonly options: InputRouterAddonOptions) {}

  activate(terminal: XTerminal) {
    this.terminal = terminal;
    this.dataDisposable = terminal.onData((data) => {
      void this.handleData(data);
    });
    terminal.attachCustomKeyEventHandler((event) => this.handleKeyEvent(event));
    this.notifyInputState();
  }

  dispose() {
    this.dataDisposable?.dispose();
    this.dataDisposable = undefined;
    this.terminal = undefined;
  }

  /** Re-reads the pending line, e.g. after the shell redrew the prompt. */
  notifyInputState() {
    const enabled =
      !this.isAlternateScreen() &&
      !this.agentRequestInFlight &&
      (this.options.isAtPrompt() ||
        this.options.isInUnintegratedSubshell?.() === true);
    const state: InputRouterInputState = {
      agentRequestInFlight: this.agentRequestInFlight,
      enabled,
      input: enabled ? this.options.readInputLine() : "",
    };
    this.options.onInputStateChange?.(state);
  }

  private async handleData(data: string) {
    if (this.agentRequestInFlight) {
      return;
    }

    if (this.isAlternateScreen()) {
      this.options.write(data);
      return;
    }

    // A command is running, a full-screen program is up, or the shell has not
    // reported a prompt yet: the shell owns every byte.
    // Exception: if we're in an unintegrated subshell (e.g., SSH without shell
    // integration), we still need to check for natural language to route to agent.
    const shouldCheckRouting =
      this.options.isAtPrompt() ||
      this.options.isInUnintegratedSubshell?.() === true;
    if (!shouldCheckRouting) {
      this.options.write(data);
      return;
    }

    if (!ENTER_KEYS.has(data)) {
      this.options.write(data);
      // The shell has not echoed the new character yet, so read after a turn.
      queueMicrotask(() => this.notifyInputState());
      return;
    }

    const line = this.options.readInputLine();
    const request = this.resolveAgentRequest(line);

    if (request === null) {
      // This is a command, let shell execute it normally
      this.options.write(data, false, line);
      this.notifyInputState();
      return;
    }

    // Keep the request visible in scrollback while the agent owns the line.
    // Clear readline before local card rows move xterm's cursor away from the
    // shell's cursor. The transport suppresses only this clear's visual echo,
    // leaving the submitted text visible as the card anchor.
    this.options.write("", true);
    this.options.onAgentInputSubmitted?.();
    await this.runAgentRequest(request, true);
  }

  /**
   * Returns the request to send to the agent, or null to let the shell run the
   * line. The explicit prefix wins; otherwise only high-confidence natural
   * language is diverted, so an ordinary command is never swallowed.
   */
  private resolveAgentRequest(line: string): string | null {
    const trimmed = line.trim();
    if (!trimmed) {
      return null;
    }

    const prefix = this.options.agentPrefix ?? DEFAULT_AGENT_PREFIX;
    if (prefix && trimmed.startsWith(prefix)) {
      const request = trimmed.slice(prefix.length).trim();
      return request || null;
    }

    const shouldRoute =
      this.options.shouldRouteToAgent ??
      ((input: string) => TerminalCommandDetector.agentConfidence(input) === "high");

    return shouldRoute(trimmed) ? trimmed : null;
  }

  private async runAgentRequest(request: string, finishShellLine = false) {
    if (this.agentRequestInFlight) {
      return false;
    }

    this.agentRequestInFlight = true;
    this.notifyInputState();
    try {
      await this.options.onAgentRequest(request);
    } catch (error) {
      console.error("Terminal agent request failed", error);
    } finally {
      if (finishShellLine) {
        this.options.write("\r");
      }
      this.agentRequestInFlight = false;
      this.notifyInputState();
    }
    return true;
  }

  /**
   * Handles shortcuts that never reach the PTY. Anything not claimed here is
   * passed to xterm untouched, so shell keybindings keep working.
   */
  private handleKeyEvent(event: KeyboardEvent) {
    if (event.type !== "keydown") {
      return true;
    }

    const key = event.key.toLowerCase();
    // Cmd/Ctrl+Shift+I: send the pending line to the agent without a prefix.
    const isAgentShortcut =
      key === "i" && (event.metaKey || (event.ctrlKey && event.shiftKey));
    const canUseShortcuts =
      !this.isAlternateScreen() &&
      (this.options.isAtPrompt() ||
        this.options.isInUnintegratedSubshell?.() === true);
    if (isAgentShortcut && canUseShortcuts) {
      const line = this.options.readInputLine().trim();
      if (line) {
        event.preventDefault();
        this.options.clearInputLine();
        void this.runAgentRequest(line);
        return false;
      }
    }

    // Cmd+K: clear host-owned overlays and scrollback, then ask the shell to
    // redraw its prompt so readline state stays synchronized.
    const isClearShortcut =
      key === "k" && event.metaKey && !event.altKey && !event.ctrlKey;
    if (isClearShortcut && canUseShortcuts) {
      event.preventDefault();
      this.options.onClearScreen?.();
      this.options.write(REDRAW);
      return false;
    }

    return true;
  }

  private isAlternateScreen() {
    return this.options.isAlternateScreen?.() === true;
  }
}

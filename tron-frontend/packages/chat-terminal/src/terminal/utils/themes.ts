import type { ITheme, Terminal as XTerminal } from "@xterm/xterm";
import type {
  TerminalThemeTokenOverrides,
  TerminalThemeTokens,
} from "../types/themes";

const TOKEN_VARIABLES: Record<
  Exclude<keyof TerminalThemeTokens, "ansi">,
  string
> = {
  background: "--ai-terminal-background",
  foreground: "--ai-terminal-foreground",
  cursor: "--ai-terminal-cursor",
  selectionBackground: "--ai-terminal-selection-background",
  panelBackground: "--ai-terminal-panel-background",
  panelForeground: "--ai-terminal-panel-foreground",
  muted: "--ai-terminal-muted",
  subtle: "--ai-terminal-subtle",
  border: "--ai-terminal-border",
  accent: "--ai-terminal-accent",
  codeBackground: "--ai-terminal-code-background",
  inputBackground: "--ai-terminal-input-background",
  hoverBackground: "--ai-terminal-hover-background",
  error: "--ai-terminal-error",
  warning: "--ai-terminal-warning",
  success: "--ai-terminal-success",
  icon: "--ai-terminal-icon",
};

const SEMANTIC_KEYS = Object.keys(TOKEN_VARIABLES) as Array<
  Exclude<keyof TerminalThemeTokens, "ansi">
>;

export function applyTerminalThemeTokens(
  container: HTMLElement,
  tokens: TerminalThemeTokenOverrides = {},
) {
  for (const key of SEMANTIC_KEYS) {
    const value = tokens[key];
    if (value === undefined) {
      continue;
    }
    container.style.setProperty(TOKEN_VARIABLES[key], value);
  }
}

export function clearTerminalThemeTokens(container: HTMLElement) {
  for (const variable of Object.values(TOKEN_VARIABLES)) {
    container.style.removeProperty(variable);
  }
}

function readCssColor(container: HTMLElement, variable: string) {
  const probe = document.createElement("span");
  probe.style.color = `var(${variable})`;
  probe.style.position = "absolute";
  probe.style.pointerEvents = "none";
  container.appendChild(probe);
  const color = getComputedStyle(probe).color;
  probe.remove();
  return color;
}

export function resolveTerminalThemeTokens(
  container: HTMLElement,
): TerminalThemeTokens {
  const resolved = {} as TerminalThemeTokens;
  for (const key of SEMANTIC_KEYS) {
    resolved[key] = readCssColor(container, TOKEN_VARIABLES[key]);
  }
  return resolved;
}

export function syncTerminalTheme(
  terminal: XTerminal,
  container: HTMLElement,
  ansi: Partial<ITheme> = {},
) {
  const tokens = resolveTerminalThemeTokens(container);
  terminal.options.theme = {
    background: tokens.background,
    foreground: tokens.foreground,
    cursor: tokens.cursor,
    selectionBackground: tokens.selectionBackground,
    ...ansi,
  };
}

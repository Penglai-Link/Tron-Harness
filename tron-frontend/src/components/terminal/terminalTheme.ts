import type { ITheme } from "@xterm/xterm";

type TerminalAppTheme = "light" | "dark";

const lightAnsiTheme: Partial<ITheme> = {
  black: "#1c1b1b",
  red: "#ba1a1a",
  green: "#2f6a3a",
  yellow: "#7a5400",
  blue: "#225e9f",
  magenta: "#7a3e8e",
  cyan: "#006f73",
  white: "#4b4b4b",
  brightBlack: "#6e6e6e",
  brightRed: "#d32f2f",
  brightGreen: "#1b7f45",
  brightYellow: "#8a5a00",
  brightBlue: "#2563b0",
  brightMagenta: "#9345a4",
  brightCyan: "#007f83",
  brightWhite: "#242424",
};

const matrixDarkAnsiTheme: Partial<ITheme> = {
  black: "#07120b",
  red: "#ff7b72",
  green: "#7ee787",
  yellow: "#e3b341",
  blue: "#79c0ff",
  magenta: "#d2a8ff",
  cyan: "#76e3ea",
  white: "#d6f5dc",
  brightBlack: "#8b949e",
  brightRed: "#ffa198",
  brightGreen: "#9af0a5",
  brightYellow: "#f2cc60",
  brightBlue: "#a5d6ff",
  brightMagenta: "#e2c5ff",
  brightCyan: "#a5f3fc",
  brightWhite: "#ffffff",
};

export function getTerminalAnsiTheme(theme: TerminalAppTheme): Partial<ITheme> {
  return theme === "light" ? lightAnsiTheme : matrixDarkAnsiTheme;
}

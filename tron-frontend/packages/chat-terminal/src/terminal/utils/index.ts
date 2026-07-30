import { Terminal as XTerminal } from "@xterm/xterm";
import type { CellMetrics } from "../types/common";

export function getTerminalCellMetrics(terminal: XTerminal): CellMetrics {
  const core = terminal as XTerminal & {
    _core?: {
      _renderService?: {
        dimensions?: {
          css?: {
            cell?: CellMetrics;
          };
        };
      };
    };
  };

  const width = core._core?._renderService?.dimensions?.css?.cell?.width;
  const height = core._core?._renderService?.dimensions?.css?.cell?.height;

  if (width && height) {
    return { width, height };
  }

  return {
    width: 9,
    height: 18,
  };
}

export function normalizeTerminalOutput(output: string) {
  return output.replace(/\r?\n/g, "\r\n");
}

export function hasVisibleTerminalOutput(output: string) {
  const text = output
    .replace(/\u001B\][^\u0007]*(?:\u0007|\u001B\\)/g, "")
    .replace(/(?:\u001B\[|\u009B)[0-?]*[ -/]*[@-~]/g, "")
    .replace(/\u001B[@-_]/g, "");

  return text.trim().length > 0;
}

export function writeTerminalAsync(terminal: XTerminal, data: string) {
  return new Promise<void>((resolve) => {
    terminal.write(data, resolve);
  });
}

export function isControlCharacter(data: string) {
  return data.length === 1 && data.charCodeAt(0) < 32 && data !== "\t";
}

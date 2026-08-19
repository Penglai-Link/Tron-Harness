export type TerminalMessageType =
  | "TEXT_MESSAGE_START"
  | "TEXT_MESSAGE_CONTENT"
  | "TEXT_MESSAGE_END"
  | "TEXT_MESSAGE_ERROR"
  | "TEXT_MESSAGE_SUGGESTION";

export interface TerminalMessage {
  currentDirectory?: string;
  delta: string;
  messageId: string;
  timestamp: number;
  type: TerminalMessageType;
}

const TERMINAL_MESSAGE_TYPES = new Set<TerminalMessageType>([
  "TEXT_MESSAGE_START",
  "TEXT_MESSAGE_CONTENT",
  "TEXT_MESSAGE_END",
  "TEXT_MESSAGE_ERROR",
  "TEXT_MESSAGE_SUGGESTION",
]);

export function parseTerminalMessage(
  value: unknown,
): TerminalMessage | undefined {
  if (
    !isRecord(value) ||
    typeof value.delta !== "string" ||
    typeof value.messageId !== "string" ||
    !value.messageId ||
    typeof value.timestamp !== "number" ||
    !Number.isFinite(value.timestamp) ||
    typeof value.type !== "string" ||
    !TERMINAL_MESSAGE_TYPES.has(value.type as TerminalMessageType) ||
    (value.currentDirectory !== undefined &&
      (typeof value.currentDirectory !== "string" || !value.currentDirectory))
  ) {
    return undefined;
  }

  return value as unknown as TerminalMessage;
}

export type TerminalSessionResponse =
  | { currentDirectory: string; error?: never }
  | { currentDirectory?: never; error: string };

export function parseTerminalSession(
  value: unknown,
): TerminalSessionResponse | undefined {
  const message = parseTerminalMessage(value);
  if (!message) {
    return undefined;
  }

  if (message.type === "TEXT_MESSAGE_ERROR") {
    return message.delta ? { error: message.delta } : undefined;
  }

  if (
    message.type !== "TEXT_MESSAGE_CONTENT" ||
    message.delta !== "" ||
    !message.currentDirectory
  ) {
    return undefined;
  }

  return { currentDirectory: message.currentDirectory };
}

export function createMessageId() {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

import { randomUUID } from "node:crypto";

export const MessageType = Object.freeze({
  CONTENT: "TEXT_MESSAGE_CONTENT",
  END: "TEXT_MESSAGE_END",
  ERROR: "TEXT_MESSAGE_ERROR",
  START: "TEXT_MESSAGE_START",
  SUGGESTION: "TEXT_MESSAGE_SUGGESTION",
});

const MESSAGE_TYPES = new Set(Object.values(MessageType));

export function createMessage({
  currentDirectory,
  delta = "",
  messageId,
  now = Date.now,
  type,
}) {
  if (!MESSAGE_TYPES.has(type)) {
    throw new TypeError(`不支持的消息类型：${type}`);
  }

  const message = {
    delta: String(delta),
    messageId: resolveMessageId(messageId),
    timestamp: now(),
    type,
  };

  if (typeof currentDirectory === "string" && currentDirectory) {
    message.currentDirectory = currentDirectory;
  }

  return message;
}

export function resolveMessageId(value, createId = randomUUID) {
  return typeof value === "string" && value.trim() ? value.trim() : createId();
}

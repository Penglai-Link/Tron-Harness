import type { ChatMessage } from "./types";

function timestampOf(message: ChatMessage) {
  const timestamp = Date.parse(message.createdAt);
  return Number.isNaN(timestamp) ? Number.MAX_SAFE_INTEGER : timestamp;
}

function compareMessageIds(left: ChatMessage, right: ChatMessage) {
  const leftId = String(left.id);
  const rightId = String(right.id);
  const decimalIdPattern = /^\d+$/;

  if (decimalIdPattern.test(leftId) && decimalIdPattern.test(rightId)) {
    if (leftId.length !== rightId.length) return leftId.length - rightId.length;
  }

  const idOrder = leftId.localeCompare(rightId);
  if (idOrder !== 0) return idOrder;
  return left.messageCode.localeCompare(right.messageCode);
}

export function sortChatMessagesChronologically(
  messages: readonly ChatMessage[],
) {
  return [...messages].sort((left, right) => {
    const timestampOrder = timestampOf(left) - timestampOf(right);
    return timestampOrder || compareMessageIds(left, right);
  });
}

export function mergeChatMessagesChronologically(
  current: readonly ChatMessage[],
  incoming: readonly ChatMessage[],
  removedMessageCodes: readonly string[] = [],
) {
  const removed = new Set(removedMessageCodes);
  const byMessageCode = new Map<string, ChatMessage>();

  for (const message of [...current, ...incoming]) {
    if (!removed.has(message.messageCode)) {
      byMessageCode.set(message.messageCode, message);
    }
  }

  return sortChatMessagesChronologically([...byMessageCode.values()]);
}

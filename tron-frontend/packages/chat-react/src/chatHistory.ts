import {
  truncateLabel,
  type ChatMessage,
  type TranscriptPresentation,
} from "@tron/chat-core";

export interface ChatHistoryTurn {
  assistantPreview: string;
  hasAssistantReply: boolean;
  hasWorkActivity: boolean;
  id: string;
  messageCodes: string[];
  userMessageCode: string;
  userPreview: string;
}

function messagePreview(content: string, maxLength = 96) {
  return truncateLabel(content.replace(/\s+/g, " ").trim(), maxLength);
}

function markdownPreview(content: string, maxLength = 240) {
  const normalized = content.replace(/\r\n?/g, "\n").trim();
  if (normalized.length <= maxLength) return normalized;
  return `${normalized.slice(0, Math.max(0, maxLength - 3)).trimEnd()}...`;
}

export function buildChatHistoryTurns(
  messages: ChatMessage[],
  presentation: TranscriptPresentation,
) {
  const turns: ChatHistoryTurn[] = [];
  let currentTurn: ChatHistoryTurn | null = null;

  function pushCurrentTurn() {
    if (!currentTurn) return;
    turns.push(currentTurn);
    currentTurn = null;
  }

  messages.forEach((message) => {
    if (message.role === "USER") {
      pushCurrentTurn();
      currentTurn = {
        assistantPreview: "",
        hasAssistantReply: false,
        hasWorkActivity: false,
        id: `turn-${message.messageCode}`,
        messageCodes: [message.messageCode],
        userMessageCode: message.messageCode,
        userPreview: messagePreview(message.content, 72),
      };
      return;
    }

    if (!currentTurn) return;

    currentTurn.messageCodes.push(message.messageCode);

    if (!currentTurn.hasAssistantReply && message.content.trim()) {
      currentTurn.hasAssistantReply = true;
      currentTurn.assistantPreview = markdownPreview(message.content);
    }
  });

  pushCurrentTurn();

  const workMessageCodes = new Set<string>();

  if (presentation.waitingForModelAnchorMessageCode) {
    workMessageCodes.add(presentation.waitingForModelAnchorMessageCode);
  }

  presentation.workRailsByMessageCode.forEach((runs, messageCode) => {
    if (runs.length) workMessageCodes.add(messageCode);
  });

  turns.forEach((turn) => {
    turn.hasWorkActivity = turn.messageCodes.some((messageCode) =>
      workMessageCodes.has(messageCode),
    );
  });

  return turns.filter((turn) => turn.userPreview);
}

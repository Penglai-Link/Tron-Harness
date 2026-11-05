import { chatErrorPresentation } from "@eliteforge/agent-ui-chat-core";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";

const errorTitles = {
  "Chat suggestions failed": "suggestionsFailedTitle",
  "Chat send failed": "chatSendFailedTitle",
  "Chat load failed": "chatLoadFailedTitle",
  "Chat delete failed": "chatDeleteFailedTitle",
  "Chat answer failed": "answerFailedTitle",
  "Chat continuation failed": "continuationFailedTitle",
  "Chat steer failed": "steerFailedTitle",
  "Chat stop failed": "chatStopFailedTitle",
  "Chat attachment upload failed": "attachmentUploadFailed",
} as const;

const feedbackMessages = {
  "chat storage failed": "chatStorageFailed",
  "chat session was not found": "chatSessionNotFound",
  "workspaceCode is required": "workspaceRequired",
  "Workspace code is required before starting a session": "workspaceRequired",
  "Chat send failed": "chatSendFailed",
  "Chat data could not be loaded": "chatLoadFailed",
  "Chat answer failed": "answerFailed",
  "Chat steer failed": "steerFailed",
  "Chat stop failed": "chatStopFailed",
  "Chat attachment upload failed": "attachmentUploadFailed",
  "Current response has been stopped": "chatStopped",
  "Another response is already running; your message was queued.": "messageQueued",
} as const;

const networkFailureMessages = new Set([
  "failed to fetch",
  "fetch failed",
  "networkerror when attempting to fetch resource.",
  "network request failed",
  "load failed",
]);

export function useChatFeedback() {
  const toast = useToast();
  const { t } = useTranslation("pages");
  const error = useCallback((message: string, title?: string) => {
    const normalizedMessage = message.trim().replace(/^(?:TypeError|NetworkError|ChatAdapterError):\s*/i, "");
    const networkFailure = networkFailureMessages.has(normalizedMessage.toLowerCase());
    const answerUnconfirmed = networkFailure && title === "Chat answer failed";
    const messageKey = answerUnconfirmed ? "answerConnectionLost"
      : networkFailure ? "chatConnectionFailed"
      : feedbackMessages[normalizedMessage as keyof typeof feedbackMessages];
    const titleKey = answerUnconfirmed ? "answerUnconfirmedTitle"
      : errorTitles[title as keyof typeof errorTitles] ?? "chatSendFailedTitle";
    toast.error(messageKey ? t(`chat.management.toasts.${messageKey}`) : chatErrorPresentation(message, t("chat.management.toasts.chatSendFailedTitle")).summary, {
      title: t(`chat.management.toasts.${titleKey}`),
    });
  }, [t, toast]);
  const info = useCallback((message: string, title?: string) => {
    const messageKey = feedbackMessages[message as keyof typeof feedbackMessages];
    const titleKey = title === "Chat stopped" ? "chatStoppedTitle"
      : title === "Message queued" ? "messageQueuedTitle" : undefined;
    toast.info(messageKey ? t(`chat.management.toasts.${messageKey}`) : message, {
      title: titleKey ? t(`chat.management.toasts.${titleKey}`) : title,
    });
  }, [t, toast]);
  return { error, info };
}

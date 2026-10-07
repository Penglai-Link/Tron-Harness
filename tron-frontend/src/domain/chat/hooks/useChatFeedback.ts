import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/components/ui/toast";

const errorTitles = {
  "Chat send failed": "chatSendFailedTitle",
  "Chat load failed": "chatLoadFailedTitle",
  "Chat answer failed": "answerFailedTitle",
  "Chat steer failed": "steerFailedTitle",
  "Chat stop failed": "chatStopFailedTitle",
  "Chat attachment upload failed": "attachmentUploadFailed",
} as const;

const feedbackMessages = {
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

export function useChatFeedback() {
  const toast = useToast();
  const { t } = useTranslation("pages");
  const error = useCallback((message: string, title?: string) => {
    const messageKey = feedbackMessages[message as keyof typeof feedbackMessages];
    const titleKey = errorTitles[title as keyof typeof errorTitles] ?? "chatSendFailedTitle";
    toast.error(messageKey ? t(`chat.management.toasts.${messageKey}`) : message, {
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

import type { ChatQueuedSubmission } from "./types";

const safeSteerQueueFallbackCodes = new Set([
  "CHAT_STEER_NO_ACTIVE_RUN",
  "CHAT_STEER_RUN_FINISHING",
  "CHAT_STEER_RUN_MISMATCH",
  "CHAT_STEER_RUN_STARTING",
  "CHAT_STEER_RUNTIME_UNSUPPORTED",
]);

export function isSteerQueueFallbackCode(code: string | null | undefined) {
  return Boolean(code && safeSteerQueueFallbackCodes.has(code));
}

export function enqueueChatSubmission(
  queue: readonly ChatQueuedSubmission[],
  submission: ChatQueuedSubmission,
) {
  return [...queue, submission];
}

export function removeQueuedChatSubmission(
  queue: readonly ChatQueuedSubmission[],
  id: string,
) {
  return queue.filter((submission) => submission.id !== id);
}

export function takeNextQueuedChatSubmission(
  queue: readonly ChatQueuedSubmission[],
  sessionCode: string,
) {
  const submissionIndex = queue.findIndex(
    (submission) => submission.sessionCode === sessionCode,
  );
  if (submissionIndex < 0) {
    return {
      submission: null,
      remaining: [...queue],
    };
  }

  return {
    submission: queue[submissionIndex],
    remaining: queue.filter((_, index) => index !== submissionIndex),
  };
}

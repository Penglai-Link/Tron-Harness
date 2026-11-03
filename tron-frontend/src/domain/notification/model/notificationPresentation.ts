import type { PlatformNotification } from "../api/notificationApi";

const changeKinds = ["Progress", "Completed", "NeedsDecision", "Blocked"] as const;
export type NotificationChangeKind = typeof changeKinds[number];

export function notificationContent(notification: PlatformNotification) {
  const target = notification.target ?? {};
  const changeKind = changeKinds.find((kind) => kind === target.changeKind) ?? null;
  return {
    changeKind,
    requestedAction: typeof target.requestedAction === "string" ? target.requestedAction.trim() : "",
    resolution: typeof target.resolution === "string" ? target.resolution.trim() : "",
    evidence: Array.isArray(target.evidence)
      ? target.evidence.filter((entry): entry is string => typeof entry === "string" && entry.trim().length > 0)
      : [],
  };
}

/** Keep the inbox bounded; the reader always receives the original full body. */
export function notificationPreview(body: string) {
  const compact = body.replace(/\s+/gu, " ").trim();
  const characters = Array.from(compact.slice(0, 482));
  return characters.length > 240 ? `${characters.slice(0, 240).join("")}…` : compact;
}

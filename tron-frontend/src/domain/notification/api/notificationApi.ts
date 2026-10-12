import { requestApi } from "@/shared/api/httpClient";
export interface PlatformNotification {
  id: string; dudeCode: string | null; title: string; body: string; category: string;
  target: Record<string, unknown>; readAt: number | null; resolvedAt: number | null;
  archivedAt: number | null; createdAt: number;
}
export const notificationQueryKey = ["platform-notifications"] as const;
export const listNotifications = (signal?: AbortSignal) => requestApi<PlatformNotification[]>("GET /api/v1/notifications", { signal });
export const updateNotification = (id: string, body: { read: boolean; archived: boolean }) => requestApi<unknown>("PATCH /api/v1/notifications/{id}", { pathParams: { id }, body });
export const deleteNotification = (id: string) => requestApi<unknown>("DELETE /api/v1/notifications/{id}", { pathParams: { id } });
export const deleteArchivedNotifications = () => requestApi<number>("DELETE /api/v1/notifications");
export function notificationTarget(notification: PlatformNotification): string | null {
  const target = notification.target ?? {};
  if (typeof target.href === "string" && target.href.startsWith("/") && !target.href.startsWith("//")) return target.href;
  if (typeof target.candidateCode === "string") return `/settings/skills?tab=approvals&candidateCode=${encodeURIComponent(target.candidateCode)}`;
  const dude = typeof target.dudeCode === "string" ? target.dudeCode : notification.dudeCode;
  if (dude) {
    const query = new URLSearchParams({ dude });
    if (typeof target.sessionCode === "string") { query.set("session", target.sessionCode); query.set("tab", target.sessionKind === "Thread" ? "work" : "history"); }
    else if (typeof target.taskCode === "string") { query.set("task", target.taskCode); query.set("tab", "tasks"); }
    return `/dudes?${query}`;
  }
  return null;
}

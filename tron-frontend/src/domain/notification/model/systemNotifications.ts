import { DesktopBridge } from "@/shared/desktop/DesktopBridge";

import { getBrowserNotificationPermission } from "./browserNotifications";

const notificationIdPrefix = "tron-notification-";

export interface SystemNotificationRequest {
  body: string;
  title: string;
  id?: string;
}

export interface SystemNotificationResult {
  channel: "browser" | "desktop";
  error?: string;
  shown: boolean;
}

export function notificationIdForItem(itemId: string) {
  return `${notificationIdPrefix}${itemId}`;
}

export function itemIdFromNotificationId(id: string): string | null {
  return id.startsWith(notificationIdPrefix) ? id.slice(notificationIdPrefix.length) : null;
}

export async function showSystemNotification(
  request: SystemNotificationRequest,
  options: { onActivate?: () => void } = {},
): Promise<SystemNotificationResult> {
  const bridge = new DesktopBridge();
  if (bridge.isElectron()) {
    try {
      const result = await bridge.showSystemNotification(request);
      return { channel: "desktop", error: result.error, shown: result.shown };
    } catch (error) {
      return { channel: "desktop", error: error instanceof Error ? error.message : String(error), shown: false };
    }
  }

  if (getBrowserNotificationPermission() !== "granted") {
    return { channel: "browser", error: "permission-not-granted", shown: false };
  }
  try {
    const notification = new Notification(request.title, {
      body: request.body,
      tag: request.id ?? notificationIdPrefix + Date.now(),
    });
    notification.onclick = () => {
      window.focus();
      notification.close();
      options.onActivate?.();
    };
    return { channel: "browser", shown: true };
  } catch (error) {
    return { channel: "browser", error: error instanceof Error ? error.message : String(error), shown: false };
  }
}

export async function openSystemNotificationSettings(): Promise<boolean> {
  const bridge = new DesktopBridge();
  if (!bridge.isElectron()) return false;
  try {
    return await bridge.openSystemNotificationSettings();
  } catch {
    return false;
  }
}

export function onSystemNotificationActivated(listener: (id: string) => void): () => void {
  const bridge = new DesktopBridge();
  if (!bridge.isElectron()) return () => {};
  return bridge.onSystemNotificationActivated(listener);
}

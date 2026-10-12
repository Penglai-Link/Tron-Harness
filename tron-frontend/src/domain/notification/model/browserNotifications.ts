const storageKey = "tron.browserNotifications.enabled";
const changedEventName = "tron.browserNotifications.changed";

export type BrowserNotificationPermission = NotificationPermission | "unsupported";

export function getBrowserNotificationPermission(): BrowserNotificationPermission {
  if (typeof window === "undefined" || !window.isSecureContext || !("Notification" in window)) {
    return "unsupported";
  }
  return Notification.permission;
}

export function areBrowserNotificationsEnabled() {
  if (getBrowserNotificationPermission() !== "granted") return false;
  try {
    return window.localStorage.getItem(storageKey) === "true";
  } catch {
    return false;
  }
}

export function setBrowserNotificationsEnabled(enabled: boolean) {
  try {
    window.localStorage.setItem(storageKey, String(enabled));
    window.dispatchEvent(new CustomEvent(changedEventName));
  } catch {
    // Storage can be unavailable in restricted browser contexts.
  }
}

export function browserNotificationPreferenceChangedEvent() {
  return changedEventName;
}

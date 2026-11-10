import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { SurfaceCard } from "@/shared";

import {
  areBrowserNotificationsEnabled,
  browserNotificationPreferenceChangedEvent,
  getBrowserNotificationPermission,
  setBrowserNotificationsEnabled,
  type BrowserNotificationPermission,
} from "../model/browserNotifications";

export function BrowserNotificationSettingsCard() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [permission, setPermission] = useState<BrowserNotificationPermission>(() => getBrowserNotificationPermission());
  const [enabled, setEnabled] = useState(() => areBrowserNotificationsEnabled());
  const [pending, setPending] = useState(false);
  const [requestFailed, setRequestFailed] = useState(false);

  useEffect(() => {
    const refresh = () => {
      setPermission(getBrowserNotificationPermission());
      setEnabled(areBrowserNotificationsEnabled());
    };
    const eventName = browserNotificationPreferenceChangedEvent();
    window.addEventListener(eventName, refresh);
    window.addEventListener("storage", refresh);
    return () => {
      window.removeEventListener(eventName, refresh);
      window.removeEventListener("storage", refresh);
    };
  }, []);

  async function toggle() {
    if (pending || permission === "unsupported" || permission === "denied") return;
    setPending(true);
    setRequestFailed(false);
    try {
      let nextPermission: NotificationPermission = permission;
      if (permission === "default") nextPermission = await Notification.requestPermission();
      setPermission(nextPermission);
      const nextEnabled = nextPermission === "granted" && !enabled;
      setBrowserNotificationsEnabled(nextEnabled);
      setEnabled(nextEnabled);
    } catch {
      setRequestFailed(true);
    } finally {
      setPending(false);
    }
  }

  function sendTestNotification() {
    if (!areBrowserNotificationsEnabled()) {
      setPermission(getBrowserNotificationPermission());
      setEnabled(false);
      toast.error(t("settingsGeneral.browserNotifications.test.requireEnabled"));
      return;
    }
    try {
      // Manual testing also works in the foreground, unlike automatic alerts.
      const notification = new Notification(t("settingsGeneral.browserNotifications.test.title"), {
        body: t("settingsGeneral.browserNotifications.test.body"),
        tag: "tron-test-notification",
      });
      notification.onclick = () => { window.focus(); notification.close(); };
      notification.onerror = () => toast.error(t("settingsGeneral.browserNotifications.test.failed"));
    } catch {
      toast.error(t("settingsGeneral.browserNotifications.test.failed"));
    }
  }

  const statusKey = permission === "unsupported"
    ? "unsupported"
    : permission === "denied"
      ? "denied"
      : enabled
        ? "enabled"
        : permission === "granted"
          ? "authorized"
          : "disabled";
  const actionKey = permission === "default"
    ? "authorize"
    : permission === "granted"
      ? enabled ? "disable" : "enable"
      : null;

  return (
    <SurfaceCard className="space-y-4 xl:col-span-2">
      <div className="space-y-1">
        <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">
          {t("settingsGeneral.browserNotifications.title")}
        </h2>
        {(requestFailed || permission === "denied" || permission === "unsupported") && <p className="text-sm text-[var(--color-on-surface-variant)]">
          {t(`settingsGeneral.browserNotifications.status.${requestFailed ? "requestFailed" : statusKey}`)}
        </p>}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        {actionKey ? (
          <Button type="button" variant={enabled ? "outline" : "default"} disabled={pending} onClick={() => void toggle()}>
            {t(`settingsGeneral.browserNotifications.actions.${actionKey}`)}
          </Button>
        ) : null}
        <Button
          type="button"
          variant="outline"
          disabled={pending || !enabled || permission !== "granted"}
          title={!enabled || permission !== "granted" ? t("settingsGeneral.browserNotifications.test.requireEnabled") : undefined}
          onClick={sendTestNotification}
        >
          {t("settingsGeneral.browserNotifications.actions.test")}
        </Button>
      </div>
    </SurfaceCard>
  );
}

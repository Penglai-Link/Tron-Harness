import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { SurfaceCard } from "@/shared";

import {
  areBrowserNotificationsEnabled,
  browserNotificationPreferenceChangedEvent,
  getBrowserNotificationPermission,
  setBrowserNotificationsEnabled,
  type BrowserNotificationPermission,
} from "../model/browserNotifications";
import { openSystemNotificationSettings, showSystemNotification } from "../model/systemNotifications";

export function BrowserNotificationSettingsCard() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [permission, setPermission] = useState<BrowserNotificationPermission>(() => getBrowserNotificationPermission());
  const [enabled, setEnabled] = useState(() => areBrowserNotificationsEnabled());
  const [pending, setPending] = useState(false);
  const [requestFailed, setRequestFailed] = useState(false);
  const [deliveryFailed, setDeliveryFailed] = useState(false);
  const canOpenSettings = useMemo(() => {
    const bridge = new DesktopBridge();
    const platform = bridge.desktopPlatform();
    return bridge.isElectron() && (platform === "darwin" || platform === "win32");
  }, []);

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
    setDeliveryFailed(false);
    try {
      let nextPermission: NotificationPermission = permission;
      if (permission === "default") nextPermission = await Notification.requestPermission();
      setPermission(nextPermission);
      const nextEnabled = nextPermission === "granted" && !enabled;
      setBrowserNotificationsEnabled(nextEnabled);
      setEnabled(nextEnabled);
      if (nextEnabled) await dispatchTestNotification();
    } catch {
      setRequestFailed(true);
    } finally {
      setPending(false);
    }
  }

  async function dispatchTestNotification() {
    const result = await showSystemNotification({
      title: t("settingsGeneral.browserNotifications.test.title"),
      body: t("settingsGeneral.browserNotifications.test.body"),
      id: "tron-test-notification",
    });
    setDeliveryFailed(!result.shown);
    if (result.shown) {
      toast.success(t("settingsGeneral.browserNotifications.test.sent"));
    } else {
      toast.error(t("settingsGeneral.browserNotifications.test.failed"));
    }
  }

  async function sendTestNotification() {
    if (!areBrowserNotificationsEnabled()) {
      setPermission(getBrowserNotificationPermission());
      setEnabled(false);
      toast.error(t("settingsGeneral.browserNotifications.test.requireEnabled"));
      return;
    }
    if (pending) return;
    setPending(true);
    try {
      await dispatchTestNotification();
    } finally {
      setPending(false);
    }
  }

  async function openSettings() {
    const opened = await openSystemNotificationSettings();
    if (!opened) toast.info(t("settingsGeneral.browserNotifications.settingsUnavailable"));
  }

  const statusKey = requestFailed
    ? "requestFailed"
    : deliveryFailed
      ? "deliveryFailed"
      : permission === "unsupported"
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
  const statusText = t(`settingsGeneral.browserNotifications.status.${statusKey}`);

  return (
    <SurfaceCard className="space-y-4 xl:col-span-2">
      <div className="space-y-1">
        <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">
          {t("settingsGeneral.browserNotifications.title")}
        </h2>
        <p className="text-sm text-[var(--color-on-surface-variant)]">{statusText}</p>
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
          onClick={() => void sendTestNotification()}
        >
          {t("settingsGeneral.browserNotifications.actions.test")}
        </Button>
        {canOpenSettings ? (
          <Button type="button" variant="outline" disabled={pending} onClick={() => void openSettings()}>
            {t("settingsGeneral.browserNotifications.actions.settings")}
          </Button>
        ) : null}
      </div>
    </SurfaceCard>
  );
}

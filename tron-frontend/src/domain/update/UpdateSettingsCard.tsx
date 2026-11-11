import { useEffect, useState } from "react";

import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { SurfaceCard } from "@/shared";
import { useToast } from "@/components/ui/toast";
import { useUpdater } from "./useUpdater";

function errorMessage(error: unknown) {
  return error instanceof Error && error.message ? error.message : String(error || "Update failed");
}

export function UpdateSettingsCard() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const { apply, bridge, check, isDesktop, startDownload, status } = useUpdater();
  const [appVersion, setAppVersion] = useState("");
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    if (!isDesktop) return;
    void bridge.getAppVersion().then(setAppVersion).catch(() => setAppVersion(""));
  }, [bridge, isDesktop]);

  if (!isDesktop) return null;

  async function handleCheck() {
    setChecking(true);
    try {
      await check();
    } catch (error) {
      toast.error(errorMessage(error), { title: t("update.errors.check") });
    } finally {
      setChecking(false);
    }
  }

  async function handleDownload() {
    try {
      await startDownload();
    } catch (error) {
      toast.error(errorMessage(error), { title: t("update.errors.download") });
    }
  }

  async function handleApply() {
    try {
      await apply();
    } catch (error) {
      toast.error(errorMessage(error), { title: t("update.errors.install") });
    }
  }

  const progress = Math.max(0, Math.min(100, Math.round(status.progress ?? 0)));
  const manualInstall = status.installationMode === "manual";
  const statusText = status.type === "upToDate"
    ? t("update.status.upToDate")
    : status.type === "updateAvailable"
      ? t("update.status.available", { version: status.version })
      : status.type === "downloading"
        ? t("update.downloading", { progress })
        : status.type === "downloaded"
          ? t(manualInstall ? "update.manualDownloaded" : "update.downloaded")
          : status.type === "installing"
            ? t("update.installing")
            : status.type === "error"
              ? t("update.errors.generic")
              : "";

  return (
    <SurfaceCard className="space-y-4 xl:col-span-2" data-testid="update-settings-card">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-[var(--color-on-surface)]">{t("settingsGeneral.update.title")}</h2>
        </div>
        <Button disabled={checking || status.type === "downloading"} size="sm" variant="secondary" onClick={() => void handleCheck()}>

          {checking ? t("update.actions.checking") : t("update.actions.check")}
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-x-6 gap-y-2 text-sm">
        <span className="text-[var(--color-on-surface-variant)]">{t("update.currentVersion")}</span>
        <span className="font-medium text-[var(--color-on-surface)]">{appVersion || "-"}</span>
        {statusText ? <span className="text-[var(--color-on-surface-variant)]">{statusText}</span> : null}
      </div>

      {status.type === "downloading" ? (
        <div className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container)]">
          <div className="h-full rounded-full bg-[var(--color-primary)] transition-[width]" style={{ width: `${progress}%` }} />
        </div>
      ) : null}

      {status.type === "updateAvailable" ? (
        <Button size="sm" onClick={() => void handleDownload()}>

          {t(manualInstall ? "update.actions.downloadManual" : "update.actions.download")}
        </Button>
      ) : null}
      {status.type === "downloaded" ? (
        <Button size="sm" onClick={() => void handleApply()}>

          {t(manualInstall ? "update.actions.installManual" : "update.actions.install")}
        </Button>
      ) : null}
    </SurfaceCard>
  );
}

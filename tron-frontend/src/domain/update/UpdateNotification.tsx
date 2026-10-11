import { X, Download, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { useAuthSession } from "@/domain/auth";
import { useUpdater } from "./useUpdater";

const STARTUP_CHECK_DELAY_MS = 2500;

function errorMessage(error: unknown) {
  return error instanceof Error && error.message ? error.message : String(error || "Update failed");
}

export function UpdateNotification() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const { session, status: authStatus } = useAuthSession();
  const { apply, check, isDesktop, startDownload, status } = useUpdater();
  const [dismissed, setDismissed] = useState(false);
  const [startupErrorHidden, setStartupErrorHidden] = useState(true);

  useEffect(() => {
    if (!isDesktop || authStatus !== "authenticated" || !session) return undefined;

    const timer = window.setTimeout(() => {
      void check()
        .then(() => setStartupErrorHidden(true))
        .catch(() => setStartupErrorHidden(true));
    }, STARTUP_CHECK_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [authStatus, check, isDesktop, session]);

  useEffect(() => {
    if (status.type === "updateAvailable") setDismissed(false);
  }, [status.type, status.version, status.versionId]);

  if (!isDesktop) return null;
  if (status.type === "error" && startupErrorHidden) return null;
  if (status.type === "idle" || status.type === "checking" || status.type === "upToDate") return null;
  if (dismissed && status.type !== "installing") return null;

  async function handleDownload() {
    setStartupErrorHidden(false);
    setDismissed(false);
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

  async function handleRetry() {
    setStartupErrorHidden(false);
    try {
      await check();
    } catch (error) {
      toast.error(errorMessage(error), { title: t("update.errors.check") });
    }
  }

  const progress = Math.max(0, Math.min(100, Math.round(status.progress ?? 0)));
  const isDownloading = status.type === "downloading";
  const isDownloaded = status.type === "downloaded";
  const isInstalling = status.type === "installing";
  const isError = status.type === "error";

  return (
    <section
      aria-live="polite"
      className="fixed bottom-4 right-4 z-[120] w-[min(25rem,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-4 shadow-[0_18px_45px_rgba(15,23,42,0.16)]"
      data-testid="desktop-update-notification"
    >
      <div className="flex items-start gap-3">
        <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-[var(--color-surface-container)] text-[var(--color-primary)]">
          {isDownloading ? <Download size={16} className="animate-pulse" aria-hidden="true" /> : <RefreshCw size={16} aria-hidden="true" />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-sm font-semibold text-[var(--color-on-surface)]">{t("update.title")}</p>
              {status.version ? <p className="mt-0.5 text-xs text-[var(--color-on-surface-variant)]">v{status.version}</p> : null}
            </div>
            {!isInstalling ? (
              <button
                aria-label={t("update.actions.dismiss")}
                className="rounded-md p-1 text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]"
                type="button"
                onClick={() => setDismissed(true)}
              >
                <X size={15} aria-hidden="true" />
              </button>
            ) : null}
          </div>

          {status.type === "updateAvailable" ? (
            <p className="mt-2 text-[13px] text-[var(--color-on-surface-variant)]">{t("update.available")}</p>
          ) : null}
          {isDownloading ? (
            <div className="mt-3">
              <div className="h-1.5 overflow-hidden rounded-full bg-[var(--color-surface-container)]">
                <div className="h-full rounded-full bg-[var(--color-primary)] transition-[width]" style={{ width: `${progress}%` }} />
              </div>
              <p className="mt-1.5 text-xs text-[var(--color-on-surface-variant)]">{t("update.downloading", { progress })}</p>
            </div>
          ) : null}
          {isDownloaded ? <p className="mt-2 text-[13px] text-[var(--color-on-surface-variant)]">{t("update.downloaded")}</p> : null}
          {isInstalling ? <p className="mt-2 text-[13px] text-[var(--color-on-surface-variant)]">{t("update.installing")}</p> : null}
          {isError ? <p className="mt-2 break-words text-[13px] text-destructive">{status.message || t("update.errors.generic")}</p> : null}

          {!isDownloading && !isInstalling ? (
            <div className="mt-3 flex flex-wrap items-center gap-2">
              {status.type === "updateAvailable" ? (
                <Button size="sm" onClick={() => void handleDownload()}>

                  {t("update.actions.download")}
                </Button>
              ) : null}
              {isDownloaded ? (
                <Button size="sm" onClick={() => void handleApply()}>

                  {t("update.actions.install")}
                </Button>
              ) : null}
              {isError ? (
                <Button size="sm" variant="secondary" onClick={() => void handleRetry()}>

                  {t("update.actions.retry")}
                </Button>
              ) : null}
              {status.type === "updateAvailable" || isError ? (
                <Button size="sm" variant="ghost" onClick={() => setDismissed(true)}>{t("update.actions.later")}</Button>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
    </section>
  );
}

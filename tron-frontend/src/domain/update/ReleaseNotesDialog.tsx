import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { MarkdownContent } from "@/components/markdown-content/MarkdownContent";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { DesktopBridge } from "@/shared/desktop/DesktopBridge";
import { OPEN_RELEASE_NOTES_EVENT } from "./releaseNotesEvents";

const VERSION = __TRON_APP_VERSION__;
const STORAGE_KEY = `tron.release-notes.acknowledged.${VERSION}`;

export function ReleaseNotesDialog() {
  const { t, i18n } = useTranslation("pages");
  const locale = (i18n.resolvedLanguage ?? i18n.language).startsWith("zh") ? "zh-CN" : "en-US";
  const [showHistory, setShowHistory] = useState(false);
  const title = showHistory
    ? t("update.releaseNotes.historyTitle")
    : t("update.releaseNotes.title", { version: VERSION });
  const entries = __TRON_RELEASE_NOTES__[locale].filter((entry) => showHistory || entry.version === VERSION);
  const bridge = useMemo(() => new DesktopBridge(), []);
  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveFailed, setSaveFailed] = useState(false);

  useEffect(() => {
    let active = true;
    let manuallyOpened = false;
    const handleOpen = () => {
      manuallyOpened = true;
      setShowHistory(true);
      setSaveFailed(false);
      setOpen(true);
    };
    window.addEventListener(OPEN_RELEASE_NOTES_EVENT, handleOpen);
    async function loadAcknowledgement() {
      try {
        const acknowledged = bridge.isElectron()
          ? await bridge.isReleaseNotesAcknowledged(VERSION)
          : window.localStorage.getItem(STORAGE_KEY) === "true";
        if (active && !manuallyOpened) setOpen(!acknowledged);
      } catch (error) {
        console.warn("Unable to read release notes acknowledgement", error);
        if (active && !manuallyOpened) setOpen(true);
      }
    }
    void loadAcknowledgement();
    return () => {
      active = false;
      window.removeEventListener(OPEN_RELEASE_NOTES_EVENT, handleOpen);
    };
  }, [bridge]);

  async function acknowledge() {
    if (saving) return;
    setSaving(true);
    setSaveFailed(false);
    try {
      if (bridge.isElectron()) {
        await bridge.acknowledgeReleaseNotes(VERSION);
      } else {
        window.localStorage.setItem(STORAGE_KEY, "true");
      }
      setOpen(false);
    } catch (error) {
      console.warn("Unable to save release notes acknowledgement", error);
      setSaveFailed(true);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => { if (!saving) setOpen(nextOpen); }}>
      <DialogContent
        aria-describedby={undefined}
        className="fixed left-1/2 top-1/2 flex max-h-[min(40rem,calc(100dvh-2rem))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden sm:max-w-xl"
        showCloseButton={!saving}
      >
        <DialogHeader className="shrink-0 pr-8">
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div
          role="region"
          aria-label={title}
          tabIndex={0}
          className="min-h-0 overflow-y-auto overscroll-contain [scrollbar-gutter:stable]"
        >
          <div className="divide-y divide-[var(--color-outline-variant)]">
            {entries.map((entry) => (
              <section key={entry.version} className="py-4 first:pt-0 last:pb-0">
                {showHistory ? (
                  <h3 className="mb-3 text-base font-semibold">
                    {t("update.releaseNotes.title", { version: entry.version })}
                  </h3>
                ) : null}
                <MarkdownContent content={entry.content} />
              </section>
            ))}
          </div>
        </div>
        {saveFailed ? <p role="alert" className="text-destructive">{t("update.releaseNotes.saveFailed")}</p> : null}
        <DialogFooter className="shrink-0">
          <Button disabled={saving} onClick={() => { if (showHistory) setOpen(false); else void acknowledge(); }}>
            {showHistory ? t("common:actions.close") : t(saving ? "update.releaseNotes.saving" : "update.releaseNotes.acknowledge")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

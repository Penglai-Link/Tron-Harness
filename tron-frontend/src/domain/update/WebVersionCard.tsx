import { useTranslation } from "react-i18next";

import { SurfaceCard } from "@/shared";
import { Button } from "@/components/ui/button";
import { openReleaseNotes } from "./releaseNotesEvents";

export function WebVersionCard() {
  const { t } = useTranslation("pages");

  return (
    <SurfaceCard className="xl:col-span-2" data-testid="web-version-card">
      <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 text-sm">
        <span className="font-semibold text-[var(--color-on-surface)]">{t("update.currentVersion")}</span>
        <div className="flex flex-wrap items-center gap-3">
          <span className="font-medium text-[var(--color-on-surface)]">{__TRON_APP_VERSION__}</span>
          <Button size="sm" variant="outline" onClick={openReleaseNotes}>
            {t("update.releaseNotes.view")}
          </Button>
        </div>
      </div>
    </SurfaceCard>
  );
}

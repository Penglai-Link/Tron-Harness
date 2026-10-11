import { OnlineSkinMarket } from "./OnlineSkinMarket";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { InfoTooltip } from "@/components/info-tooltip";
import { useToast } from "@/components/ui/toast";
import { SurfaceCard, useTheme } from "@/shared";
import type { Skin } from "@/shared/theme/Skin";

export function SkinSettings() {
  const { t } = useTranslation("pages");
  const { t: shellT } = useTranslation("shell");
  const { skins, activeSkin, loading, error, selectSkin, importSkin, uninstallSkin, refreshSkins } = useTheme();
  const input = useRef<HTMLInputElement>(null);
  const toast = useToast();
  const [skinToUninstall, setSkinToUninstall] = useState<Skin | null>(null);
  const label = (skin: Skin) => skin.id === "light" ? shellT("theme.options.light")
    : skin.id === "matrix-dark" ? shellT("theme.options.matrixDark") : skin.name;

  async function select(id: string) {
    try { await selectSkin(id); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("settingsAppearance.skinFailed")); }
  }
  async function install(file: File) {
    if (file.size > 20 * 1024 * 1024) { toast.error(t("settingsAppearance.skinTooLarge")); return; }
    try {
      await importSkin(file);
      toast.success(t("settingsAppearance.skinImported"));
    } catch (reason) { toast.error(reason instanceof Error ? reason.message : t("settingsAppearance.skinFailed")); }
  }
  async function uninstall() {
    if (!skinToUninstall) return;
    try {
      await uninstallSkin(skinToUninstall.id);
      toast.success(t("settingsAppearance.skinUninstalled", { name: label(skinToUninstall) }));
      setSkinToUninstall(null);
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("settingsAppearance.skinUninstallFailed"));
    }
  }
  return (
    <SurfaceCard className="space-y-4" aria-busy={loading}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">{t("settingsAppearance.skins")}</h2>
          <InfoTooltip ariaLabel={t("settingsAppearance.skinHint")}>{t("settingsAppearance.skinHint")}</InfoTooltip>
        </div>
        <Button variant="outline" disabled={loading} onClick={() => input.current?.click()}>
          {t("settingsAppearance.importSkin")}
        </Button>
        <input ref={input} type="file" accept=".zip,.json" className="hidden" aria-label={t("settingsAppearance.importSkin")}
          onChange={(event) => {
            const file = event.target.files?.[0];
            event.target.value = "";
            if (file) void install(file);
          }} />
      </div>
      {error ? <div className="flex items-center gap-3 text-sm" role="alert">
        <span>{t("settingsAppearance.skinLoadFailed")}</span>
        <Button variant="ghost" disabled={loading} onClick={() => void refreshSkins()}>{t("settingsAppearance.retrySkins")}</Button>
      </div> : null}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3" aria-label={t("settingsAppearance.skins")}>
        {skins.map((skin) => {
          const selected = skin.id === activeSkin.id;
          const colors = { ...skin.colors, ...skin.layout };
          const dark = skin.mode === "Dark";
          const background = colors["--color-background"] ?? (dark ? "#010603" : "#fdf8f8");
          const surface = colors["--color-surface-container"] ?? (dark ? "#102017" : "#f1edec");
          const foreground = colors["--color-on-surface"] ?? (dark ? "#e4ffe8" : "#1c1b1b");
          const accent = colors["--color-primary"] ?? (dark ? "#5cff85" : "#000000");
          const imported = !["light", "matrix-dark"].includes(skin.id);
          return <div key={skin.id} data-theme-anchor="card" className="relative min-w-0">
            <Button variant="outline" aria-label={label(skin)} aria-pressed={selected}
              disabled={loading} onClick={() => void select(skin.id)}
              className="h-auto w-full min-w-0 flex-col items-stretch gap-0 overflow-hidden rounded-lg px-2 text-left aria-pressed:border-ring aria-pressed:ring-1 aria-pressed:ring-ring">
              <span className="flex h-28 gap-3 p-3" style={{ background, color: foreground }} aria-hidden="true">
                <span className="flex w-1/4 flex-col gap-2 rounded p-2" style={{ background: surface }}>
                  <span className="h-2 w-3/4 rounded" style={{ background: accent }} />
                  <span className="h-1 w-full rounded opacity-30" style={{ background: foreground }} />
                  <span className="h-1 w-2/3 rounded opacity-30" style={{ background: foreground }} />
                </span>
                <span className="flex flex-1 flex-col gap-2 py-1">
                  <span className="h-2 w-1/3 rounded opacity-80" style={{ background: foreground }} />
                  <span className="h-6 w-full rounded" style={{ background: surface }} />
                  <span className="mt-auto h-4 w-12 rounded" style={{ background: accent }} />
                </span>
              </span>
              <span className="flex min-w-0 items-center justify-between gap-2 px-3 py-2.5">
                <span className="truncate">{label(skin)}</span>

              </span>
            </Button>
            {imported ? <Button
              variant="ghost"
              size="sm"
              aria-label={t("settingsAppearance.uninstallSkin", { name: label(skin) })}
              title={t("settingsAppearance.uninstallSkin", { name: label(skin) })}
              disabled={loading}
              onClick={() => setSkinToUninstall(skin)}
              className="absolute right-2 top-2 z-10 border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)]/90 text-[var(--color-error)] shadow-sm hover:bg-[var(--color-elevated-surface)]"
            >{t("settingsAppearance.uninstallSkin", { name: label(skin) })}</Button> : null}
          </div>;
        })}
      </div>
      <OnlineSkinMarket onUninstall={setSkinToUninstall} />
      <ConfirmDialog
        open={Boolean(skinToUninstall)}
        title={t("settingsAppearance.uninstallSkinTitle")}
        description={skinToUninstall ? t("settingsAppearance.uninstallSkinDescription", { name: label(skinToUninstall) }) : undefined}
        cancelText={t("settingsAppearance.cancelUninstall")}
        confirmText={t("settingsAppearance.confirmUninstall")}
        pending={loading}
        onCancel={() => setSkinToUninstall(null)}
        onConfirm={() => void uninstall()}
      />
    </SurfaceCard>
  );
}

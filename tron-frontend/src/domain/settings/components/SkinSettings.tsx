import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Check, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { SurfaceCard, useTheme } from "@/shared";
import type { Skin } from "@/shared/theme/Skin";
import { SkinPreview } from "./SkinPreview";

export function SkinSettings() {
  const { t } = useTranslation("pages");
  const { t: shellT } = useTranslation("shell");
  const { skins, activeSkin, loading, error, selectSkin, importSkin, refreshSkins } = useTheme();
  const input = useRef<HTMLInputElement>(null);
  const toast = useToast();
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
  return (
    <SurfaceCard className="space-y-4" aria-busy={loading}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-sm font-semibold">{t("settingsAppearance.skins")}</h2>
        <Button variant="outline" disabled={loading} onClick={() => input.current?.click()}>
          <Upload aria-hidden="true" />{t("settingsAppearance.importSkin")}
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
          return <Button key={skin.id} variant="outline" aria-label={label(skin)} aria-pressed={selected}
            disabled={loading} onClick={() => void select(skin.id)}
            className="h-auto min-w-0 flex-col items-stretch gap-0 overflow-hidden rounded-lg p-0 text-left aria-pressed:border-ring aria-pressed:ring-1 aria-pressed:ring-ring">
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
              {selected ? <Check className="size-4 shrink-0" aria-hidden="true" /> : null}
            </span>
          </Button>;
        })}
      </div>
      <p className="text-xs text-muted-foreground">{t("settingsAppearance.skinHint")}</p>
      <SkinPreview />
    </SurfaceCard>
  );
}

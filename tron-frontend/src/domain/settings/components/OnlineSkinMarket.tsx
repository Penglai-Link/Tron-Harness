import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, Palette } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { useTheme } from "@/shared";
import { skinApi, type MarketSkinPage } from "@/shared/theme/skinApi";
import type { Skin } from "@/shared/theme/Skin";

export function OnlineSkinMarket({ onUninstall }: { onUninstall: (skin: Skin) => void }) {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const { skins, activeSkin, loading, installMarketSkin, selectSkin } = useTheme();
  const [page, setPage] = useState(1);
  const [revision, setRevision] = useState(0);
  const [result, setResult] = useState<MarketSkinPage | null>(null);
  const [fetching, setFetching] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    setFetching(true);
    setError(null);
    void skinApi.market(page, controller.signal).then((data) => {
      if (!controller.signal.aborted) setResult(data);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    }).finally(() => { if (!controller.signal.aborted) setFetching(false); });
    return () => controller.abort();
  }, [page, revision]);

  async function install(id: string) {
    try { await installMarketSkin(id); toast.success(t("settingsAppearance.marketInstalled")); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("settingsAppearance.skinFailed")); }
  }
  async function activate(id: string) {
    try { await selectSkin(id); }
    catch (reason) { toast.error(reason instanceof Error ? reason.message : t("settingsAppearance.skinFailed")); }
  }
  return <section className="space-y-3 border-t pt-4" aria-label={t("settingsAppearance.onlineSkins")} aria-busy={fetching}>
    <div className="flex items-center justify-between gap-3">
      <h3 className="text-sm font-semibold">{t("settingsAppearance.onlineSkins")}</h3>
      <Button variant="ghost" size="sm" disabled={fetching || loading} onClick={() => setRevision((value) => value + 1)}>{t("settingsAppearance.retrySkins")}</Button>
    </div>
    {fetching ? <p className="text-sm text-muted-foreground">{t("settingsAppearance.marketLoading")}</p>
      : error ? <p role="alert" className="text-sm text-destructive">{t("settingsAppearance.marketFailed")}: {error}</p>
      : !result?.records.length ? <p className="text-sm text-muted-foreground">{t("settingsAppearance.marketEmpty")}</p>
      : <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
        {result.records.map((item) => {
          const installed = skins.find((skin) => skin.marketId === item.id);
          const selected = installed?.id === activeSkin.id;
          return <article key={item.id} className="min-w-0 overflow-hidden rounded-lg border">
            {item.previewUrl ? <img src={item.previewUrl} alt="" referrerPolicy="no-referrer" className="aspect-video w-full object-cover" />
              : <div className="flex aspect-video items-center justify-center bg-muted"><Palette className="size-8 text-muted-foreground" aria-hidden="true" /></div>}
            <div className="flex items-center gap-3 px-3 py-2">
              <div className="flex min-w-0 flex-1 items-baseline gap-2">
                <h4 className="truncate text-sm font-medium" title={item.name}>{item.name}</h4>
                {item.author ? <p className="max-w-[40%] truncate text-xs text-muted-foreground" title={item.author}>{item.author}</p> : null}
              </div>
              <div className="flex shrink-0 items-center gap-1">
                {installed ? <>
                  <Button variant="outline" size="sm" disabled={loading || selected} onClick={() => void activate(installed.id)}>{t(selected ? "settingsAppearance.marketActive" : "settingsAppearance.marketApply")}</Button>
                  <Button variant="ghost" size="sm" disabled={loading} onClick={() => onUninstall(installed)}>{t("settingsAppearance.confirmUninstall")}</Button>
                </> : <Button variant="outline" size="sm" disabled={loading} onClick={() => void install(item.id)}><Download aria-hidden="true" />{t("settingsAppearance.marketInstall")}</Button>}
              </div>
            </div>
          </article>;
        })}
      </div>}
    {result && result.total > result.size ? <div className="flex items-center justify-end gap-3 text-sm">
      <Button variant="ghost" size="sm" disabled={fetching || page <= 1} onClick={() => setPage((value) => value - 1)}>{t("settingsAppearance.marketPrevious")}</Button>
      <span>{page} / {Math.ceil(result.total / result.size)}</span>
      <Button variant="ghost" size="sm" disabled={fetching || page * result.size >= result.total} onClick={() => setPage((value) => value + 1)}>{t("settingsAppearance.marketNext")}</Button>
    </div> : null}
  </section>;
}

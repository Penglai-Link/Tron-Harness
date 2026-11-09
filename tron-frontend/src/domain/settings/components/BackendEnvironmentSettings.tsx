import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Select, SelectItem, SelectPopup, SelectTrigger, SelectValue } from "@/components/ui/select";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { DesktopBridge, type BackendEnvironmentState, type WslDistribution } from "@/shared/desktop/DesktopBridge";

const errorCodes = ["wsl-not-installed", "wsl2-distribution-required", "wsl-unsupported-distribution", "wsl-linux-path-required", "wsl-version-mismatch", "wsl-package-not-published", "wsl-localhost-forwarding-unavailable", "wsl-backend-already-running"] as const;

export function BackendEnvironmentSettings() {
  const { t } = useTranslation("pages");
  const bridge = useMemo(() => new DesktopBridge(), []);
  const [environment, setEnvironment] = useState<BackendEnvironmentState | null>(null);
  const [distributions, setDistributions] = useState<WslDistribution[]>([]);
  const [choice, setChoice] = useState("local");
  const [busy, setBusy] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    void bridge.getBackendEnvironment().then(async (value) => {
      if (!active) return;
      setEnvironment(value);
      setChoice(value.selection.mode === "wsl" ? `wsl:${value.selection.distribution}` : "local");
      if (value.supported) {
        try { const list = await bridge.listWslDistributions(); if (active) setDistributions(list); }
        catch (reason) { if (active) setError(String(reason)); }
      }
    }).catch((reason) => { if (active) setError(String(reason)); });
    return () => { active = false; };
  }, [bridge]);

  useEffect(() => {
    if (!busy) return;
    let active = true;
    const timer = window.setInterval(() => {
      void bridge.getBackendEnvironment().then((value) => { if (active) setEnvironment(value); }).catch(() => {});
    }, 1000);
    return () => { active = false; window.clearInterval(timer); };
  }, [bridge, busy]);

  if (!environment?.supported) return null;
  const current = environment.selection.mode === "wsl" ? `WSL: ${environment.selection.distribution}` : t("backendEnvironment.local");
  const items = [
    { value: "local", label: t("backendEnvironment.local") },
    ...distributions.map((item) => ({ value: `wsl:${item.name}`, label: `${item.name} · WSL ${item.wslVersion || "?"}` })),
  ];
  const selected = distributions.find((item) => `wsl:${item.name}` === choice);
  const unchanged = choice === (environment.selection.mode === "wsl" ? `wsl:${environment.selection.distribution}` : "local");
  const errorText = error || environment.status.message || "";
  const errorCode = errorCodes.find((code) => errorText.includes(code));

  async function refresh() {
    setRefreshing(true);
    setError("");
    try { setDistributions(await bridge.listWslDistributions()); }
    catch (reason) { setError(String(reason)); }
    finally { setRefreshing(false); }
  }

  async function connect() {
    setBusy(true);
    setError("");
    try {
      await bridge.selectBackendEnvironment(choice === "local" ? { mode: "local" } : { mode: "wsl", distribution: choice.slice(4) });
    } catch (reason) { setError(String(reason)); setBusy(false); }
  }

  return <SurfaceCard className="space-y-4">
    <h2 className="text-sm font-semibold">{t("backendEnvironment.title")}</h2>
    <p className="text-sm text-muted-foreground">{t("backendEnvironment.current", { environment: current })}</p>
    <p className="text-sm text-muted-foreground">{t("backendEnvironment.description")}</p>
    <Select value={choice} onValueChange={(value) => value && setChoice(value)} items={items} disabled={busy}>
      <SelectTrigger aria-label={t("backendEnvironment.title")}><SelectValue /></SelectTrigger>
      <SelectPopup>{items.map((item) => <SelectItem key={item.value} value={item.value} disabled={item.value !== "local" && distributions.find((distro) => `wsl:${distro.name}` === item.value)?.wslVersion !== 2}>{item.label}</SelectItem>)}</SelectPopup>
    </Select>
    {!distributions.length && <p className="text-sm text-muted-foreground">{t("backendEnvironment.noDistributions")}</p>}
    <p className="text-sm text-muted-foreground">{t("backendEnvironment.switchNotice")}</p>
    {environment.status.dataDir && <p className="break-all font-mono text-xs">{environment.status.dataDir}</p>}
    {busy && <p role="status" className="text-sm">{t(`backendEnvironment.phases.${environment.status.phase}`)}{environment.status.progress !== undefined ? ` ${environment.status.progress}%` : ""}</p>}
    {errorText && <p role="alert" className="break-words text-sm text-destructive">{errorCode ? t(`backendEnvironment.errors.${errorCode}`) : errorText}</p>}
    <div className="flex flex-wrap gap-2">
      <Button onClick={() => void connect()} disabled={busy || unchanged || (choice !== "local" && selected?.wslVersion !== 2)}>{t("backendEnvironment.connect")}</Button>
      <Button variant="outline" disabled={busy || refreshing} onClick={() => void refresh()}>{t("backendEnvironment.refresh")}</Button>
    </div>
  </SurfaceCard>;
}

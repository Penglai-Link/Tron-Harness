import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Sparkles } from "lucide-react";
import { InfoTooltip } from "@/components/info-tooltip";
import { Button } from "@/components/ui/button";
import { Select, SelectItem, SelectPopup, SelectTrigger, SelectValue } from "@/components/ui/select";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { DesktopBridge, type BackendEnvironmentState, type WslDistribution } from "@/shared/desktop/DesktopBridge";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { useCreateChatTab } from "@/shared/layout/chat/hooks/useCreateChatTab";

const errorCodes = ["wsl-not-installed", "wsl2-distribution-required", "wsl-unsupported-distribution", "wsl-linux-path-required", "wsl-version-mismatch", "wsl-package-not-published", "wsl-localhost-forwarding-unavailable", "wsl-backend-already-running"] as const;

export function BackendEnvironmentSettings() {
  const { t } = useTranslation("pages");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspaceCode = (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const { createWithMessage } = useCreateChatTab(workspaceCode);
  const bridge = useMemo(() => new DesktopBridge(), []);
  const [environment, setEnvironment] = useState<BackendEnvironmentState | null>(null);
  const [distributions, setDistributions] = useState<WslDistribution[]>([]);
  const [choice, setChoice] = useState("local");
  const [busy, setBusy] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<{ message: string; distribution?: string; phase: string } | null>(null);
  const lastStatus = useRef<BackendEnvironmentState["status"] | null>(null);

  useEffect(() => {
    let active = true;
    void bridge.getBackendEnvironment().then(async (value) => {
      if (!active) return;
      setEnvironment(value);
      lastStatus.current = value.status;
      setChoice(value.selection.mode === "wsl" ? `wsl:${value.selection.distribution}` : "local");
      if (value.supported) {
        try { const list = await bridge.listWslDistributions(); if (active) setDistributions(list); }
        catch (reason) { if (active) setError({ message: String(reason), phase: "refresh" }); }
      }
    }).catch((reason) => { if (active) setError({ message: String(reason), phase: "checking" }); });
    return () => { active = false; };
  }, [bridge]);

  useEffect(() => {
    return bridge.onBackendEnvironmentStatus((status) => {
      if (status.phase === "error") {
        setError({ message: status.message ?? "", distribution: status.distribution, phase: lastStatus.current?.phase ?? "error" });
      } else {
        setError(null);
      }
      lastStatus.current = status;
      setEnvironment((current) => current ? { ...current, status } : current);
    });
  }, [bridge]);

  if (!environment?.supported) return null;
  const current = environment.selection.mode === "wsl" ? `WSL: ${environment.selection.distribution}` : t("backendEnvironment.local");
  const items = [
    { value: "local", label: t("backendEnvironment.local") },
    ...distributions.map((item) => ({ value: `wsl:${item.name}`, label: `${item.name} · WSL ${item.wslVersion || "?"}` })),
  ];
  const selected = distributions.find((item) => `wsl:${item.name}` === choice);
  const unchanged = choice === (environment.selection.mode === "wsl" ? `wsl:${environment.selection.distribution}` : "local");
  const errorText = error?.message || (environment.status.phase === "error" ? environment.status.message : "") || "";
  const errorCode = errorCodes.find((code) => errorText.includes(code));

  async function refresh() {
    setRefreshing(true);
    setError(null);
    try { setDistributions(await bridge.listWslDistributions()); }
    catch (reason) { setError({ message: String(reason), phase: "refresh" }); }
    finally { setRefreshing(false); }
  }

  async function connect() {
    setBusy(true);
    setError(null);
    try {
      await bridge.selectBackendEnvironment(choice === "local" ? { mode: "local" } : { mode: "wsl", distribution: choice.slice(4) });
    } catch (reason) {
      setError((current) => current ?? { message: String(reason), distribution: choice.startsWith("wsl:") ? choice.slice(4) : undefined, phase: "checking" });
      setBusy(false);
    }
  }

  function analyze() {
    if (!environment || !errorText || !workspaceCode) return;
    const diagnostics = {
      capturedAt: new Date().toISOString(),
      currentEnvironment: environment.selection,
      failedDistribution: error?.distribution ?? environment.status.distribution ?? null,
      failedPhase: error?.phase ?? environment.status.phase,
      distributions,
      backendVersion: environment.status.backendVersion ?? null,
      architecture: environment.status.architecture ?? null,
      dataDirectory: environment.status.dataDir ?? null,
      error: errorText,
    };
    createWithMessage(t("backendEnvironment.analysisPrompt", { diagnostics: JSON.stringify(diagnostics, null, 2) }));
  }

  return <SurfaceCard className="space-y-4">
    <div className="flex items-center gap-1.5">
      <h2 className="text-sm font-semibold">{t("backendEnvironment.title")}</h2>
      <InfoTooltip ariaLabel={t("backendEnvironment.title")}>
        <div className="space-y-2">
          <p>{t("backendEnvironment.description")}</p>
          <p>{t("backendEnvironment.switchNotice")}</p>
        </div>
      </InfoTooltip>
    </div>
    <p className="text-sm text-muted-foreground">{t("backendEnvironment.current", { environment: current })}</p>
    <Select value={choice} onValueChange={(value) => value && setChoice(value)} items={items} disabled={busy}>
      <SelectTrigger aria-label={t("backendEnvironment.title")}><SelectValue /></SelectTrigger>
      <SelectPopup>{items.map((item) => <SelectItem key={item.value} value={item.value} disabled={item.value !== "local" && distributions.find((distro) => `wsl:${distro.name}` === item.value)?.wslVersion !== 2}>{item.label}</SelectItem>)}</SelectPopup>
    </Select>
    {!distributions.length && <p className="text-sm text-muted-foreground">{t("backendEnvironment.noDistributions")}</p>}
    {environment.status.dataDir && <p className="break-all font-mono text-xs">{environment.status.dataDir}</p>}
    {busy && <p role="status" className="text-sm">{t(`backendEnvironment.phases.${environment.status.phase}`)}{environment.status.progress !== undefined ? ` ${environment.status.progress}%` : ""}</p>}
    {errorText && <div className="space-y-2">
      <p role="alert" className="break-words text-sm text-destructive">{errorCode ? t(`backendEnvironment.errors.${errorCode}`) : errorText}</p>
      <Button variant="outline" onClick={analyze} disabled={busy || refreshing || !workspaceCode}>
        <Sparkles aria-hidden="true" />{t("backendEnvironment.analyze")}
      </Button>
    </div>}
    <div className="flex flex-wrap gap-2">
      <Button onClick={() => void connect()} disabled={busy || unchanged || (choice !== "local" && selected?.wslVersion !== 2)}>{t("backendEnvironment.connect")}</Button>
      <Button variant="outline" disabled={busy || refreshing} onClick={() => void refresh()}>{t("backendEnvironment.refresh")}</Button>
    </div>
  </SurfaceCard>;
}

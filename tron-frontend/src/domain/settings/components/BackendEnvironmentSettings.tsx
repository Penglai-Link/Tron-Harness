import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { LoaderCircle, Sparkles } from "lucide-react";
import { InfoTooltip } from "@/components/info-tooltip";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useToast } from "@/components/ui/toast";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import { DesktopBridge, type BackendEnvironmentState, type WslDistribution } from "@/shared/desktop/DesktopBridge";
import { useCreateChatTab } from "@/shared/layout/chat/hooks/useCreateChatTab";
import { useAppShellStore } from "@/store/appShellStore";
import { useTabStore } from "@/store/tabStore";

const errorCodes = ["wsl-uninstall-unsafe-path", "wsl-operation-in-progress", "wsl-uninstall-local-only", "wsl-not-installed", "wsl2-distribution-required", "wsl-unsupported-distribution", "wsl-linux-path-required", "wsl-version-mismatch", "wsl-package-not-published", "wsl-insecure-download-url", "wsl-localhost-forwarding-unavailable", "wsl-backend-already-running"] as const;

export function BackendEnvironmentSettings() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  // Settings renders outside Shell's workspace context. Keep the chat in the
  // workspace the user was using before opening settings.
  const tabWorkspaceCode = useTabStore((state) => state.workspaceCode);
  const selectedWorkspaceCode = useAppShellStore((state) => state.selectedSecondarySpaceId || state.selectedPrimarySpaceId);
  const workspaceCode = tabWorkspaceCode || selectedWorkspaceCode;
  const { createWithMessage } = useCreateChatTab(workspaceCode);
  const bridge = useMemo(() => new DesktopBridge(), []);
  const [environment, setEnvironment] = useState<BackendEnvironmentState | null>(null);
  const [distributions, setDistributions] = useState<WslDistribution[]>([]);
  const [choice, setChoice] = useState("local");
  const [busy, setBusy] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const environmentGroupId = useId();
  const [panel, setPanel] = useState<"switch" | "manage">("switch");
  const [panelOpen, setPanelOpen] = useState(false);
  const [uninstallTarget, setUninstallTarget] = useState<string | null>(null);
  const [deleteUserData, setDeleteUserData] = useState(false);
  const [error, setError] = useState<{ message: string; distribution?: string; phase: string } | null>(null);
  const lastStatus = useRef<BackendEnvironmentState["status"] | null>(null);
  const dialogTitleRef = useRef<HTMLHeadingElement>(null);
  const deleteDataRef = useRef<HTMLInputElement>(null);
  const uninstallButtons = useRef(new Map<string, HTMLButtonElement>());
  const previousTarget = useRef<string | null>(null);

  useEffect(() => {
    if (panelOpen) {
      if (uninstallTarget) deleteDataRef.current?.focus();
      else if (previousTarget.current) (uninstallButtons.current.get(previousTarget.current) ?? dialogTitleRef.current)?.focus();
      else dialogTitleRef.current?.focus();
    }
    previousTarget.current = uninstallTarget;
  }, [panelOpen, uninstallTarget]);

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
  const current = environment.selection.mode === "wsl" ? environment.selection.distribution : t("backendEnvironment.local");
  const items = [
    { value: "local", label: t("backendEnvironment.local") },
    ...distributions.map((item) => ({ value: `wsl:${item.name}`, label: item.name })),
  ];
  const installedDistributions = distributions.filter((item) => item.tronInstalled);
  const canManageWsl = environment.selection.mode === "local";
  const selected = distributions.find((item) => `wsl:${item.name}` === choice);
  const activeChoice = environment.selection.mode === "wsl" ? `wsl:${environment.selection.distribution}` : "local";
  const unchanged = choice === activeChoice;
  const errorText = error?.message || (environment.status.phase === "error" ? environment.status.message : "") || "";
  const errorCode = errorCodes.find((code) => errorText.includes(code));

  async function refresh() {
    setRefreshing(true);
    setError(null);
    try {
      const list = await bridge.listWslDistributions();
      setDistributions(list);
      setChoice((value) => value === "local" || list.some((item) => `wsl:${item.name}` === value) ? value : "local");
    }
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

  async function uninstall() {
    if (!uninstallTarget || environment?.selection.mode !== "local") return;
    setBusy(true);
    setError(null);
    try {
      await bridge.uninstallWslBackend(uninstallTarget, deleteUserData);
      setDistributions((items) => items.map((item) => item.name === uninstallTarget ? { ...item, tronInstalled: false } : item));
      toast.success(t(deleteUserData ? "backendEnvironment.uninstallDataDeleted" : "backendEnvironment.uninstallSuccess", { distribution: uninstallTarget }));
      setUninstallTarget(null);
    } catch (reason) {
      setError({ message: String(reason), distribution: uninstallTarget, phase: "uninstalling" });
      setUninstallTarget(null);
    } finally { setBusy(false); }
  }

  function analyze() {
    if (!environment || !errorText) return;
    if (!workspaceCode) {
      toast.info(t("backendEnvironment.analysisWorkspaceRequired"));
      return;
    }
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

  const feedback = <>
    {busy && <p role="status" className="flex items-center gap-2 text-xs text-muted-foreground">
      <LoaderCircle aria-hidden="true" className="size-3.5 animate-spin motion-reduce:animate-none" />
      {t(`backendEnvironment.phases.${environment.status.phase}`)}{environment.status.progress !== undefined ? ` ${environment.status.progress}%` : ""}
    </p>}
    {errorText && <div className="flex flex-wrap items-center justify-between gap-2">
      <p role="alert" className="min-w-0 break-words text-sm text-destructive">{errorCode ? t(`backendEnvironment.errors.${errorCode}`) : errorText}</p>
      <Button variant="outline" size="sm" onClick={analyze} disabled={busy || refreshing}>
        <Sparkles aria-hidden="true" />{t("backendEnvironment.analyze")}
      </Button>
    </div>}
  </>;
  const refreshButton = <Button variant="outline" disabled={busy || refreshing || uninstallTarget !== null} aria-busy={refreshing} onClick={() => void refresh()}>
    {refreshing && <LoaderCircle aria-hidden="true" className="animate-spin motion-reduce:animate-none" />}
    {t("backendEnvironment.refresh")}
  </Button>;

  const dialogHint = t(panel === "switch" ? "backendEnvironment.switchNotice" : "backendEnvironment.installedWslHint");

  return <SurfaceCard className="space-y-4">
    <div className="flex items-center gap-1">
      <h2 className="text-sm font-semibold">{t("backendEnvironment.title")}</h2>
      <InfoTooltip ariaLabel={t("backendEnvironment.title")}>
        <div className="space-y-2">
          <p>{t("backendEnvironment.description")}</p>
          <p>{t(environment.selection.mode === "local" ? "backendEnvironment.localHint" : "backendEnvironment.wslHint")}</p>
          {environment.status.dataDir && <p className="break-all">{t("backendEnvironment.dataDirectory")}: {environment.status.dataDir}</p>}
        </div>
      </InfoTooltip>
    </div>
    <dl className="flex items-baseline gap-6 text-sm">
      <dt className="shrink-0 text-muted-foreground">{t("backendEnvironment.currentLabel")}</dt>
      <dd className="min-w-0 break-words font-medium">{current}</dd>
    </dl>
    <div>
      <Button variant="outline" disabled={busy} onClick={() => { setUninstallTarget(null); setChoice(activeChoice); setPanel("switch"); setPanelOpen(true); }}>{t("backendEnvironment.configure")}</Button>
    </div>
    {!panelOpen && feedback}
    <Dialog open={panelOpen} onOpenChange={(open) => { if (!busy) setPanelOpen(open); }}>
      <DialogContent showCloseButton={!busy} className="gap-6 p-6 sm:max-w-xl">
        <DialogHeader className="pr-6">
          <div className="flex items-center gap-1">
            <DialogTitle ref={dialogTitleRef} tabIndex={-1} className="leading-relaxed outline-none">{t("backendEnvironment.title")}</DialogTitle>
            <InfoTooltip ariaLabel={t("backendEnvironment.title")}>
              <div className="space-y-2">
                <p>{dialogHint}</p>
                {!canManageWsl && <p>{t("backendEnvironment.errors.wsl-uninstall-local-only")}</p>}
                {!distributions.length && <p>{t("backendEnvironment.noDistributions")}</p>}
              </div>
            </InfoTooltip>
          </div>
          <DialogDescription className="sr-only">{dialogHint}</DialogDescription>
        </DialogHeader>
        <Tabs value={panel} onValueChange={(value) => { if (!busy && (value === "switch" || value === "manage")) { setUninstallTarget(null); setPanel(value); } }} className="gap-5">
          <TabsList className="self-start" aria-label={t("backendEnvironment.title")}>
            <TabsTrigger value="switch" disabled={busy} className="px-4">{t("backendEnvironment.switchLabel")}</TabsTrigger>
            <TabsTrigger value="manage" disabled={busy || !canManageWsl} className="px-4">{t("backendEnvironment.installationsTab")}</TabsTrigger>
          </TabsList>
          <TabsContent value="switch" className="space-y-5">
            <fieldset className="min-w-0 space-y-1">
              <legend className="sr-only">{t("backendEnvironment.switchLabel")}</legend>
              {items.map((item) => {
                const distro = distributions.find((entry) => `wsl:${entry.name}` === item.value);
                const disabled = busy || refreshing || (item.value !== "local" && distro?.wslVersion !== 2);
                return <label key={item.value} className={`flex min-h-14 items-center gap-3 rounded-lg px-3 text-sm transition-colors motion-reduce:transition-none ${disabled ? "cursor-not-allowed text-muted-foreground" : "cursor-pointer hover:bg-muted/40"} ${choice === item.value ? "bg-muted/60" : ""}`}>
                  <input type="radio" name={environmentGroupId} value={item.value} checked={choice === item.value} disabled={disabled} onChange={() => setChoice(item.value)} className="size-4 shrink-0 accent-[var(--primary)]" />
                  <span className="min-w-0 flex-1 break-words font-medium">{item.label}</span>
                  {item.value === activeChoice && <span className="shrink-0 text-xs text-muted-foreground">{t("backendEnvironment.active")}</span>}
                </label>;
              })}
            </fieldset>
            <div className="flex flex-wrap items-center gap-2">
              {refreshButton}
              <Button data-dialog-primary onClick={() => void connect()} disabled={unchanged || busy || refreshing || (choice !== "local" && selected?.wslVersion !== 2)}>{t("backendEnvironment.connect")}</Button>
            </div>
          </TabsContent>
          <TabsContent value="manage" className="space-y-5">
            {installedDistributions.length ? <ul className="space-y-1">
              {installedDistributions.map((distro) => {
                const confirming = uninstallTarget === distro.name;
                return <li key={distro.name} className={`rounded-lg px-3 py-3 transition-colors motion-reduce:transition-none ${confirming ? "bg-muted/50" : ""}`}>
                  <div className="flex min-h-8 items-center justify-between gap-4">
                    <div className="flex min-w-0 items-center gap-1">
                      <span className="min-w-0 break-words text-sm font-medium">{distro.name}</span>
                      {confirming && <InfoTooltip ariaLabel={t("backendEnvironment.uninstallTitle", { distribution: distro.name })}><p>{t("backendEnvironment.uninstallNotice")}</p></InfoTooltip>}
                    </div>
                    {!confirming && <Button variant="outline" disabled={busy || refreshing} ref={(node) => { if (node) uninstallButtons.current.set(distro.name, node); else uninstallButtons.current.delete(distro.name); }} onClick={() => { setDeleteUserData(false); setUninstallTarget(distro.name); }} aria-expanded={false} aria-label={t("backendEnvironment.uninstallTitle", { distribution: distro.name })}>{t("backendEnvironment.uninstallAction")}</Button>}
                  </div>
                  {confirming && <div role="group" aria-label={t("backendEnvironment.uninstallTitle", { distribution: distro.name })} className="space-y-4 pt-3 animate-in fade-in-0 duration-150 motion-reduce:animate-none">
                    <div className="flex items-center gap-1">
                      <label className="flex min-h-9 cursor-pointer items-center gap-2 text-sm">
                        <input ref={deleteDataRef} className="size-4 shrink-0 accent-[var(--primary)]" type="checkbox" checked={deleteUserData} disabled={busy} onChange={(event) => setDeleteUserData(event.target.checked)} />
                        {t("backendEnvironment.deleteUserData")}
                      </label>
                      <InfoTooltip ariaLabel={t("backendEnvironment.deleteUserData")}>
                        <div className="space-y-2"><p>{t("backendEnvironment.deleteUserDataHint")}</p><p>{t("backendEnvironment.deleteUserDataWarning")}</p></div>
                      </InfoTooltip>
                    </div>
                    <div className="flex flex-wrap justify-end gap-2">
                      <Button variant="outline" disabled={busy} onClick={() => setUninstallTarget(null)}>{t("backendEnvironment.cancel")}</Button>
                      <Button variant="destructive" disabled={busy} aria-busy={busy} data-dialog-primary onClick={() => void uninstall()}>
                        {busy && <LoaderCircle aria-hidden="true" className="animate-spin motion-reduce:animate-none" />}
                        {t(busy ? "backendEnvironment.phases.uninstalling" : deleteUserData ? "backendEnvironment.uninstallAndDelete" : "backendEnvironment.confirmUninstall")}
                      </Button>
                    </div>
                  </div>}
                </li>;
              })}
            </ul> : <p className="py-6 text-sm text-muted-foreground">{t("backendEnvironment.noInstallations")}</p>}
            <div className="flex">{refreshButton}</div>
          </TabsContent>
        </Tabs>
        {feedback}
      </DialogContent>
    </Dialog>
  </SurfaceCard>;
}

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, LoaderCircle, QrCode, RefreshCw, Smartphone } from "lucide-react";
import { QRCodeSVG } from "qrcode.react";

import { Button } from "@/components/ui/button";
import { Field, TextInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { Switch } from "@/components/ui/switch";
import { useToast } from "@/components/ui/toast";
import { SurfaceCard } from "@/shared/layout/SurfaceCard";
import {
  getRemoteAccessStatus,
  getServiceDevices,
  getServicePairing,
  refreshServicePairing,
  revokeServiceDevice,
  updateRemoteAccess,
  type RemoteAccessStatus,
  type ServiceDevice,
  type ServicePairing,
} from "@/domain/settings/api/remoteAccessApi";

export function RemoteAccessSettings() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const [status, setStatus] = useState<RemoteAccessStatus | null>(null);
  const [publicBaseUrl, setPublicBaseUrl] = useState("");
  const [machineName, setMachineName] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [pairing, setPairing] = useState<ServicePairing | null>(null);
  const [devices, setDevices] = useState<ServiceDevice[]>([]);
  const [pairingLoading, setPairingLoading] = useState(false);

  function applyStatus(next: RemoteAccessStatus) {
    setStatus(next);
    setPublicBaseUrl(next.publicBaseUrl);
    setMachineName(next.machineName);
  }

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    void getRemoteAccessStatus(controller.signal)
      .then(applyStatus)
      .catch((reason) => {
        if (!controller.signal.aborted) {
          toast.error(t("settingsGeneral.remoteAccess.loadFailed"));
          console.error(reason);
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    void getServicePairing(controller.signal)
      .then(setPairing)
      .catch((reason) => {
        if (!controller.signal.aborted) console.error(reason);
      });
    void getServiceDevices(controller.signal)
      .then(setDevices)
      .catch((reason) => {
        if (!controller.signal.aborted) console.error(reason);
      });
    return () => controller.abort();
  }, [t, toast]);

  async function refreshPairing() {
    setPairingLoading(true);
    try {
      setPairing(await refreshServicePairing());
      toast.success(t("settingsGeneral.remoteAccess.refreshPairingDone"));
    } catch (reason) {
      toast.error(t("settingsGeneral.remoteAccess.refreshPairingFailed"));
      console.error(reason);
    } finally {
      setPairingLoading(false);
    }
  }

  async function revokeDevice(device: ServiceDevice) {
    if (!window.confirm(t("settingsGeneral.remoteAccess.revokeConfirm"))) return;
    try {
      await revokeServiceDevice(device.deviceId);
      setDevices(await getServiceDevices());
      toast.success(t("settingsGeneral.remoteAccess.revoked"));
    } catch (reason) {
      toast.error(t("settingsGeneral.remoteAccess.revokeFailed"));
      console.error(reason);
    }
  }

  async function save(next: {
    enabled?: boolean;
    machineName?: string;
    publicBaseUrl?: string;
  }) {
    if (!status) return;
    setSaving(true);
    try {
      const result = await updateRemoteAccess({
        enabled: next.enabled ?? status.enabled,
        machineName: next.machineName ?? machineName,
        publicBaseUrl: next.publicBaseUrl ?? publicBaseUrl,
      });
      applyStatus(result);
      if (result.restartRequired) {
        toast.success(t("settingsGeneral.remoteAccess.savedRestart"));
      } else {
        toast.success(t("settingsGeneral.remoteAccess.saved"));
      }
    } catch (reason) {
      toast.error(t("settingsGeneral.remoteAccess.saveFailed"));
      console.error(reason);
    } finally {
      setSaving(false);
    }
  }

  async function copy(value: string, message: string) {
    try {
      await navigator.clipboard.writeText(value);
      toast.success(message);
    } catch {
      toast.error(t("settingsGeneral.remoteAccess.copyFailed"));
    }
  }

  if (loading && !status) {
    return (
      <SurfaceCard className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold text-[var(--color-on-surface)]">
          <Smartphone size={16} aria-hidden="true" />
          {t("settingsGeneral.remoteAccess.title")}
        </h2>
        <p className="flex items-center gap-2 text-sm text-[var(--color-on-surface-variant)]">
          <LoaderCircle className="animate-spin" size={14} aria-hidden="true" />
          {t("settingsGeneral.remoteAccess.loading")}
        </p>
      </SurfaceCard>
    );
  }
  if (!status) return null;

  return (
    <SurfaceCard className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 space-y-1">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-[var(--color-on-surface)]">
            <Smartphone size={16} aria-hidden="true" />
            {t("settingsGeneral.remoteAccess.title")}
            <InfoTooltip ariaLabel={t("settingsGeneral.remoteAccess.help")}>
              {t("settingsGeneral.remoteAccess.description")}
            </InfoTooltip>
          </h2>
          <p className="text-sm text-[var(--color-on-surface-variant)]">
            {t("settingsGeneral.remoteAccess.enableHint")}
          </p>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-[var(--color-on-surface-variant)]">
            {status.active
              ? t("settingsGeneral.remoteAccess.active")
              : t("settingsGeneral.remoteAccess.inactive")}
          </span>
          <Switch
            aria-label={t("settingsGeneral.remoteAccess.enable")}
            checked={status.enabled}
            disabled={saving}
            onCheckedChange={(checked) => void save({ enabled: checked })}
          />
        </div>
      </div>

      {status.restartRequired && (
        <p className="rounded-lg bg-[var(--color-warning-container,var(--color-surface-container))] px-3 py-2 text-sm text-[var(--color-on-surface)]">
          {t("settingsGeneral.remoteAccess.restartRequired")}
        </p>
      )}

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto]">
        <div className="space-y-4">
          <Field
            hint={t("settingsGeneral.remoteAccess.publicBaseUrlHint")}
            label={t("settingsGeneral.remoteAccess.publicBaseUrl")}
          >
            <TextInput
              aria-label={t("settingsGeneral.remoteAccess.publicBaseUrl")}
              disabled={saving}
              placeholder="https://tron.example.com"
              value={publicBaseUrl}
              onChange={(event) => setPublicBaseUrl(event.target.value)}
            />
          </Field>
          <Field label={t("settingsGeneral.remoteAccess.machineName")}>
            <TextInput
              aria-label={t("settingsGeneral.remoteAccess.machineName")}
              disabled={saving}
              placeholder={status.machineName}
              value={machineName}
              onChange={(event) => setMachineName(event.target.value)}
            />
          </Field>
          <div className="flex flex-wrap gap-2">
            <Button disabled={saving} onClick={() => void save({})} size="sm" type="button">
              {saving
                ? t("settingsGeneral.remoteAccess.saving")
                : t("settingsGeneral.remoteAccess.save")}
            </Button>
            <Button
              disabled={saving}
              onClick={() => void save({ publicBaseUrl: "" })}
              size="sm"
              type="button"
              variant="ghost"
            >
              {t("settingsGeneral.remoteAccess.useLanAddress")}
            </Button>
          </div>
          <dl className="space-y-2 text-sm">
            <div className="flex flex-wrap items-center gap-2">
              <dt className="w-24 shrink-0 text-[var(--color-on-surface-variant)]">
                {t("settingsGeneral.remoteAccess.product")}
              </dt>
              <dd className="font-mono">
                {status.productCode} · {status.productName}
              </dd>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <dt className="w-24 shrink-0 text-[var(--color-on-surface-variant)]">
                {t("settingsGeneral.remoteAccess.address")}
              </dt>
              <dd className="min-w-0 break-all font-mono">{status.baseUrl}</dd>
              <Button
                aria-label={t("settingsGeneral.remoteAccess.copyAddress")}
                onClick={() => void copy(status.baseUrl, t("settingsGeneral.remoteAccess.copied"))}
                size="icon-sm"
                type="button"
                variant="ghost"
              >
                <Copy size={14} aria-hidden="true" />
              </Button>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <dt className="w-24 shrink-0 text-[var(--color-on-surface-variant)]">
                {t("settingsGeneral.remoteAccess.remotePort")}
              </dt>
              <dd className="font-mono">{status.port}</dd>
            </div>
          </dl>
        </div>

      </div>

      <div className="space-y-3 border-t border-[var(--color-outline-variant)] pt-4">
        <div className="space-y-1">
          <h3 className="text-sm font-semibold text-[var(--color-on-surface)]">
            {t("settingsGeneral.remoteAccess.newPairingTitle")}
          </h3>
          <p className="text-xs text-[var(--color-on-surface-variant)]">
            {t("settingsGeneral.remoteAccess.newPairingHint")}
          </p>
        </div>
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto]">
          <div className="space-y-2 text-sm">
            <div className="flex flex-wrap items-center gap-2">
              <span className="w-24 shrink-0 text-[var(--color-on-surface-variant)]">
                {t("settingsGeneral.remoteAccess.transport")}
              </span>
              <span>
                {pairing?.transport === "public"
                  ? t("settingsGeneral.remoteAccess.transportPublic")
                  : t("settingsGeneral.remoteAccess.transportLan")}
              </span>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <span className="w-24 shrink-0 text-[var(--color-on-surface-variant)]">
                {t("settingsGeneral.remoteAccess.fingerprint")}
              </span>
              <span className="break-all font-mono text-xs">
                {pairing?.nodeKeyFingerprint || "—"}
              </span>
              {pairing?.nodeKeyFingerprint && (
                <Button
                  aria-label={t("settingsGeneral.remoteAccess.copyFingerprint")}
                  onClick={() =>
                    void copy(pairing.nodeKeyFingerprint, t("settingsGeneral.remoteAccess.copied"))
                  }
                  size="icon-sm"
                  type="button"
                  variant="ghost"
                >
                  <Copy size={14} aria-hidden="true" />
                </Button>
              )}
            </div>
            <div className="space-y-2">
              <p className="text-xs font-medium text-[var(--color-on-surface)]">
                {t("settingsGeneral.remoteAccess.devices")}
              </p>
              {devices.length === 0 ? (
                <p className="text-xs text-[var(--color-on-surface-variant)]">
                  {t("settingsGeneral.remoteAccess.noDevices")}
                </p>
              ) : (
                <ul className="space-y-2">
                  {devices.map((device) => (
                    <li
                      key={device.deviceId}
                      className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-[var(--color-outline-variant)] px-3 py-2"
                    >
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium">{device.name}</p>
                        <p className="text-xs text-[var(--color-on-surface-variant)]">
                          {device.platform} · {device.deviceId}
                          {device.revoked
                            ? ` · ${t("settingsGeneral.remoteAccess.revokedLabel")}`
                            : ""}
                        </p>
                      </div>
                      {!device.revoked && (
                        <Button
                          onClick={() => void revokeDevice(device)}
                          size="sm"
                          type="button"
                          variant="ghost"
                        >
                          {t("settingsGeneral.remoteAccess.revoke")}
                        </Button>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
          <div className="flex flex-col items-center gap-3">
            <div className="rounded-xl bg-white p-3 shadow-sm">
              {pairing?.pairingUri ? (
                <QRCodeSVG
                  aria-label={t("settingsGeneral.remoteAccess.newPairingTitle")}
                  bgColor="#ffffff"
                  fgColor="#000000"
                  level="M"
                  marginSize={2}
                  size={176}
                  value={pairing.pairingUri}
                />
              ) : (
                <div className="flex size-[176px] items-center justify-center rounded-lg bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]">
                  <QrCode size={48} aria-hidden="true" />
                </div>
              )}
            </div>
            <Button
              disabled={pairingLoading}
              onClick={() => void refreshPairing()}
              size="sm"
              type="button"
              variant="outline"
            >
              {pairingLoading ? (
                <LoaderCircle className="animate-spin" size={14} aria-hidden="true" />
              ) : (
                <RefreshCw size={14} aria-hidden="true" />
              )}
              {t("settingsGeneral.remoteAccess.refreshPairing")}
            </Button>
            {pairing?.pairingUri && (
              <Button
                onClick={() =>
                  void copy(pairing.pairingUri ?? "", t("settingsGeneral.remoteAccess.copied"))
                }
                size="sm"
                type="button"
                variant="ghost"
              >
                <Copy size={14} aria-hidden="true" />
                {t("settingsGeneral.remoteAccess.copyPairing")}
              </Button>
            )}
          </div>
        </div>
      </div>

      <p className="border-t border-[var(--color-outline-variant)] pt-3 text-xs text-[var(--color-on-surface-variant)]">
        {t("settingsGeneral.remoteAccess.securityHint")}
      </p>
    </SurfaceCard>
  );
}
import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Switch } from "@/components/ui/switch";
import { SurfaceCard } from "@/shared";
import { getToolPolicyConfig, updateToolPolicyConfig } from "../api/capabilityApi";

export function ToolPolicySettings({ revision, onSavingChange }: {
  revision: number;
  onSavingChange: (saving: boolean) => void;
}) {
  const { t } = useTranslation("pages");
  const id = useId();
  const [smart, setSmart] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const version = useRef(0);
  const pending = useRef(false);
  useEffect(() => {
    const controller = new AbortController();
    const current = ++version.current;
    setSmart(null);
    setError(null);
    setSaved(false);
    getToolPolicyConfig(controller.signal).then((config) => {
      if (!controller.signal.aborted && current === version.current) setSmart(config.smart);
    }).catch((reason: unknown) => {
      if (!controller.signal.aborted && current === version.current) setError(reason instanceof Error ? reason.message : String(reason));
    });
    return () => { controller.abort(); version.current++; };
  }, [revision]);

  const change = async (value: boolean) => {
    if (pending.current || smart === null) return;
    pending.current = true;
    const current = ++version.current;
    setSaving(true);
    onSavingChange(true);
    setSaved(false);
    setError(null);
    try {
      const config = await updateToolPolicyConfig(value);
      if (current === version.current) { setSmart(config.smart); setSaved(true); }
    } catch (reason) {
      if (current === version.current) setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      pending.current = false;
      setSaving(false);
      onSavingChange(false);
    }
  };

  return <SurfaceCard className="mb-4 flex items-center gap-6" aria-busy={saving}>
    <div className="min-w-0 flex-1 space-y-2">
      <label id={`${id}-label`} htmlFor={id} className="block text-sm font-medium">{t("capabilityManagement.smartTitle")}</label>
      <p id={`${id}-description`} className="text-sm text-[var(--color-on-surface-variant)]">{t("capabilityManagement.smartDescription")}</p>
      {smart === null && !error ? <p role="status" className="text-xs">{t("capabilityManagement.loading")}</p> : null}
      {saved ? <p role="status" className="text-xs text-[var(--color-success)]">{t("capabilityManagement.smartSaved")}</p> : null}
      {error ? <p role="alert" className="text-sm text-[var(--color-error)]">{error}</p> : null}
    </div>
    <Switch id={id} checked={smart ?? true} disabled={smart === null || saving}
      className="[--thumb-size:--spacing(6)] sm:[--thumb-size:--spacing(6)]"
      aria-labelledby={`${id}-label`} aria-describedby={`${id}-description`}
      onCheckedChange={(value) => void change(value)} />
  </SurfaceCard>;
}

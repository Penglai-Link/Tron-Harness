import { ChevronRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { SelectInput, TextInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { SurfaceCard } from "@/shared";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { SelectItem } from "@/components/ui/select";
import { useSystemConfig } from "@/domain/settings/hooks/useSystemConfig";
import type { SystemConfigValue } from "@/domain/settings/model/SystemConfig";
import { systemConfigHelp } from "../model/systemConfigHelp";
import { userSystemConfigGroups } from "../model/userSystemConfig";

function NumericConfigInput({ label, value, disabled, onChange }: { label: string; value: SystemConfigValue; disabled: boolean; onChange: (value: number | null) => void }) {
  const [draft, setDraft] = useState(value == null ? "" : String(value));
  useEffect(() => {
    const next = value == null ? "" : String(value);
    setDraft((current) => next === "" && (current === "-" || current.endsWith(".")) ? current : current && next !== "" && Number(current) === Number(next) ? current : next);
  }, [value]);
  return <TextInput aria-label={label} disabled={disabled} type="text" inputMode="decimal" value={draft} onChange={(event) => {
    const raw = event.target.value;
    if (!/^-?\d*(?:\.\d*)?$/.test(raw)) return;
    setDraft(raw);
    onChange(raw && /\d$/.test(raw) && Number.isFinite(Number(raw)) ? Number(raw) : null);
  }} />;
}

export function SystemConfigTable() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const { document, error, loading, refresh, save, saving } = useSystemConfig();
  const [changes, setChanges] = useState<Record<string, SystemConfigValue>>({});
  const [expanded, setExpanded] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(() => new Set());
  const groups = useMemo(() => {
    const byPath = new Map(document?.items.map((item) => [item.path, item]));
    return userSystemConfigGroups.map(({ id, paths }) => ({
      id,
      items: paths.flatMap((path) => {
        const item = byPath.get(path);
        return item ? [item] : [];
      }),
    })).filter(({ items }) => items.length > 0);
  }, [document]);
  const dirtyItems = groups.flatMap(({ items }) => items).filter((item) => Object.hasOwn(changes, item.path));

  function update(path: string, value: SystemConfigValue) {
    setChanges((current) => ({ ...current, [path]: value }));
  }

  function toggleGroup(group: string) {
    setExpandedGroups((current) => {
      const next = new Set(current);
      if (next.has(group)) next.delete(group);
      else next.add(group);
      return next;
    });
  }

  async function handleSave() {
    if (!document || dirtyItems.length === 0) return;
    try {
      const result = await save(dirtyItems.map((item) => ({ path: item.path, value: changes[item.path] })), false);
      setChanges({});
      const message = result?.migrationPaths.length ? "savedMigration" : result?.hotUpdatePaths.length
        ? result.restartPaths.length ? "savedMixed" : "savedHot"
        : result?.restartPaths.length ? "savedRestart" : "saved";
      toast.success(t(`settingsGeneral.systemConfig.messages.${message}`));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : t("settingsGeneral.systemConfig.messages.saveFailed"));
    }
  }

  return (
    <SurfaceCard className="space-y-4 xl:col-span-2">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-center gap-1.5">
          <button
            aria-controls="system-config-groups"
            aria-expanded={expanded}
            className="flex min-h-9 cursor-pointer items-center gap-2 rounded-md px-2 text-left text-sm font-semibold text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-hover-surface)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
            type="button"
            onClick={() => setExpanded((current) => !current)}
          >
            <ChevronRight aria-hidden="true" className={`size-4 shrink-0 transition-transform ${expanded ? "rotate-90" : ""}`} />
            <span>{t("settingsGeneral.systemConfig.title")}</span>
          </button>
          <InfoTooltip ariaLabel={t("settingsGeneral.systemConfig.help")}>{t("settingsGeneral.systemConfig.description")}</InfoTooltip>
        </div>
        <div className="flex gap-[12px]">
          <Button disabled={loading || saving} onClick={refresh} variant="secondary">{t("settingsGeneral.systemConfig.actions.refresh")}</Button>
          {dirtyItems.length > 0 && <Button disabled={saving} onClick={() => void handleSave()}>
            {saving ? t("settingsGeneral.systemConfig.actions.saving") : t("settingsGeneral.systemConfig.actions.save", { count: dirtyItems.length })}
          </Button>}
        </div>
      </div>

      {expanded && (
        <div id="system-config-groups">
          {loading && <p className="text-sm text-[var(--color-on-surface-variant)]">{t("settingsGeneral.systemConfig.loading")}</p>}
          {Boolean(error) && !loading && <p className="rounded-lg bg-[var(--color-error-container)] p-3 text-sm text-[var(--color-on-error-container)]">{error instanceof Error ? error.message : t("settingsGeneral.systemConfig.messages.loadFailed")}</p>}

          {!loading && groups.map(({ id: group, items }) => (
            <section className="mt-6 overflow-hidden border-t border-[var(--color-outline-variant)] pt-4 first:mt-0 first:border-t-0 first:pt-0" key={group}>
              <h3>
                <button
                  aria-controls={`system-config-group-${group}`}
                  aria-expanded={expandedGroups.has(group)}
                  className="flex w-full items-center justify-between gap-3 bg-[var(--color-surface-container)] px-4 py-3 text-left text-sm font-semibold text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-hover-surface)]"
                  type="button"
                  onClick={() => toggleGroup(group)}
                >
                  <span>{t(`settingsGeneral.systemConfig.categories.${group}.label`, { defaultValue: group })}</span>
                  <span className="flex items-center gap-[12px] text-xs font-normal text-[var(--color-on-surface-variant)]">
                    {t("settingsGeneral.systemConfig.itemCount", { count: items.length })}
                    <ChevronRight className={`size-4 transition-transform ${expandedGroups.has(group) ? "rotate-90" : ""}`} />
                  </span>
                </button>
              </h3>
              {expandedGroups.has(group) && <div className="divide-y divide-[var(--color-outline-variant)]" id={`system-config-group-${group}`}>
                {items.map((item) => {
                  const value = Object.hasOwn(changes, item.path) ? changes[item.path] : item.value;
                  const description = systemConfigHelp(item.path, t);
                  const label = t(`settingsGeneral.systemConfig.fields.${item.path.replaceAll(".", "_")}`);
                  return (
                    <div className="grid gap-3 px-4 py-3 lg:grid-cols-[minmax(300px,1fr)_minmax(260px,1.5fr)] lg:items-center" key={item.path}>
                      <div className="min-w-0">
                        <div className="flex flex-wrap items-center gap-1.5">
                          <span className="text-sm text-[var(--color-on-surface)]">{label}</span>
                          <Badge variant={item.effect === "migration" ? "warning" : item.effect === "hotUpdate" ? "success" : "info"}>
                            {t(`settingsGeneral.systemConfig.effects.${item.effect}`)}
                          </Badge>
                          {item.overriddenByEnvironment && <Badge variant="outline">{t("settingsGeneral.systemConfig.environmentOverride")}</Badge>}
                          {description && <InfoTooltip ariaLabel={t("settingsGeneral.systemConfig.fieldHelp", { path: label })}>
                            {`${description}${item.effectReason ? ` ${t(`settingsGeneral.systemConfig.effectReasons.${item.effectReason}`, { defaultValue: "" })}` : ""}`}
                          </InfoTooltip>}
                        </div>
                      </div>
                      {item.valueType === "boolean" ? (
                        <SelectInput aria-label={label} disabled={saving} value={String(value)} onValueChange={(next) => update(item.path, next === "true")}>
                          <SelectItem value="true">{t("settingsGeneral.systemConfig.values.true")}</SelectItem>
                          <SelectItem value="false">{t("settingsGeneral.systemConfig.values.false")}</SelectItem>
                        </SelectInput>
                      ) : item.valueType === "number" && !item.sensitive ? (
                        <NumericConfigInput label={label} value={value} disabled={saving} onChange={(next) => update(item.path, next)} />
                      ) : (
                        <TextInput
                          aria-label={label}
                          disabled={saving}
                          placeholder={item.sensitive ? t("settingsGeneral.systemConfig.sensitivePlaceholder") : undefined}
                          type={item.sensitive ? "password" : "text"}
                          value={value === null ? "" : String(value)}
                          onChange={(event) => update(item.path, item.valueType === "number" ? (event.target.value === "" ? null : Number(event.target.value)) : (item.path.includes(".prices.") && event.target.value === "" ? null : event.target.value))}
                        />
                      )}
                    </div>
                  );
                })}
              </div>}
            </section>
          ))}
        </div>
      )}

    </SurfaceCard>
  );
}

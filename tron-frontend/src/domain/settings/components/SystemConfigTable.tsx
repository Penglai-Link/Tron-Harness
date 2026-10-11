import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { SelectInput, TextArea, TextInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { SurfaceCard } from "@/shared";
import { useToast } from "@/components/ui/toast";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { SelectItem } from "@/components/ui/select";
import { useSystemConfig } from "@/domain/settings/hooks/useSystemConfig";
import type { MineruNodeConfig, SystemConfigItem, SystemConfigValue } from "@/domain/settings/model/SystemConfig";
import { MineruNodeEditor } from "./MineruNodeEditor";
import { validateMineruNodes } from "../model/mineruNodeValidation";
import { systemConfigHelp } from "../model/systemConfigHelp";

function displayGroup(path: string) {
  return path.includes(".") ? path.slice(0, path.indexOf(".")) : "server";
}

function displayField(path: string) {
  return path.slice(path.lastIndexOf(".") + 1);
}

export function SystemConfigTable() {
  const { t } = useTranslation("pages");
  const toast = useToast();
  const { document, error, loading, refresh, save, saving } = useSystemConfig();
  const [changes, setChanges] = useState<Record<string, SystemConfigValue>>({});
  const [confirmMigration, setConfirmMigration] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(() => new Set());
  const groups = useMemo(() => {
    const grouped = new Map<string, SystemConfigItem[]>();
    document?.items.filter((item) => !item.path.startsWith("mineru.") || !["parseEndpoint", "backend", "parseMethod", "returnContentList"].includes(displayField(item.path))).forEach((item) => {
      const group = displayGroup(item.path);
      grouped.set(group, [...(grouped.get(group) ?? []), item]);
    });
    return [...grouped.entries()];
  }, [document]);
  const dirtyItems = document?.items.filter((item) => Object.hasOwn(changes, item.path)) ?? [];
  const hasMigration = dirtyItems.some((item) => item.effect === "migration");

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
      if (Object.hasOwn(changes, "mineru.nodes")) {
        const issue = validateMineruNodes(changes["mineru.nodes"] as MineruNodeConfig[]);
        if (issue) throw new Error(t(`settingsGeneral.systemConfig.mineruNodes.validation.${issue.key}`, { node: issue.node }));
      }
      const result = await save(dirtyItems.map((item) => {
        const value = changes[item.path];
        return { path: item.path, value: item.path === "mineru.nodes" && Array.isArray(value)
          ? (value as MineruNodeConfig[]).map((node) => ({ ...node, id: node.id.trim(), baseUrl: node.baseUrl.trim(), resourceGroup: node.resourceGroup.trim() }))
          : Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string").map((entry) => entry.trim()).filter(Boolean) : value };
      }), confirmMigration);
      setChanges({});
      setConfirmMigration(false);
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
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex items-center gap-1.5">
          <button
            aria-controls="system-config-groups"
            aria-expanded={expanded}
            className="flex items-center gap-1.5 text-left text-sm font-semibold text-[var(--color-on-surface)]"
            type="button"
            onClick={() => setExpanded((current) => !current)}
          >

            <span>{t("settingsGeneral.systemConfig.title")}</span>
          </button>
          <InfoTooltip ariaLabel={t("settingsGeneral.systemConfig.help")}>{t("settingsGeneral.systemConfig.description")}</InfoTooltip>
        </div>
        <div className="flex gap-[12px]">
          <Button disabled={loading || saving} onClick={refresh} variant="secondary">{t("settingsGeneral.systemConfig.actions.refresh")}</Button>
          {dirtyItems.length > 0 && <Button disabled={saving || (hasMigration && !confirmMigration)} onClick={() => void handleSave()}>
            {saving ? t("settingsGeneral.systemConfig.actions.saving") : t("settingsGeneral.systemConfig.actions.save", { count: dirtyItems.length })}
          </Button>}
        </div>
      </div>

      {expanded && (
        <div id="system-config-groups">
          {loading && <p className="text-sm text-[var(--color-on-surface-variant)]">{t("settingsGeneral.systemConfig.loading")}</p>}
          {Boolean(error) && !loading && <p className="rounded-lg bg-[var(--color-error-container)] p-3 text-sm text-[var(--color-on-error-container)]">{error instanceof Error ? error.message : t("settingsGeneral.systemConfig.messages.loadFailed")}</p>}

          {!loading && groups.map(([group, items]) => (
            <section className="mt-4 overflow-hidden rounded-xl border border-[var(--color-outline-variant)] first:mt-0" key={group}>
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

                  </span>
                </button>
              </h3>
              {expandedGroups.has(group) && <div className="divide-y divide-[var(--color-outline-variant)]" id={`system-config-group-${group}`}>
                {items.map((item) => {
                  const value = Object.hasOwn(changes, item.path) ? changes[item.path] : item.value;
                  const description = systemConfigHelp(item.path, t);
                  return (
                    <div className="grid gap-3 px-4 py-3 lg:grid-cols-[minmax(300px,1fr)_minmax(260px,1.5fr)] lg:items-center" key={item.path}>
                      <div className="min-w-0">
                        <div className="flex flex-wrap items-center gap-1.5">
                          <span className="text-xs text-[var(--color-on-surface-variant)]">{t(`settingsGeneral.systemConfig.valueTypes.${item.valueType}`)}</span>
                          <code className="break-all text-xs text-[var(--color-on-surface)]">{item.path}</code>
                          <Badge variant={item.effect === "migration" ? "warning" : item.effect === "hotUpdate" ? "success" : "info"}>
                            {t(`settingsGeneral.systemConfig.effects.${item.effect}`)}
                          </Badge>
                          {item.overriddenByEnvironment && <Badge variant="outline">{t("settingsGeneral.systemConfig.environmentOverride")}</Badge>}
                          {description && <InfoTooltip ariaLabel={t("settingsGeneral.systemConfig.fieldHelp", { path: item.path })}>
                            {`${description}${item.effectReason ? ` ${t(`settingsGeneral.systemConfig.effectReasons.${item.effectReason}`, { defaultValue: "" })}` : ""}`}
                          </InfoTooltip>}
                        </div>
                      </div>
                      {item.path === "mineru.nodes" ? <MineruNodeEditor value={value} disabled={saving} onChange={(nodes) => update(item.path, nodes)} /> : item.path === "mineru.defaultTier" ? (<SelectInput value={String(value)} onValueChange={(next) => update(item.path, next)}>{(["flash", "basic", "standard", "advanced"] as const).map((tier) => <SelectItem key={tier} value={tier}>{t(`knowledgeLibrary.parsingTiers.${tier}`)}</SelectItem>)}</SelectInput>) : item.valueType === "array" ? (
                        <TextArea
                          aria-label={item.path}
                          rows={6}
                          value={Array.isArray(value) ? value.join("\n") : ""}
                          onChange={(event) => update(item.path, event.target.value.split("\n"))}
                        />
                      ) : item.valueType === "boolean" ? (
                        <SelectInput value={String(value)} onValueChange={(next) => update(item.path, next === "true")}>
                          <SelectItem value="true">{t("settingsGeneral.systemConfig.values.true")}</SelectItem>
                          <SelectItem value="false">{t("settingsGeneral.systemConfig.values.false")}</SelectItem>
                        </SelectInput>
                      ) : (
                        <TextInput
                          aria-label={item.path}
                          placeholder={item.sensitive ? t("settingsGeneral.systemConfig.sensitivePlaceholder") : undefined}
                          type={item.sensitive ? "password" : item.valueType === "number" ? "number" : "text"}
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

      {hasMigration && (
        <label className="flex items-start gap-3 rounded-lg bg-[var(--color-warning-container)] p-3 text-sm text-[var(--color-on-warning-container)]">
          <input className="mt-1 size-4" type="checkbox" checked={confirmMigration} onChange={(event) => setConfirmMigration(event.target.checked)} />
          <span>{t("settingsGeneral.systemConfig.confirmMigration")}</span>
        </label>
      )}
    </SurfaceCard>
  );
}

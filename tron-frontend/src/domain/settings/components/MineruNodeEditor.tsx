import { useTranslation } from "react-i18next";
import { TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import type { MineruNodeConfig, SystemConfigValue } from "../model/SystemConfig";

export function MineruNodeEditor({ value, disabled, onChange }: {
  value: SystemConfigValue;
  disabled: boolean;
  onChange: (nodes: MineruNodeConfig[]) => void;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string) => t(`settingsGeneral.systemConfig.mineruNodes.${key}`);
  const nodes = Array.isArray(value) ? value as MineruNodeConfig[] : [];
  const update = (index: number, patch: Partial<MineruNodeConfig>) => onChange(nodes.map((node, i) => i === index ? { ...node, ...patch } : node));

  function add() {
    let next = 1;
    while (nodes.some((node) => node.id === `parser_${next}`)) next++;
    onChange([...nodes, {
      id: `parser_${next}`, baseUrl: "http://127.0.0.1:8000", enabled: false,
      allowRemote: false, capacity: 1, weight: 1, resourceGroup: "", resourceGroupCapacity: 1,
    }]);
  }

  return <div className="min-w-0 space-y-3">
    <p className="text-xs text-[var(--color-on-surface-variant)]">{text(nodes.length ? "description" : "empty")}</p>
    {nodes.map((node, index) => <fieldset disabled={disabled} key={index} className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] p-3">
      <legend className="px-1 text-sm font-medium">{t("settingsGeneral.systemConfig.mineruNodes.node", { count: index + 1 })}</legend>
      <div className="grid gap-3 sm:grid-cols-2">
        {(["id", "baseUrl", "resourceGroup"] as const).map((field) => <label className={`space-y-1 text-xs ${field === "baseUrl" ? "sm:col-span-2" : ""}`} key={field}>
          <span>{text(field)}</span>
          <TextInput aria-label={`${text(field)} ${index + 1}`} value={node[field]} onChange={(event) => update(index, { [field]: event.target.value })} />
        </label>)}
        {(["capacity", "weight", "resourceGroupCapacity"] as const).map((field) => <label className="space-y-1 text-xs" key={field}>
          <span>{text(field)}</span>
          <TextInput aria-label={`${text(field)} ${index + 1}`} type="text" inputMode="numeric" value={Number.isFinite(node[field]) ? node[field] : ""} onChange={(event) => { const raw = event.target.value; if (/^\d*$/.test(raw)) update(index, { [field]: raw === "" ? NaN : Number(raw) }); }} />
        </label>)}
      </div>
      <div className="flex flex-wrap items-center gap-4 text-xs">
        <label className="flex items-center gap-2"><Switch checked={node.enabled} disabled={disabled} onCheckedChange={(enabled) => update(index, { enabled })} />{text("enabled")}</label>
        <label className="flex items-center gap-2"><Switch checked={node.allowRemote} disabled={disabled} onCheckedChange={(allowRemote) => update(index, { allowRemote })} />{text("allowRemote")}</label>
        <Button disabled={disabled} size="sm" variant="outline" onClick={() => onChange(nodes.filter((_, i) => i !== index))}>{text("remove")}</Button>
      </div>
    </fieldset>)}
    <Button disabled={disabled} size="sm" variant="outline" onClick={add}>{text("add")}</Button>
  </div>;
}

import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Select, SelectItem, SelectPopup, SelectTrigger, SelectValue } from "@/components/ui/select";
import type { ToolPermissionRecord, ToolRiskLevel } from "../model/ToolPermissionModel";

const levels: ToolRiskLevel[] = ["safe", "low", "medium", "high", "critical"];
export function ToolPermissionControl({ value, name, disabled, onChange }: {
  value: ToolPermissionRecord; name: string; disabled: boolean; onChange: (value: ToolPermissionRecord) => void;
}) {
  const { t } = useTranslation("pages");
  const prefix = "settingsTools.permissions";
  return <div className="min-w-0 space-y-2">
    <div className="flex flex-wrap gap-1" role="group" aria-label={t(`${prefix}.label`, { tool: name })}>
      {(["run", "ask", "no"] as const).map((permission) => <Button key={permission} type="button" size="sm"
        disabled={disabled} aria-pressed={value.permission === permission}
        variant={value.permission === permission ? (permission === "no" ? "destructive" : "default") : "outline"}
        onClick={() => onChange({ ...value, permission })}>{t(`${prefix}.${permission}`)}</Button>)}
    </div>
    {value.permission === "ask" && <div className="space-y-1">
      <Select value={value.confirmLevel} disabled={disabled} onValueChange={(confirmLevel) => {
        if (confirmLevel) onChange({ ...value, confirmLevel: confirmLevel as ToolRiskLevel });
      }}>
        <SelectTrigger size="sm" aria-label={t(`${prefix}.threshold`, { tool: name })} className="w-full">
          <SelectValue>{t(`${prefix}.levels.${value.confirmLevel}`)}</SelectValue>
        </SelectTrigger>
        <SelectPopup>{levels.map((level) => <SelectItem key={level} value={level}>{t(`${prefix}.levels.${level}`)}</SelectItem>)}</SelectPopup>
      </Select>
      <p className="max-w-72 text-xs text-[var(--color-on-surface-variant)]">{t(`${prefix}.riskHint`)}</p>
    </div>}
  </div>;
}

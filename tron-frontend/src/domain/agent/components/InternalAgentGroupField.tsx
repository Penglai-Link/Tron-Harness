import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { useInternalAgentGroups } from "../hooks/useInternalAgentGroups";

export function InternalAgentGroupField({ workspaceCode, value, disabled, onChange }: {
  workspaceCode: string; value: string; disabled?: boolean; onChange: (name: string) => void;
}) {
  const { t } = useTranslation(["pages"]);
  const labels: Record<string, string> = { label: "Group", new: "New group", name: "Group name", add: "Add", ungrouped: "Ungrouped", failed: "Could not load or save groups" };
  const text = (key: string) => t(`pages:workspaceAgents.internal.groups.${key}`, { defaultValue: labels[key] });
  const { groups, isPending, error } = useInternalAgentGroups(workspaceCode);
  const [adding, setAdding] = useState(false);
  const [name, setName] = useState("");
  function add() {
    const next = name.trim();
    if (!next || disabled) return;
    onChange(next);
    setName("");
    setAdding(false);
  }
  const options = [...new Set([...groups, ...(value ? [value] : [])])].sort();
  return <Field label={text("label")} error={error ? text("failed") : undefined} actions={!disabled ? <Button type="button" size="sm" variant="ghost" disabled={disabled} onClick={() => setAdding(!adding)}>{text("new")}</Button> : undefined}>
    <SelectInput aria-label={text("label")} disabled={disabled || isPending} value={value ? `group:${value}` : "ungrouped"} onValueChange={(next) => onChange(!next || next === "ungrouped" ? "" : next.slice(6))}>
      <SelectItem value="ungrouped">{text("ungrouped")}</SelectItem>
      {options.map((group) => <SelectItem key={group} value={`group:${group}`}>{group}</SelectItem>)}
    </SelectInput>
    {adding && !disabled ? <div className="mt-2 flex gap-2">
      <TextInput aria-label={text("name")} placeholder={text("name")} maxLength={60} disabled={disabled} value={name} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); void add(); } }} />
      <Button type="button" variant="outline" disabled={disabled || !name.trim()} onClick={() => void add()}>{text("add")}</Button>
    </div> : null}
  </Field>;
}

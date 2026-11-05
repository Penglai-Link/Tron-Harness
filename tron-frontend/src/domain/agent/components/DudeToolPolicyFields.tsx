import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { PlatformToolRecord, ToolAssignmentMode } from "../model";
import type { AgentPolicyDraft } from "./AgentPolicyFields";
import { dudeToolsForRule, hasDudeToolAlias } from "./dudeToolPolicy";

/** Category rules remain dynamic; explicit selections remain a fixed list. */
export function DudeToolPolicyFields({ draft, tools, disabled, onChange, onChooseTools }: {
  draft: AgentPolicyDraft;
  tools: PlatformToolRecord[];
  disabled: boolean;
  onChange: (patch: Partial<AgentPolicyDraft>) => void;
  onChooseTools: () => void;
}) {
  const { t, i18n } = useTranslation("pages");
  const text = (key: string, options?: Record<string, unknown>) => t(`workspaceAgents.internal.dudeToolPolicy.${key}`, options);
  const categories = useMemo(() => {
    const locale = i18n.resolvedLanguage ?? i18n.language;
    const grouped = new Map<string, { code: string; name: string; tools: PlatformToolRecord[] }>();
    for (const tool of tools) {
      if (!tool.category) continue;
      const { code, names } = tool.category;
      const entry = grouped.get(code) ?? { code, name: names[locale] ?? names["en-US"] ?? code, tools: [] };
      entry.tools.push(tool);
      grouped.set(code, entry);
    }
    for (const code of draft.selectedCategoryCodes ?? []) {
      if (code !== "readonly" && !grouped.has(code)) grouped.set(code, { code, name: code, tools: [] });
    }
    return [...grouped.values()];
  }, [tools, draft.selectedCategoryCodes, i18n.language, i18n.resolvedLanguage]);
  const chosen = new Set(draft.selectedCategoryCodes ?? []);
  const allTools = draft.toolAssignmentMode === "all_platform_tools";
  const matching = dudeToolsForRule(tools, draft.toolAssignmentMode, draft.selectedCategoryCodes ?? [], draft.selectedToolCodes);
  const selectedNames = draft.selectedToolCodes.map((code) => tools.find((tool) => tool.toolCode === code)?.name ?? code);
  const modes: { value: ToolAssignmentMode; label: string }[] = [
    { value: "selected_categories", label: text("byCategory") },
    { value: "selected_tools", label: text("byTool") },
    { value: "all_platform_tools", label: text("all") },
    { value: "all_read_only_tools", label: text("readOnly") },
  ];
  const setCategory = (code: string, checked: boolean) => {
    const next = new Set(draft.selectedCategoryCodes ?? []);
    if (checked) next.add(code); else next.delete(code);
    onChange({ selectedCategoryCodes: [...next] });
  };
  return (
    <section className="space-y-3" aria-label={text("title")}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-medium">{text("title")}</h3>
        {!allTools && <span className="text-xs text-[var(--color-on-surface-variant)]">{text("matchingCount", { count: matching.length })}</span>}
      </div>
      <div className="flex flex-wrap gap-2" role="group" aria-label={text("modeLabel")}>
        {modes.map((mode) => <Button key={mode.value} type="button" size="sm"
          variant={draft.toolAssignmentMode === mode.value ? "default" : "outline"}
          aria-pressed={draft.toolAssignmentMode === mode.value} disabled={disabled}
          onClick={() => mode.value === "selected_tools" ? onChooseTools() : onChange({ toolAssignmentMode: mode.value })}>
          {mode.label}
        </Button>)}
      </div>
      {draft.toolAssignmentMode === "selected_tools" ? (
        <div className="space-y-2 rounded-md border border-[var(--color-outline-variant)] p-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="text-sm">{text("selectedCount", { count: draft.selectedToolCodes.length })}</span>
            <Button type="button" size="sm" variant="outline" disabled={disabled} onClick={onChooseTools}>{text("editTools")}</Button>
          </div>
          <p className="text-xs text-[var(--color-on-surface-variant)]">{text("fixedHint")}</p>
          {selectedNames.length ? <p className="break-words text-sm">{selectedNames.join(" · ")}</p> : <p className="text-sm text-[var(--color-on-surface-variant)]">{text("noneSelected")}</p>}
        </div>
      ) : !allTools ? (
        <>
          <p className="text-xs text-[var(--color-on-surface-variant)]">{text(draft.toolAssignmentMode === "selected_categories" ? "categoryHint" : "readOnlyHint")}</p>
          {draft.toolAssignmentMode === "selected_categories" && (
            <div className="divide-y divide-[var(--color-outline-variant)] rounded-md border border-[var(--color-outline-variant)]">
              {[...categories, { code: "readonly", name: text("readOnly"), tools: tools.filter((tool) => tool.operationKind === "query") }].map((category) => (
                <div key={category.code} className="px-3 py-2">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <label className="flex min-w-0 items-center gap-2 text-sm">
                      <input type="checkbox" className="size-4 shrink-0 accent-[var(--color-primary)]" disabled={disabled} checked={chosen.has(category.code)} onChange={(event) => setCategory(category.code, event.target.checked)} />
                      <span className="break-words">{category.name}</span>
                    </label>
                    <span className="text-xs text-[var(--color-on-surface-variant)]">{text("matchingCount", { count: dudeToolsForRule(tools, "selected_categories", [category.code]).length })}</span>
                  </div>
                  {category.tools.length > 0 && <details className="mt-1 pl-6 text-xs text-[var(--color-on-surface-variant)]">
                    <summary className="w-fit cursor-pointer">{text("viewTools")}</summary>
                    <ul className="mt-2 space-y-2">{category.tools.map((tool) => <li key={tool.toolCode} className="break-words">
                      <span className="font-medium text-[var(--color-on-surface)]">{tool.name}</span>
                      {(tool.assignable === false || tool.dudeAssignable === false) && <span> · {tool.assignable === false ? text("unavailable") : tool.dudeAssignable === false && hasDudeToolAlias(tool) ? text("replacedByDude") : text("workspaceOnly")}</span>}
                      {tool.description && <p className="mt-0.5">{tool.description}</p>}
                    </li>)}</ul>
                  </details>}
                </div>
              ))}
            </div>
          )}
          {draft.toolAssignmentMode !== "selected_categories" && <details className="text-sm text-[var(--color-on-surface-variant)]">
            <summary className="w-fit cursor-pointer">{text("viewTools")}</summary>
            <ul className="mt-2 space-y-1">{matching.map((tool) => <li key={tool.toolCode} className="break-words">{tool.name}</li>)}</ul>
          </details>}
        </>
      ) : null}
    </section>
  );
}

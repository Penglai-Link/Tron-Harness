import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { SelectItem } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { InfoTooltip } from "@/components/info-tooltip";
import type { WorkspaceInternalAgentRecord } from "../model";

export type AgentPolicyDraft = Pick<WorkspaceInternalAgentRecord, "toolAssignmentMode" | "toolExposureMode" | "expandedDomainPersistenceMode" | "maxSteps" | "maxToolCalls" | "maxParallelToolCalls" | "mcpAccessMode" | "selectedToolCodes" | "selectedServerCodes">;
export const newAgentPolicyDraft = (): AgentPolicyDraft => ({ toolAssignmentMode: "all_platform_tools", toolExposureMode: "domain_discovery", expandedDomainPersistenceMode: "sticky_expanded", maxSteps: 100, maxToolCalls: 100, maxParallelToolCalls: 4, mcpAccessMode: "all_enabled_servers", selectedToolCodes: [], selectedServerCodes: [] });
const labels = { mcpAccessMode: "MCP access mode", toolAssignmentMode: "Tool assignment mode", toolExposureMode: "Tool exposure mode", expandedDomainPersistenceMode: "Expanded domain persistence mode", maxSteps: "Max steps", maxToolCalls: "Max total tool calls", maxParallelToolCalls: "Max parallel tool calls" };
export function AgentPolicyFields({ panel, draft, disabled, onChange, onAssignment, children }: {
  panel: "resources" | "advanced"; draft: AgentPolicyDraft; disabled: boolean;
  onChange: (patch: Partial<AgentPolicyDraft>) => void; onAssignment?: (kind: "mcp" | "tools") => void; children?: ReactNode;
}) {
  const { t } = useTranslation("pages");
  const text = (key: string, fallback: string, options?: Record<string, unknown>) => t(`workspaceAgents.internal.${key}`, { defaultValue: fallback, ...options });
  const label = (key: keyof typeof labels) => <span className="inline-flex items-center gap-1.5"><span>{text(`labels.${key}`, labels[key])}</span><InfoTooltip ariaLabel={text(`help.${key}.ariaLabel`, `Explain ${labels[key]}`)}><p>{text(`help.${key}.summary`, "")}</p><p>{text(`help.${key}.optionOne`, "")}</p><p>{text(`help.${key}.optionTwo`, "")}</p>{key === "toolAssignmentMode" ? <p>{text(`help.${key}.optionThree`, "Select declared read-only tools automatically each run.")}</p> : null}</InfoTooltip></span>;
  if (panel === "advanced") return <div className="grid gap-3 lg:grid-cols-2">
    <Field label={label("toolExposureMode")}><SelectInput disabled={disabled} value={draft.toolExposureMode} onValueChange={(value) => onChange({ toolExposureMode: value as AgentPolicyDraft["toolExposureMode"] })}><SelectItem value="direct_list">{text("modes.directList", "Direct list")}</SelectItem><SelectItem value="domain_discovery">{text("modes.domainDiscovery", "Domain discovery")}</SelectItem></SelectInput></Field>
    <Field label={label("expandedDomainPersistenceMode")}><SelectInput disabled={disabled} value={draft.expandedDomainPersistenceMode} onValueChange={(value) => onChange({ expandedDomainPersistenceMode: value as AgentPolicyDraft["expandedDomainPersistenceMode"] })}><SelectItem value="sticky_expanded">{text("modes.stickyExpanded", "Sticky expanded")}</SelectItem><SelectItem value="ephemeral_expanded">{text("modes.ephemeralExpanded", "Ephemeral expanded")}</SelectItem></SelectInput></Field>
    {(["maxSteps", "maxToolCalls", "maxParallelToolCalls"] as const).map((key) => <Field key={key} label={label(key)}><TextInput aria-label={text(`labels.${key}`, labels[key])} disabled={disabled} min={1} step={1} type="number" value={Number.isFinite(draft[key]) ? draft[key] : ""} onChange={(event) => onChange({ [key]: Number.isFinite(event.currentTarget.valueAsNumber) ? Math.max(1, Math.trunc(event.currentTarget.valueAsNumber)) : draft[key] })} /></Field>)}
  </div>;
  return <div className="grid gap-3 lg:grid-cols-2">
    <Field label={label("mcpAccessMode")} actions={draft.mcpAccessMode === "selected_servers" ? <Button type="button" size="sm" variant="outline" disabled={disabled} onClick={() => onAssignment?.("mcp")}>{text("assignmentDialog.editServers", "Choose servers ({{count}})", { count: draft.selectedServerCodes.length })}</Button> : undefined}>
      <SelectInput disabled={disabled} value={draft.mcpAccessMode} onValueChange={(value) => value === "selected_servers" ? onAssignment?.("mcp") : onChange({ mcpAccessMode: value as AgentPolicyDraft["mcpAccessMode"] })}>
        <SelectItem value="disabled">{text("modes.mcpDisabled", "Disabled")}</SelectItem><SelectItem value="all_enabled_servers">{text("modes.allEnabledServers", "All enabled servers")}</SelectItem><SelectItem value="selected_servers">{text("modes.selectedServers", "Selected servers")}</SelectItem>
      </SelectInput>
    </Field>
    <Field label={label("toolAssignmentMode")} actions={draft.toolAssignmentMode === "selected_tools" ? <Button type="button" size="sm" variant="outline" disabled={disabled} onClick={() => onAssignment?.("tools")}>{text("assignmentDialog.editTools", "Choose tools ({{count}})", { count: draft.selectedToolCodes.length })}</Button> : undefined}>
      <SelectInput disabled={disabled} value={draft.toolAssignmentMode} onValueChange={(value) => value === "selected_tools" ? onAssignment?.("tools") : onChange({ toolAssignmentMode: value as AgentPolicyDraft["toolAssignmentMode"] })}>
        <SelectItem value="all_platform_tools">{text("modes.allTools", "All platform tools")}</SelectItem><SelectItem value="all_read_only_tools">{text("modes.allReadOnlyTools", "All read-only tools")}</SelectItem><SelectItem value="selected_tools">{text("modes.selectedTools", "Selected tools")}</SelectItem>
      </SelectInput>
    </Field>
    {children}
  </div>;
}

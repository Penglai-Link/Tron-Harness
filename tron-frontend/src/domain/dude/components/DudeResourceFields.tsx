import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Field, SelectInput, TextInput } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { InfoTooltip } from "@/components/info-tooltip";
import {
  fetchToolDomains, listTools,
  patchWorkspaceInternalAgentPolicy, patchWorkspaceInternalAgentMcpPolicy,
  updateWorkspaceInternalAgentToolAssignments, updateWorkspaceInternalAgentMcpServerAssignments,
} from "@/domain/agent/api";
import { getDudeResources } from "../api/dudeApi";
import { PLATFORM_INTERNAL_AGENT_SCOPE, notifyInternalAgentRegistryChanged } from "@/domain/agent/model";
import type { WorkspaceInternalAgentRecord, WorkspaceInternalAgentPolicyPayload, PlatformToolRecord, ToolCatalogDomainRecord } from "@/domain/agent/model";
import { InternalAgentAssignmentDialog, type InternalAgentAssignmentKind } from "@/domain/agent/components/InternalAgentAssignmentDialog";
import { InternalAgentSkillField } from "@/domain/agent/components/InternalAgentSkillField";
import { InternalAgentKnowledgeField } from "@/domain/agent/components/InternalAgentKnowledgeField";
import { setAgentSkills } from "@/domain/skill/api";
import { setAgentKnowledgeLibraries } from "@/domain/knowledge-base/api/knowledgeBaseApi";
import { fetchPlatformMcpServers } from "@/domain/mcp/api/mcpApi";
import type { WorkspaceMcpServerRecord } from "@/domain/mcp/model";

export interface DudeResourceFieldsHandle { save(): Promise<void> }
export interface DudeResourceFieldsProps {
  dudeCode: string;
  panel?: "resources" | "advanced";
  disabled?: boolean;
  onDirty?: (dirty: boolean) => void;
}

type PolicyDraft = Pick<WorkspaceInternalAgentRecord,
  "toolAssignmentMode" | "toolExposureMode" | "expandedDomainPersistenceMode" |
  "maxSteps" | "maxToolCalls" | "maxParallelToolCalls" | "mcpAccessMode" | "selectedToolCodes" | "selectedServerCodes">;
const policyKeys = ["toolAssignmentMode", "toolExposureMode", "expandedDomainPersistenceMode", "maxSteps", "maxToolCalls", "maxParallelToolCalls"] as const;
const sameCodes = (a: string[], b: string[]) => a.length === b.length && a.every((code) => b.includes(code));
const asDraft = (record: WorkspaceInternalAgentRecord): PolicyDraft => ({
  toolAssignmentMode: record.toolAssignmentMode, toolExposureMode: record.toolExposureMode,
  expandedDomainPersistenceMode: record.expandedDomainPersistenceMode, maxSteps: record.maxSteps,
  maxToolCalls: record.maxToolCalls, maxParallelToolCalls: record.maxParallelToolCalls,
  mcpAccessMode: record.mcpAccessMode, selectedToolCodes: [...record.selectedToolCodes], selectedServerCodes: [...record.selectedServerCodes],
});

/** All selectors edit local drafts. Only the enclosing Dude editor calls save(). */
export const DudeResourceFields = forwardRef<DudeResourceFieldsHandle, DudeResourceFieldsProps>(function DudeResourceFields({ dudeCode, panel = "resources", disabled = false, onDirty }, ref) {
  const { t } = useTranslation("pages");
  const text = (key: string, options?: Record<string, unknown>) => t(`workspaceAgents.internal.${key}`, options);
  const client = useQueryClient();
  const [record, setRecord] = useState<WorkspaceInternalAgentRecord | null>(null);
  const [draft, setDraft] = useState<PolicyDraft | null>(null);
  const [tools, setTools] = useState<PlatformToolRecord[]>([]);
  const [domains, setDomains] = useState<ToolCatalogDomainRecord[]>([]);
  const [servers, setServers] = useState<WorkspaceMcpServerRecord[]>([]);
  const [skillDraft, setSkillDraft] = useState<string[] | undefined>();
  const [knowledgeDraft, setKnowledgeDraft] = useState<string[] | undefined>();
  const [assignment, setAssignment] = useState<InternalAgentAssignmentKind | null>(null);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const savingRef = useRef(false);
  const dirtyCallback = useRef(onDirty);
  dirtyCallback.current = onDirty;
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true); setRecord(null); setDraft(null); setError("");
    setSkillDraft(undefined); setKnowledgeDraft(undefined); setAssignment(null);
    void Promise.all([
      getDudeResources(dudeCode, controller.signal),
      fetchToolDomains(controller.signal), listTools({}, controller.signal), fetchPlatformMcpServers({}, controller.signal),
    ]).then(([agent, nextDomains, nextTools, nextServers]) => {
      if (controller.signal.aborted) return;
      setRecord(agent); setDraft(asDraft(agent)); setDomains(nextDomains); setTools(nextTools);
      setServers(nextServers.map((server) => ({ ...server, platformEnabled: server.enabled, effectiveEnabled: server.enabled, source: "platform", workspaceOverrideDisabled: false })));
    }).catch((failure: unknown) => {
      if (!controller.signal.aborted) setError(failure instanceof Error ? failure.message : String(failure));
    }).finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [dudeCode, loadAttempt]);
  const dirty = !!record && !!draft && (panel === "advanced"
    ? policyKeys.some((key) => record[key] !== draft[key])
    : record.toolAssignmentMode !== draft.toolAssignmentMode || record.mcpAccessMode !== draft.mcpAccessMode ||
      !sameCodes(record.selectedToolCodes, draft.selectedToolCodes) || !sameCodes(record.selectedServerCodes, draft.selectedServerCodes) ||
      skillDraft !== undefined || knowledgeDraft !== undefined);
  useEffect(() => { dirtyCallback.current?.(dirty); }, [dirty]);
  const patch = (change: Partial<PolicyDraft>) => setDraft((current) => current ? { ...current, ...change } : current);
  const blocked = disabled || saving || loading || !draft || !record;

  useImperativeHandle(ref, () => ({ save: async () => {
    if (savingRef.current) throw new Error(t("agentSkills.loading"));
    if (!record || !draft || loading) throw new Error(error || t("agentSkills.loading"));
    if (panel === "advanced") for (const key of ["maxSteps", "maxToolCalls", "maxParallelToolCalls"] as const) {
      if (!Number.isSafeInteger(draft[key]) || draft[key] < 1) throw new Error(text(`labels.${key}`));
    }
    savingRef.current = true; setSaving(true); setError("");
    try {
      let baseline = record;
      if (panel === "advanced") {
        const policy: WorkspaceInternalAgentPolicyPayload = {};
        for (const key of policyKeys) if (baseline[key] !== draft[key]) Object.assign(policy, { [key]: draft[key] });
        if (Object.keys(policy).length) baseline = await patchWorkspaceInternalAgentPolicy(PLATFORM_INTERNAL_AGENT_SCOPE, dudeCode, policy);
        setRecord(baseline); setDraft(asDraft(baseline)); notifyInternalAgentRegistryChanged();
        return;
      }
      // Store the visible assignments before changing modes; all-tools is an effective list.
      if (draft.toolAssignmentMode === "selected_tools" && (record.toolAssignmentMode !== "selected_tools" || !sameCodes(record.selectedToolCodes, draft.selectedToolCodes))) {
        baseline = await updateWorkspaceInternalAgentToolAssignments(PLATFORM_INTERNAL_AGENT_SCOPE, dudeCode, draft.selectedToolCodes);
        setRecord(baseline);
      }
      if (baseline.toolAssignmentMode !== draft.toolAssignmentMode) {
        baseline = await patchWorkspaceInternalAgentPolicy(PLATFORM_INTERNAL_AGENT_SCOPE, dudeCode, { toolAssignmentMode: draft.toolAssignmentMode });
        setRecord(baseline);
      }
      if (draft.mcpAccessMode === "selected_servers" && (record.mcpAccessMode !== "selected_servers" || !sameCodes(record.selectedServerCodes, draft.selectedServerCodes))) {
        baseline = await updateWorkspaceInternalAgentMcpServerAssignments(PLATFORM_INTERNAL_AGENT_SCOPE, dudeCode, draft.selectedServerCodes);
        setRecord(baseline);
      }
      if (baseline.mcpAccessMode !== draft.mcpAccessMode) {
        baseline = await patchWorkspaceInternalAgentMcpPolicy(PLATFORM_INTERNAL_AGENT_SCOPE, dudeCode, { mcpAccessMode: draft.mcpAccessMode });
        setRecord(baseline);
      }
      if (skillDraft !== undefined) {
        await setAgentSkills(dudeCode, skillDraft); setSkillDraft(undefined);
        await client.invalidateQueries({ queryKey: ["agent-skills", dudeCode] });
      }
      if (knowledgeDraft !== undefined) {
        await setAgentKnowledgeLibraries(dudeCode, knowledgeDraft); setKnowledgeDraft(undefined);
        await client.invalidateQueries({ queryKey: ["knowledge-agent-libraries"] });
      }
      setRecord(baseline); setDraft(asDraft(baseline));
      notifyInternalAgentRegistryChanged();
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
      throw failure;
    } finally { savingRef.current = false; setSaving(false); }
  } }), [record, draft, loading, error, dudeCode, skillDraft, knowledgeDraft, client, t]);

  const label = (key: string) => <span className="inline-flex items-center gap-1.5"><span>{text(`labels.${key}`)}</span><InfoTooltip ariaLabel={text(`help.${key}.ariaLabel`)}><p>{text(`help.${key}.summary`)}</p><p>{text(`help.${key}.optionOne`)}</p><p>{text(`help.${key}.optionTwo`)}</p></InfoTooltip></span>;
  if (loading) return <p role="status">{t("agentSkills.loading")}</p>;
  if (!record || !draft) return <div role="alert" className="space-y-2"><p>{error}</p><Button type="button" variant="outline" disabled={disabled} onClick={() => setLoadAttempt((value) => value + 1)}>{t("agentSkills.retry")}</Button></div>;
  if (panel === "advanced") return <div className="grid gap-3 lg:grid-cols-2">
    {error ? <p role="alert" className="text-[var(--color-error)]">{error}</p> : null}
    <Field label={label("toolExposureMode")}><SelectInput disabled={blocked} value={draft.toolExposureMode} onValueChange={(value) => patch({ toolExposureMode: value as PolicyDraft["toolExposureMode"] })}><SelectItem value="direct_list">{text("modes.directList")}</SelectItem><SelectItem value="domain_discovery">{text("modes.domainDiscovery")}</SelectItem></SelectInput></Field>
    <Field label={label("expandedDomainPersistenceMode")}><SelectInput disabled={blocked} value={draft.expandedDomainPersistenceMode} onValueChange={(value) => patch({ expandedDomainPersistenceMode: value as PolicyDraft["expandedDomainPersistenceMode"] })}><SelectItem value="sticky_expanded">{text("modes.stickyExpanded")}</SelectItem><SelectItem value="ephemeral_expanded">{text("modes.ephemeralExpanded")}</SelectItem></SelectInput></Field>
    {(["maxSteps", "maxToolCalls", "maxParallelToolCalls"] as const).map((key) => <Field key={key} label={label(key)}><TextInput aria-label={text(`labels.${key}`)} disabled={blocked} min={1} step={1} type="number" value={Number.isFinite(draft[key]) ? draft[key] : ""} onChange={(event) => patch({ [key]: event.currentTarget.valueAsNumber })} /></Field>)}
  </div>;
  return <div className="space-y-4">
    {error ? <p role="alert" className="text-[var(--color-error)]">{error}</p> : null}
    <div className="grid gap-3 lg:grid-cols-2">
      <Field label={label("mcpAccessMode")} actions={draft.mcpAccessMode === "selected_servers" ? <Button type="button" size="sm" variant="outline" disabled={blocked} onClick={() => setAssignment("mcp")}>{text("assignmentDialog.editServers", { count: draft.selectedServerCodes.length })}</Button> : undefined}>
        <SelectInput disabled={blocked} value={draft.mcpAccessMode} onValueChange={(value) => value === "selected_servers" ? setAssignment("mcp") : patch({ mcpAccessMode: value as PolicyDraft["mcpAccessMode"] })}>
          <SelectItem value="disabled">{text("modes.mcpDisabled")}</SelectItem><SelectItem value="all_enabled_servers">{text("modes.allEnabledServers")}</SelectItem><SelectItem value="selected_servers">{text("modes.selectedServers")}</SelectItem>
        </SelectInput>
      </Field>
      <Field label={label("toolAssignmentMode")} actions={draft.toolAssignmentMode === "selected_tools" ? <Button type="button" size="sm" variant="outline" disabled={blocked} onClick={() => setAssignment("tools")}>{text("assignmentDialog.editTools", { count: draft.selectedToolCodes.length })}</Button> : undefined}>
        <SelectInput disabled={blocked} value={draft.toolAssignmentMode} onValueChange={(value) => value === "selected_tools" ? setAssignment("tools") : patch({ toolAssignmentMode: value as PolicyDraft["toolAssignmentMode"] })}>
          <SelectItem value="all_platform_tools">{text("modes.allTools")}</SelectItem><SelectItem value="selected_tools">{text("modes.selectedTools")}</SelectItem>
        </SelectInput>
      </Field>
      <InternalAgentSkillField agentCode={dudeCode} disabled={blocked} selectedNames={skillDraft} onSelectionChange={setSkillDraft} />
      <InternalAgentKnowledgeField agentCode={dudeCode} agentName={record.name} disabled={blocked} selectedIds={knowledgeDraft} onSelectionChange={setKnowledgeDraft} />
    </div>
    {assignment ? <InternalAgentAssignmentDialog key={assignment} kind={assignment} agentName={record.name} busy={blocked} initialSelectedCodes={assignment === "mcp" ? draft.selectedServerCodes : draft.toolAssignmentMode === "selected_tools" ? draft.selectedToolCodes : []} servers={servers} tools={tools} toolDomains={domains} onClose={() => setAssignment(null)} onConfirm={(codes) => {
      patch(assignment === "mcp" ? { mcpAccessMode: "selected_servers", selectedServerCodes: codes } : { toolAssignmentMode: "selected_tools", selectedToolCodes: codes }); setAssignment(null);
    }} /> : null}
  </div>;
});

import {
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  Clipboard,
  Save,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import {
  Field,
  PageHeader,
  SelectInput,
  SurfaceCard,
  TextArea,
  TextInput,
  ToggleSwitch,
  useLocaleText,
  useToast,
  useWorkspaceSpaces,
} from "@/shared";
import { Badge } from "@/shared/ui/Badge";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  analyzePromptTemplateApi,
  createPlatformPrompt,
  createWorkspacePrompt,
  fetchPlatformPromptDetail,
  fetchWorkspacePromptDetail,
  updatePlatformPrompt,
  updateWorkspacePrompt,
} from "@/domain/prompt/api";
import {
  promptCategoryLabel,
  promptCategoryOptions,
  promptVariableTypeOptions,
} from "@/domain/prompt/constants";
import {
  analyzePromptTemplateMock,
  createPlatformPromptMock,
  createWorkspacePromptMock,
  getPlatformPromptMock,
  getWorkspacePromptMock,
  updatePlatformPromptMock,
  updateWorkspacePromptMock,
} from "@/domain/prompt/mock";
import {
  analyzePromptTemplate,
  normalizePromptVariables,
  promptVariableDescriptionMaxLength,
  promptVariablesAreReady,
  reconcilePromptVariables,
  type PromptAnalyzeResult,
  type PromptCategory,
  type PromptRecord,
  type PromptScopeKind,
  type PromptVariable,
  type PromptVariableType,
} from "@/domain/prompt/model";

interface PromptFormState {
  allowInheritance: boolean;
  category: PromptCategory;
  description: string;
  displayName: string;
  enabled: boolean;
  promptBody: string;
  variables: PromptVariable[];
}

function emptyForm(): PromptFormState {
  return {
    allowInheritance: true,
    category: "SYSTEM",
    description: "",
    displayName: "",
    enabled: true,
    promptBody: "",
    variables: [],
  };
}

function formFromRecord(record: PromptRecord): PromptFormState {
  return {
    allowInheritance: record.allowInheritance ?? true,
    category: record.category,
    description: record.description ?? "",
    displayName: record.displayName,
    enabled: record.enabled,
    promptBody: record.promptBody,
    variables: record.variables,
  };
}

export function PromptEditorPage({
  mode,
  scopeKind,
}: {
  mode: "create" | "detail";
  scopeKind: PromptScopeKind;
}) {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const pt = useLocaleText();
  const toast = useToast();
  const promptDataSource = getServiceDataSource("prompt");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const workspaceCode = scopeKind === "WORKSPACE" ? activeWorkspace?.workspaceCode ?? "" : "";
  const promptCode = mode === "detail" ? searchParams.get("promptCode")?.trim() ?? "" : "";
  const isOverrideMode = searchParams.get("mode") === "override";
  const [form, setForm] = useState<PromptFormState>(() => emptyForm());
  const [record, setRecord] = useState<PromptRecord | null>(null);
  const [loading, setLoading] = useState(mode === "detail");
  const [submitting, setSubmitting] = useState(false);
  const [backendAnalysis, setBackendAnalysis] = useState<PromptAnalyzeResult | null>(null);
  const localAnalysis = useMemo(() => analyzePromptTemplate(form.promptBody), [form.promptBody]);
  const analysis = backendAnalysis ?? {
    invalidVariables: localAnalysis.invalidVariables,
    variables: localAnalysis.variables.map((name) => ({ name, type: "STRING" as const })),
  };
  const variablesReady = useMemo(
    () => promptVariablesAreReady(form.promptBody, form.variables),
    [form.promptBody, form.variables],
  );
  const listHref = scopeKind === "WORKSPACE" ? "/prompts" : "/settings/prompts";

  useEffect(() => {
    setRecord(null);
    setBackendAnalysis(null);
    setForm(emptyForm());

    if (mode === "create") {
      setLoading(false);
      return;
    }
    if (!promptCode || (scopeKind === "WORKSPACE" && !workspaceCode)) {
      setLoading(false);
      return;
    }

    const controller = new AbortController();
    setLoading(true);
    const request = scopeKind === "WORKSPACE"
      ? (promptDataSource === "api"
        ? fetchWorkspacePromptDetail({ promptCode, workspaceCode }, controller.signal)
        : Promise.resolve(getWorkspacePromptMock({ promptCode, workspaceCode })))
      : (promptDataSource === "api"
        ? fetchPlatformPromptDetail({ promptCode }, controller.signal)
        : Promise.resolve(getPlatformPromptMock({ promptCode })));

    request
      .then((nextRecord) => {
        if (controller.signal.aborted) return;
        setRecord(nextRecord);
        setForm(formFromRecord(nextRecord));
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) toast.error(reason instanceof Error ? reason.message : pt("Prompt could not be loaded"));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [mode, promptCode, promptDataSource, pt, scopeKind, toast, workspaceCode]);

  useEffect(() => {
    setBackendAnalysis(null);

    if (promptDataSource !== "api" || !form.promptBody) return;
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      analyzePromptTemplateApi(form.promptBody, controller.signal)
        .then((result) => {
          if (!controller.signal.aborted) setBackendAnalysis(result);
        })
        .catch(() => {
          if (!controller.signal.aborted) setBackendAnalysis(null);
        });
    }, 250);

    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [form.promptBody, promptDataSource]);

  function updateForm<Key extends keyof PromptFormState>(key: Key, value: PromptFormState[Key]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function updatePromptBody(promptBody: string) {
    setForm((current) => ({
      ...current,
      promptBody,
      variables: reconcilePromptVariables(promptBody, current.variables),
    }));
  }

  function updateVariable(name: string, patch: Partial<Pick<PromptVariable, "description" | "type">>) {
    setForm((current) => ({
      ...current,
      variables: current.variables.map((variable) => (
        variable.name === name ? { ...variable, ...patch } : variable
      )),
    }));
  }

  async function authoritativeAnalysis() {
    return promptDataSource === "api"
      ? analyzePromptTemplateApi(form.promptBody)
      : analyzePromptTemplateMock(form.promptBody);
  }

  async function savePrompt() {
    if (submitting || !form.displayName.trim() || !form.promptBody.trim() || !variablesReady) return;
    if (scopeKind === "WORKSPACE" && !workspaceCode) {
      toast.error(pt("Select a workspace before saving"));
      return;
    }
    setSubmitting(true);
    try {
      const checked = await authoritativeAnalysis();
      setBackendAnalysis(checked);
      if (checked.invalidVariables.length) {
        toast.error(pt("Fix invalid variables before saving"));
        return;
      }
      const variables = normalizePromptVariables(form.promptBody, form.variables);
      const commonFields = {
        description: form.description.trim(),
        displayName: form.displayName.trim(),
        enabled: form.enabled,
        promptBody: form.promptBody,
        variables,
      };

      let saved: PromptRecord;
      if (mode === "create" && scopeKind === "WORKSPACE") {
        const payload = { ...commonFields, allowInheritance: form.allowInheritance, category: form.category, workspaceCode };
        saved = promptDataSource === "api" ? await createWorkspacePrompt(payload) : createWorkspacePromptMock(payload);
      } else if (mode === "create") {
        const payload = { ...commonFields, category: form.category };
        saved = promptDataSource === "api" ? await createPlatformPrompt(payload) : createPlatformPromptMock(payload);
      } else if (scopeKind === "WORKSPACE") {
        if (!record) throw new Error(pt("Prompt details are not loaded"));
        const payload = {
          ...commonFields,
          allowInheritance: record.sourceKind === "LOCAL" ? form.allowInheritance : undefined,
          expectedVersion: record.version,
          promptCode,
          workspaceCode,
        };
        saved = promptDataSource === "api" ? await updateWorkspacePrompt(payload) : updateWorkspacePromptMock(payload);
      } else {
        if (!record) throw new Error(pt("Prompt details are not loaded"));
        const payload = { ...commonFields, expectedVersion: record.version, promptCode };
        saved = promptDataSource === "api" ? await updatePlatformPrompt(payload) : updatePlatformPromptMock(payload);
      }

      toast.success(pt(isOverrideMode ? "Override saved" : "Prompt saved"));
      const params = new URLSearchParams({ promptCode: saved.promptCode });
      navigate(`${scopeKind === "WORKSPACE" ? "/prompts/detail" : "/settings/prompts/detail"}?${params.toString()}`);
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("Prompt could not be saved"));
    } finally {
      setSubmitting(false);
    }
  }

  const title = mode === "create"
    ? pt(scopeKind === "WORKSPACE" ? "Create Prompt" : "Create Platform Prompt")
    : pt(isOverrideMode ? "Create Prompt Override" : "Prompt Details");

  return (
    <>
      <PageHeader
        breadcrumbs={[{ label: pt(scopeKind === "WORKSPACE" ? "Prompts" : "Platform Prompts"), href: listHref }, { label: title }]}
        title={title}
        actions={(
          <>
            <Button size="lg" variant="outline" onClick={() => navigate(listHref)}>{pt("Cancel")}</Button>
            <Button
              size="lg"
              disabled={submitting || loading || !form.displayName.trim() || !form.promptBody.trim() || !variablesReady}
              aria-busy={submitting}
              onClick={() => void savePrompt()}
            >
              <Save size={16} aria-hidden="true" />
              {submitting ? pt("Saving...") : pt("Save")}
            </Button>
          </>
        )}
      />
      <div className="max-w-5xl">
        {scopeKind === "WORKSPACE" && !workspaceCode ? (
          <SurfaceCard className="text-sm text-[var(--color-on-surface-variant)]">{pt("Select a workspace to edit prompts.")}</SurfaceCard>
        ) : (
          <SurfaceCard className="space-y-5">
            {record ? (
              <div className="flex flex-wrap items-center gap-3 text-sm text-[var(--color-on-surface-variant)]">
                <Badge tone={record.sourceKind === "OVERRIDE" ? "warning" : "info"}>{pt(record.sourceKind)}</Badge>
                <span>{record.sourceLabel}</span>
                {record.sourceWorkspacePath.length ? <span>{record.sourceWorkspacePath.map((item) => item.name).join(" / ")}</span> : null}
              </div>
            ) : null}

            <div className="grid gap-4 md:grid-cols-2">
              {mode === "detail" ? (
                <Field label={pt("Prompt code")}>
                  <div className="flex gap-2">
                    <TextInput aria-label={pt("Prompt code")} className="font-mono" readOnly value={record?.promptCode ?? promptCode} />
                    <Button size="icon" variant="outline" aria-label={pt("Copy prompt code")} onClick={() => { void navigator.clipboard?.writeText(record?.promptCode ?? promptCode); toast.success(pt("Prompt code copied")); }}><Clipboard size={15} aria-hidden="true" /></Button>
                  </div>
                </Field>
              ) : null}
              <Field label={pt("Name")}>
                <TextInput value={form.displayName} onChange={(event) => updateForm("displayName", event.target.value)} />
              </Field>
              <Field label={pt("Category")}>
                {mode === "create" ? (
                  <SelectInput value={form.category} onValueChange={(value) => updateForm("category", value as PromptCategory)}>
                    {promptCategoryOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(option.label)}</SelectItem>)}
                  </SelectInput>
                ) : <div className="flex h-10 items-center"><Badge>{pt(promptCategoryLabel(form.category))}</Badge></div>}
              </Field>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <ToggleSwitch checked={form.enabled} label={pt("Enabled")} onCheckedChange={(checked) => updateForm("enabled", checked)} />
              {scopeKind === "WORKSPACE" && (mode === "create" || record?.sourceKind === "LOCAL") ? <ToggleSwitch checked={form.allowInheritance} label={pt("Allow inheritance")} onCheckedChange={(checked) => updateForm("allowInheritance", checked)} /> : null}
            </div>

            <Field label={pt("Description")}>
              <TextArea aria-label={pt("Description")} className="min-h-24 resize-y" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
            </Field>

            <Field label={pt("Prompt body")} error={analysis.invalidVariables.length ? pt("Invalid variables: {{variables}}", { variables: analysis.invalidVariables.join(", ") }) : undefined}>
              <TextArea aria-label={pt("Prompt body")} className="min-h-64 resize-y font-mono leading-6" value={form.promptBody} onChange={(event) => updatePromptBody(event.target.value)} />
            </Field>

            <div className="border-t border-[var(--color-outline-variant)] pt-5">
              <div className="mb-4 flex items-center justify-between gap-3">
                <h2 className="text-base font-semibold">{pt("Variables")}</h2>
                <Badge tone={analysis.invalidVariables.length || !variablesReady ? "error" : "info"}>{form.variables.length}</Badge>
              </div>
              {form.variables.length ? (
                <div className="divide-y divide-[var(--color-outline-variant)]">
                  {form.variables.map((variable) => (
                    <div key={variable.name} className="grid gap-4 py-4 first:pt-0 last:pb-0 md:grid-cols-[minmax(0,1fr)_180px_minmax(0,2fr)]">
                      <Field label={pt("Variable")}>
                        <div className="flex min-h-10 items-center">
                          <code className="min-w-0 break-all font-mono text-sm">{`{{${variable.name}}}`}</code>
                        </div>
                      </Field>
                      <Field label={<>{pt("Type")}<span className="sr-only">{` {{${variable.name}}}`}</span></>}>
                        <SelectInput
                          value={variable.type}
                          onValueChange={(value) => updateVariable(variable.name, { type: value as PromptVariableType })}
                        >
                          {promptVariableTypeOptions.map((option) => (
                            <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
                          ))}
                        </SelectInput>
                      </Field>
                      <Field
                        label={<>{pt("Description")}<span className="sr-only">{` {{${variable.name}}}`}</span></>}
                        error={!variable.description.trim() ? pt("Variable description is required") : undefined}
                      >
                        <TextInput
                          maxLength={promptVariableDescriptionMaxLength}
                          value={variable.description}
                          onChange={(event) => updateVariable(variable.name, { description: event.target.value })}
                        />
                      </Field>
                    </div>
                  ))}
                </div>
              ) : <p className="text-sm text-[var(--color-on-surface-variant)]">{pt("No variables detected")}</p>}
            </div>
          </SurfaceCard>
        )}
      </div>
    </>
  );
}

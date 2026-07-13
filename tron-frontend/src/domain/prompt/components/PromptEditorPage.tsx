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
  Play,
  Save,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import {
  Field,
  OptionSelect,
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
  setWorkspacePromptAllowInheritance,
  testPromptPreview,
  updatePlatformPrompt,
  updateWorkspacePrompt,
} from "@/domain/prompt/api";
import {
  promptCategoryLabel,
  promptCategoryOptions,
} from "@/domain/prompt/constants";
import { usePromptModelOptions } from "@/domain/prompt/hooks";
import {
  analyzePromptTemplateMock,
  createPlatformPromptMock,
  createWorkspacePromptMock,
  getPlatformPromptMock,
  getWorkspacePromptMock,
  setWorkspacePromptAllowInheritanceMock,
  testPromptPreviewMock,
  updatePlatformPromptMock,
  updateWorkspacePromptMock,
} from "@/domain/prompt/mock";
import {
  analyzePromptTemplate,
  type PromptAnalyzeResult,
  type PromptCategory,
  type PromptPreviewResult,
  type PromptRecord,
  type PromptScopeKind,
} from "@/domain/prompt/model";

interface PromptFormState {
  allowInheritance: boolean;
  category: PromptCategory;
  description: string;
  displayName: string;
  enabled: boolean;
  maxOutputTokens: string;
  modelCode: string;
  promptBody: string;
  temperature: number;
}

function emptyForm(): PromptFormState {
  return {
    allowInheritance: true,
    category: "SYSTEM",
    description: "",
    displayName: "",
    enabled: true,
    maxOutputTokens: "2048",
    modelCode: "",
    promptBody: "",
    temperature: 0.2,
  };
}

function formFromRecord(record: PromptRecord): PromptFormState {
  return {
    allowInheritance: record.allowInheritance ?? true,
    category: record.category,
    description: record.description ?? "",
    displayName: record.displayName,
    enabled: record.enabled,
    maxOutputTokens: record.maxOutputTokens === null ? "" : String(record.maxOutputTokens),
    modelCode: record.modelCode ?? "",
    promptBody: record.promptBody,
    temperature: record.temperature ?? 0.2,
  };
}

function numericOutputTokens(value: string) {
  if (!value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
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
  const [testing, setTesting] = useState(false);
  const [backendAnalysis, setBackendAnalysis] = useState<PromptAnalyzeResult | null>(null);
  const [variableValues, setVariableValues] = useState<Record<string, string>>({});
  const [preview, setPreview] = useState<PromptPreviewResult | null>(null);
  const localAnalysis = useMemo(() => analyzePromptTemplate(form.promptBody), [form.promptBody]);
  const analysis = backendAnalysis ?? {
    invalidVariables: localAnalysis.invalidVariables,
    variables: localAnalysis.variables.map((name) => ({ name, type: "STRING" as const })),
  };
  const { loading: modelsLoading, options: modelOptions } = usePromptModelOptions(scopeKind, workspaceCode);
  const listHref = scopeKind === "WORKSPACE" ? "/prompts" : "/settings/prompts";

  useEffect(() => {
    setRecord(null);
    setPreview(null);
    setVariableValues({});
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
    setPreview(null);
    setBackendAnalysis(null);
    const nextValues = Object.fromEntries(localAnalysis.variables.map((name) => [name, variableValues[name] ?? ""]));
    setVariableValues(nextValues);

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

  useEffect(() => {
    if (!modelOptions.length || form.modelCode) return;
    setForm((current) => ({ ...current, modelCode: modelOptions[0].modelCode }));
  }, [form.modelCode, modelOptions]);

  function updateForm<Key extends keyof PromptFormState>(key: Key, value: PromptFormState[Key]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  async function authoritativeAnalysis() {
    return promptDataSource === "api"
      ? analyzePromptTemplateApi(form.promptBody)
      : analyzePromptTemplateMock(form.promptBody);
  }

  async function savePrompt() {
    if (submitting || !form.displayName.trim() || !form.promptBody.trim()) return;
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
      const commonFields = {
        description: form.description.trim(),
        displayName: form.displayName.trim(),
        enabled: form.enabled,
        maxOutputTokens: numericOutputTokens(form.maxOutputTokens),
        modelCode: form.modelCode || null,
        promptBody: form.promptBody,
        temperature: form.temperature,
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
        const payload = { ...commonFields, expectedVersion: record.version, promptCode, workspaceCode };
        saved = promptDataSource === "api" ? await updateWorkspacePrompt(payload) : updateWorkspacePromptMock(payload);
        if (record?.sourceKind === "LOCAL" && form.allowInheritance !== record.allowInheritance) {
          const inheritancePayload = { allowInheritance: form.allowInheritance, promptCode, workspaceCode };
          saved = promptDataSource === "api"
            ? await setWorkspacePromptAllowInheritance(inheritancePayload)
            : setWorkspacePromptAllowInheritanceMock(inheritancePayload);
        }
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

  async function runPreview() {
    if (testing || !form.promptBody.trim()) return;
    setTesting(true);
    setPreview(null);
    try {
      const payload = {
        promptBody: form.promptBody,
        promptCode: promptCode || undefined,
        variables: variableValues,
        workspaceCode: workspaceCode || undefined,
      };
      const result = promptDataSource === "api"
        ? await testPromptPreview(payload)
        : testPromptPreviewMock(payload);
      setPreview(result);
      toast.success(pt("Preview completed"));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("Preview failed"));
    } finally {
      setTesting(false);
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
            <Button variant="outline" onClick={() => navigate(listHref)}>{pt("Cancel")}</Button>
            <Button variant="outline" disabled={testing || loading || !form.promptBody.trim()} onClick={() => void runPreview()}><Play size={15} aria-hidden="true" />{pt("Test")}</Button>
            <Button disabled={submitting || loading || !form.displayName.trim() || !form.promptBody.trim()} onClick={() => void savePrompt()}><Save size={15} aria-hidden="true" />{pt("Save")}</Button>
          </>
        )}
      />
      {scopeKind === "WORKSPACE" && !workspaceCode ? <SurfaceCard className="mt-5 text-sm text-[var(--color-on-surface-variant)]">{pt("Select a workspace to edit prompts.")}</SurfaceCard> : (
        <div className="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
          <div className="space-y-5">
            <SurfaceCard>
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
              {record ? (
                <div className="mt-4 flex flex-wrap items-center gap-2 border-t border-[var(--color-outline-variant)] pt-4 text-sm text-[var(--color-on-surface-variant)]">
                  <Badge tone={record.sourceKind === "OVERRIDE" ? "warning" : "info"}>{pt(record.sourceKind)}</Badge>
                  <span>{record.sourceLabel}</span>
                  {record.sourceWorkspacePath.length ? <span>{record.sourceWorkspacePath.map((item) => item.name).join(" / ")}</span> : null}
                </div>
              ) : null}
            </SurfaceCard>
            <SurfaceCard>
              <Field label={pt("Prompt body")} error={analysis.invalidVariables.length ? pt("Invalid variables: {{variables}}", { variables: analysis.invalidVariables.join(", ") }) : undefined}>
                <TextArea aria-label={pt("Prompt body")} className="min-h-80 resize-y font-mono leading-6" value={form.promptBody} onChange={(event) => updateForm("promptBody", event.target.value)} />
              </Field>
              <div className="mt-4">
                <Field label={pt("Description")}>
                  <TextArea aria-label={pt("Description")} className="min-h-24 resize-y" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
                </Field>
              </div>
            </SurfaceCard>
            {preview ? (
              <SurfaceCard>
                <h2 className="text-base font-semibold">{pt("Test Preview")}</h2>
                <div className="mt-3 text-xs text-[var(--color-on-surface-variant)]">{pt("Model")}: <span className="font-mono">{preview.modelCode ?? pt("Default")}</span></div>
                <div className="mt-4 grid gap-4 lg:grid-cols-2">
                  <div><h3 className="text-sm font-medium">{pt("Rendered prompt")}</h3><pre className="mt-2 max-h-80 overflow-auto whitespace-pre-wrap rounded-lg bg-[var(--color-surface-container-low)] p-4 text-sm leading-6">{preview.renderedPrompt}</pre></div>
                  <div><h3 className="text-sm font-medium">{pt("Output")}</h3><pre className="mt-2 max-h-80 overflow-auto whitespace-pre-wrap rounded-lg bg-[var(--color-surface-container-low)] p-4 text-sm leading-6">{preview.output}</pre></div>
                </div>
              </SurfaceCard>
            ) : null}
          </div>
          <div className="space-y-5">
            <SurfaceCard>
              <h2 className="mb-4 text-base font-semibold">{pt("Model Parameters")}</h2>
              <div className="grid gap-4">
                <Field label={pt("Model")}>
                  <OptionSelect ariaLabel={pt("Model")} emptyLabel={pt("No enabled text models")} loading={modelsLoading} onChange={(value) => updateForm("modelCode", value)} options={modelOptions.map((option) => ({ label: `${option.displayName} · ${option.sourceLabel}`, value: option.modelCode }))} value={form.modelCode} />
                </Field>
                <Field label={`${pt("Temperature")} ${form.temperature.toFixed(1)}`}>
                  <input className="w-full accent-[var(--color-primary)]" type="range" min={0} max={2} step={0.1} value={form.temperature} onChange={(event) => updateForm("temperature", Number(event.target.value))} />
                </Field>
                <Field label={pt("Max output tokens")}>
                  <TextInput min={1} step={1} type="number" value={form.maxOutputTokens} onChange={(event) => updateForm("maxOutputTokens", event.target.value)} />
                </Field>
                <ToggleSwitch checked={form.enabled} label={pt("Enabled")} onCheckedChange={(checked) => updateForm("enabled", checked)} />
                {scopeKind === "WORKSPACE" && (mode === "create" || record?.sourceKind === "LOCAL") ? <ToggleSwitch checked={form.allowInheritance} label={pt("Allow inheritance")} onCheckedChange={(checked) => updateForm("allowInheritance", checked)} /> : null}
              </div>
            </SurfaceCard>
            <SurfaceCard>
              <div className="mb-4 flex items-center justify-between gap-3">
                <h2 className="text-base font-semibold">{pt("Variables")}</h2>
                <Badge tone={analysis.invalidVariables.length ? "error" : "info"}>{analysis.variables.length}</Badge>
              </div>
              {analysis.variables.length ? <div className="grid gap-3">{analysis.variables.map((variable) => <Field key={variable.name} label={<span className="flex items-center justify-between gap-2"><code className="font-mono text-xs">{`{{${variable.name}}}`}</code><Badge tone="info">{variable.type}</Badge></span>}><TextInput aria-label={`{{${variable.name}}}`} value={variableValues[variable.name] ?? ""} onChange={(event) => setVariableValues((current) => ({ ...current, [variable.name]: event.target.value }))} /></Field>)}</div> : <p className="text-sm text-[var(--color-on-surface-variant)]">{pt("No variables detected")}</p>}
            </SurfaceCard>
          </div>
        </div>
      )}
    </>
  );
}

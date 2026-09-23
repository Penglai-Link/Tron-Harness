import {
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  useNavigate,
  useSearchParams,
} from "react-router";
import {
  Clipboard,
  LoaderCircle,
  Maximize2,
  Save,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { SelectItem } from "@/components/ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { Field, SelectInput, TextArea, TextInput } from "@/components/forms";
import { InfoTooltip } from "@/components/info-tooltip";
import { MarkdownContent } from "@/components/markdown-content";
import {
  PageHeader,
  SurfaceCard,
  useTabPanelConfig,
  useWorkspaceSpaces,
} from "@/shared";
import { useToast } from "@/components/ui/toast";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import {
  analyzePromptTemplateApi,
  createPlatformPrompt,
  createWorkspacePrompt,
  fetchPlatformPromptDetail,
  fetchWorkspacePromptDetail,
  updatePlatformPrompt,
  updateWorkspacePrompt,
  translatePromptDraft,
} from "@/domain/prompt/api";
import {
  PromptVariablePicker,
  findPromptVariablePickerTrigger,
  promptVariableAlias,
  type PromptVariablePickerTrigger,
} from "@/domain/prompt/components/PromptVariablePicker";
import {
  promptCategoryOptions,
  promptVariableTypeOptions,
} from "@/domain/prompt/constants";
import { usePromptTranslation } from "@/domain/prompt/hooks";
import {
  analyzePromptTemplateMock,
  createPlatformPromptMock,
  createWorkspacePromptMock,
  getPlatformPromptMock,
  getWorkspacePromptMock,
  updatePlatformPromptMock,
  updateWorkspacePromptMock,
  translatePromptDraftMock,
} from "@/domain/prompt/mock";
import {
  fetchPlatformPromptVariableOptions,
  fetchWorkspacePromptVariableOptions,
} from "@/domain/variable/api";
import {
  listPlatformPromptVariableOptions,
  listWorkspacePromptVariableOptions,
} from "@/domain/variable/mock/data";
import type { PromptVariableOption } from "@/domain/variable/model";
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
  type PromptTranslation,
  type PromptTranslationPayload,
  type PromptTranslationStatus,
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
  translations: PromptTranslation[];
}

interface PromptFormErrors {
  displayName?: string;
  promptBody?: string;
  variables?: string;
}

type PromptLocale = "en-US" | "zh-CN";

const promptLocales: Array<{ value: PromptLocale; label: string }> = [
  { value: "zh-CN", label: "简体中文 (zh-CN)" },
  { value: "en-US", label: "English (en-US)" },
];

function emptyTranslation(locale: PromptLocale): PromptTranslation {
  return {
    locale,
    displayName: "",
    promptBody: "",
    description: null,
    status: "DRAFT",
    sourceVersion: 1,
    updatedAt: new Date(0).toISOString(),
  };
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
    translations: promptLocales.map(({ value }) => emptyTranslation(value)),
  };
}

function formFromRecord(record: PromptRecord): PromptFormState {
  const translations = record.translations?.length
    ? record.translations
    : promptLocales.map(({ value }) => ({
      ...emptyTranslation(value),
      displayName: record.displayName,
      promptBody: record.promptBody,
      description: record.description,
      status: "PUBLISHED" as const,
      sourceVersion: record.version,
      updatedAt: record.updatedAt,
    }));
  const defaultTranslation = translations.find((item) => item.locale === "zh-CN")
    ?? translations.find((item) => item.locale === "en-US")
    ?? translations[0];
  return {
    allowInheritance: record.allowInheritance ?? true,
    category: record.category,
    description: defaultTranslation?.description ?? record.description ?? "",
    displayName: defaultTranslation?.displayName ?? record.displayName,
    enabled: record.enabled,
    promptBody: defaultTranslation?.promptBody ?? record.promptBody,
    variables: reconcilePromptVariables(
      defaultTranslation?.promptBody ?? record.promptBody,
      record.variables,
    ),
    translations,
  };
}

function requiredLabel(label: string) {
  return (
    <span>
      {label}
      <span aria-hidden="true" className="ml-1 text-[var(--color-error)]">*</span>
    </span>
  );
}

export function PromptEditorPage({
  mode,
  scopeKind,
  editable = mode === "create",
}: {
  mode: "create" | "detail";
  scopeKind: PromptScopeKind;
  editable?: boolean;
}) {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const pt = usePromptTranslation();
  const toast = useToast();
  const promptDataSource = getServiceDataSource("prompt");
  const variableDataSource = getServiceDataSource("variable");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const activeWorkspace = selectedSecondarySpace ?? selectedPrimarySpace;
  const workspaceCode = scopeKind === "WORKSPACE" ? activeWorkspace?.workspaceCode ?? "" : "";
  const promptCode = mode === "detail" ? searchParams.get("promptCode")?.trim() ?? "" : "";
  const isOverrideMode = searchParams.get("mode") === "override";
  const isReadOnlyDetail = mode === "detail" && !editable;
  const [form, setForm] = useState<PromptFormState>(() => emptyForm());
  const [activeLocale, setActiveLocale] = useState<PromptLocale>("zh-CN");
  const [record, setRecord] = useState<PromptRecord | null>(null);
  const [loading, setLoading] = useState(mode === "detail");
  const [submitting, setSubmitting] = useState(false);
  const [translating, setTranslating] = useState(false);
  const [dirtyLocales, setDirtyLocales] = useState<PromptLocale[]>([]);
  const [promptBodyFullscreenOpen, setPromptBodyFullscreenOpen] = useState(false);
  const [formErrors, setFormErrors] = useState<PromptFormErrors>({});
  const promptBodyInputRef = useRef<HTMLTextAreaElement>(null);
  const promptBodyFullscreenInputRef = useRef<HTMLTextAreaElement>(null);
  const promptVariablePickerRequestId = useRef(0);
  const [promptVariablePickerTrigger, setPromptVariablePickerTrigger] = useState<PromptVariablePickerTrigger | null>(null);
  const editorLocked = isReadOnlyDetail || loading || submitting || translating;
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
  const detailHref = scopeKind === "WORKSPACE" ? "/prompts/detail" : "/settings/prompts/detail";
  const editHref = scopeKind === "WORKSPACE" ? "/prompts/edit" : "/settings/prompts/edit";

  useTabPanelConfig({
    searchPlaceholder: pt("shell.searchPlaceholder", "Search prompts..."),
  });

  useEffect(() => {
    setRecord(null);
    setBackendAnalysis(null);
    setForm(emptyForm());
    setActiveLocale("zh-CN");
    setDirtyLocales([]);
    setTranslating(false);
    setFormErrors({});
    setPromptVariablePickerTrigger(null);

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
        setActiveLocale("zh-CN");
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) toast.error(reason instanceof Error ? reason.message : pt("errors.detailLoad", "Prompt could not be loaded"));
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
    if (editorLocked) return;
    if (key === "displayName") {
      setFormErrors((current) => ({ ...current, displayName: undefined }));
    }
    if (key === "displayName" || key === "promptBody" || key === "description") {
      setDirtyLocales((current) => current.includes(activeLocale) ? current : [...current, activeLocale]);
    }
    setForm((current) => {
      const next = { ...current, [key]: value };
      if (key === "displayName" || key === "promptBody" || key === "description") {
        next.translations = current.translations.map((translation) => (
          translation.locale === activeLocale
            ? {
              ...translation,
              ...(key === "displayName" ? { displayName: value as string } : {}),
              ...(key === "promptBody" ? { promptBody: value as string } : {}),
              ...(key === "description" ? { description: (value as string) || null } : {}),
              status: "DRAFT" as PromptTranslationStatus,
            }
            : { ...translation, status: "UPDATE_AVAILABLE" as PromptTranslationStatus }
        ));
      }
      return next;
    });
  }

  function updatePromptBody(
    promptBody: string,
    selectionStart?: number,
    showVariablePicker = false,
  ) {
    if (editorLocked) return;
    setFormErrors((current) => ({ ...current, promptBody: undefined }));
    if (showVariablePicker) {
      promptVariablePickerRequestId.current += 1;
      setPromptVariablePickerTrigger(findPromptVariablePickerTrigger(
        promptBody,
        selectionStart ?? promptBody.length,
        promptVariablePickerRequestId.current,
      ));
    } else {
      setPromptVariablePickerTrigger(null);
    }
    setDirtyLocales((current) => current.includes(activeLocale) ? current : [...current, activeLocale]);
    setForm((current) => ({
      ...current,
      promptBody,
      translations: current.translations.map((translation) => (
        translation.locale === activeLocale
          ? { ...translation, promptBody, status: "DRAFT" as PromptTranslationStatus }
          : { ...translation, status: "UPDATE_AVAILABLE" as PromptTranslationStatus }
      )),
      variables: reconcilePromptVariables(promptBody, current.variables),
    }));
  }

  function selectLocale(locale: PromptLocale) {
    if (loading || submitting || translating) return;
    const translation = form.translations.find((item) => item.locale === locale) ?? emptyTranslation(locale);
    setActiveLocale(locale);
    setFormErrors({});
    setPromptVariablePickerTrigger(null);
    setForm((current) => ({
      ...current,
      displayName: translation.displayName,
      promptBody: translation.promptBody,
      description: translation.description ?? "",
      variables: reconcilePromptVariables(translation.promptBody, current.variables),
    }));
  }

  async function translateAndSyncOtherLocales() {
    if (editorLocked) return;
    const sourceLocale = activeLocale;
    const targetLocales = promptLocales
      .map(({ value }) => value)
      .filter((locale) => locale !== sourceLocale);
    setTranslating(true);
    try {
      const drafts = await Promise.all(targetLocales.map(async (targetLocale) => {
        const draft = promptDataSource === "api"
          ? await translatePromptDraft({
            promptCode: promptCode || undefined,
            workspaceCode: activeWorkspace?.workspaceCode || undefined,
            sourceLocale,
            targetLocale,
            displayName: form.displayName,
            promptBody: form.promptBody,
            description: form.description || null,
          })
          : translatePromptDraftMock({
            sourceLocale,
            targetLocale,
            displayName: form.displayName,
            promptBody: form.promptBody,
            description: form.description || null,
          });
        return [targetLocale, draft] as const;
      }));
      const draftsByLocale = new Map(drafts);
      setForm((current) => ({
        ...current,
        translations: current.translations.map((item) => draftsByLocale.get(item.locale) ?? item),
      }));
      setDirtyLocales((current) => Array.from(new Set([...current, ...targetLocales])));
      toast.success(pt("messages.translationDraftCreated", "Other languages translated and synchronized from the current language"));
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.translation", "Translation failed"));
    } finally {
      setTranslating(false);
    }
  }

  function updateVariable(name: string, patch: Partial<Pick<PromptVariable, "description" | "type">>) {
    if (editorLocked) return;
    setForm((current) => ({
      ...current,
      variables: current.variables.map((variable) => (
        variable.name === name ? { ...variable, ...patch } : variable
      )),
    }));
  }

  function selectPromptVariable(option: PromptVariableOption) {
    const trigger = promptVariablePickerTrigger;
    if (editorLocked || !trigger) return;
    if (option.sourceKind === "RUNTIME" && !option.runtimeProvider) return;
    const variableName = option.sourceKind === "RUNTIME"
      ? option.key
      : promptVariableAlias(option.key, form.variables);
    const replacement = `{{${variableName}}}`;
    const promptBody = `${form.promptBody.slice(0, trigger.start)}${replacement}${form.promptBody.slice(trigger.end)}`;
    const description = option.description?.trim() || `Variable ${option.key}`;
    const nextCaret = trigger.start + replacement.length;

    setFormErrors((current) => ({ ...current, promptBody: undefined, variables: undefined }));
    setDirtyLocales((current) => current.includes(activeLocale) ? current : [...current, activeLocale]);
    setForm((current) => ({
      ...current,
      promptBody,
      translations: current.translations.map((translation) => (
        translation.locale === activeLocale
          ? { ...translation, promptBody, status: "DRAFT" as PromptTranslationStatus }
          : { ...translation, status: "UPDATE_AVAILABLE" as PromptTranslationStatus }
      )),
      variables: reconcilePromptVariables(promptBody, current.variables).map((variable) => (
        variable.name === variableName
          ? {
            ...variable,
            description: variable.description.trim() || description,
            type: option.sourceKind === "RUNTIME" ? option.type : variable.type,
            sourceKind: option.sourceKind,
            managedVariableKey: option.managedVariableKey,
            runtimeProvider: option.runtimeProvider,
          }
          : variable
      )),
    }));
    setPromptVariablePickerTrigger(null);
    window.requestAnimationFrame(() => {
      const activeInput = promptBodyFullscreenOpen
        ? promptBodyFullscreenInputRef.current
        : promptBodyInputRef.current;
      activeInput?.focus();
      activeInput?.setSelectionRange(nextCaret, nextCaret);
    });
  }

  async function validateManagedVariableBindings(variables: PromptVariable[]) {
    const managedVariables = variables.filter((variable) => variable.sourceKind === "MANAGED");
    if (!managedVariables.length) return;

    const options = variableDataSource === "api"
      ? (scopeKind === "WORKSPACE"
        ? await fetchWorkspacePromptVariableOptions(workspaceCode)
        : await fetchPlatformPromptVariableOptions())
      : (scopeKind === "WORKSPACE"
        ? listWorkspacePromptVariableOptions(workspaceCode)
        : listPlatformPromptVariableOptions());
    const availableKeys = new Set(options
      .filter((option) => option.sourceKind === "MANAGED")
      .map((option) => option.managedVariableKey ?? option.key));
    const unavailable = managedVariables.find((variable) => (
      !variable.managedVariableKey || !availableKeys.has(variable.managedVariableKey)
    ));
    if (unavailable) {
      throw new Error(pt(
        "validation.variableBindingUnavailable",
        "A referenced variable management entry is unavailable. Reinsert it before saving.",
      ));
    }
  }

  async function authoritativeAnalysis() {
    return promptDataSource === "api"
      ? analyzePromptTemplateApi(form.promptBody)
      : analyzePromptTemplateMock(form.promptBody);
  }

  async function savePrompt() {
    if (editorLocked) return;
    const nextErrors: PromptFormErrors = {
      displayName: form.displayName.trim() ? undefined : pt("validation.nameRequired", "Name is required"),
      promptBody: form.promptBody.trim() ? undefined : pt("validation.promptBodyRequired", "Prompt body is required"),
    };
    setFormErrors(nextErrors);
    if (nextErrors.displayName || nextErrors.promptBody) {
      toast.error(pt("validation.requiredFields", "Complete the required fields before saving"));
      return;
    }
    if (!variablesReady) {
      const variables = pt(
        "validation.variableBindingRequired",
        "Every placeholder must be bound to an available variable management entry or runtime variable.",
      );
      setFormErrors((current) => ({ ...current, variables }));
      toast.error(variables);
      return;
    }
    if (scopeKind === "WORKSPACE" && !workspaceCode) {
      toast.error(pt("errors.workspaceRequiredForSave", "Select a workspace before saving"));
      return;
    }
    setSubmitting(true);
    try {
      const checked = await authoritativeAnalysis();
      setBackendAnalysis(checked);
      if (checked.invalidVariables.length) {
        toast.error(pt("errors.invalidVariables", "Fix invalid variables before saving"));
        return;
      }
      const variables = normalizePromptVariables(form.promptBody, form.variables);
      await validateManagedVariableBindings(variables);
      const fallbackTranslation = {
        locale: "en-US" as const,
        displayName: form.displayName.trim(),
        promptBody: form.promptBody,
        description: form.description.trim() || null,
        status: "DRAFT" as const,
        sourceVersion: record?.version ?? 1,
        updatedAt: new Date().toISOString(),
      };
      const translations: PromptTranslationPayload[] = promptLocales.map(({ value }) => {
        const existing = form.translations.find((translation) => translation.locale === value);
        const translation = existing?.displayName.trim() && existing.promptBody.trim()
          ? existing
          : { ...fallbackTranslation, locale: value, status: "DRAFT" as const };
        return {
          locale: translation.locale,
          displayName: translation.displayName,
          promptBody: translation.promptBody,
          description: translation.description,
          status: translation.status,
        };
      });
      const commonFields = {
        description: form.description.trim(),
        displayName: form.displayName.trim(),
        enabled: form.enabled,
        promptBody: form.promptBody,
        variables,
        translations,
      };

      let saved: PromptRecord;
      if (mode === "create" && scopeKind === "WORKSPACE") {
        const payload = { ...commonFields, allowInheritance: form.allowInheritance, category: form.category, workspaceCode };
        saved = promptDataSource === "api" ? await createWorkspacePrompt(payload) : createWorkspacePromptMock(payload);
      } else if (mode === "create") {
        const payload = { ...commonFields, category: form.category };
        saved = promptDataSource === "api" ? await createPlatformPrompt(payload) : createPlatformPromptMock(payload);
      } else if (scopeKind === "WORKSPACE") {
        if (!record) throw new Error(pt("errors.detailsNotLoaded", "Prompt details are not loaded"));
        const payload = {
          ...commonFields,
          allowInheritance: record.sourceKind === "LOCAL" ? form.allowInheritance : undefined,
          expectedVersion: record.version,
          promptCode,
          workspaceCode,
        };
        saved = promptDataSource === "api" ? await updateWorkspacePrompt(payload) : updateWorkspacePromptMock(payload);
      } else {
        if (!record) throw new Error(pt("errors.detailsNotLoaded", "Prompt details are not loaded"));
        const payload = { ...commonFields, expectedVersion: record.version, promptCode };
        saved = promptDataSource === "api" ? await updatePlatformPrompt(payload) : updatePlatformPromptMock(payload);
      }

      toast.success(isOverrideMode
        ? pt("messages.overrideSaved", "Override saved")
        : pt("messages.saved", "Prompt saved"));
      const params = new URLSearchParams({ promptCode: saved.promptCode });
      navigate(`${scopeKind === "WORKSPACE" ? "/prompts/detail" : "/settings/prompts/detail"}?${params.toString()}`);
    } catch (reason) {
      toast.error(reason instanceof Error ? reason.message : pt("errors.save", "Prompt could not be saved"));
    } finally {
      setSubmitting(false);
    }
  }

  const title = mode === "create"
    ? (scopeKind === "WORKSPACE"
      ? pt("titles.createWorkspace", "Create Prompt")
      : pt("titles.createPlatform", "Create Platform Prompt"))
    : (isOverrideMode
      ? pt("titles.createOverride", "Create Prompt Override")
      : editable
        ? (scopeKind === "WORKSPACE"
          ? pt("titles.editWorkspace", "Edit Prompt")
          : pt("titles.editPlatform", "Edit Platform Prompt"))
        : pt("titles.details", "Prompt Details"));
  const cancelHref = mode === "detail"
    ? `${detailHref}?${new URLSearchParams({ promptCode }).toString()}`
    : listHref;
  const editUrl = `${editHref}?${new URLSearchParams({
    ...(promptCode ? { promptCode } : {}),
    ...(isOverrideMode ? { mode: "override" } : {}),
  }).toString()}`;
  const categoryField = (
    <Field label={requiredLabel(pt("fields.category", "Category"))}>
      <SelectInput
        disabled={mode === "detail" || editorLocked}
        value={form.category}
        onValueChange={(value) => updateForm("category", value as PromptCategory)}
      >
        {promptCategoryOptions.map((option) => <SelectItem key={option.value} value={option.value}>{pt(`categories.${option.value}`, option.label)}</SelectItem>)}
      </SelectInput>
    </Field>
  );
  const enabledField = (
    <Field label={requiredLabel(pt("fields.status", "Status"))}>
      <SelectInput
        disabled={editorLocked}
        value={form.enabled ? "true" : "false"}
        onValueChange={(value) => updateForm("enabled", value === "true")}
      >
        <SelectItem value="true">{pt("status.enabled", "Enabled")}</SelectItem>
        <SelectItem value="false">{pt("status.disabled", "Disabled")}</SelectItem>
      </SelectInput>
    </Field>
  );
  const inheritanceField = (
    <Field label={requiredLabel(pt("fields.allowInheritance", "Allow inheritance"))}>
      <SelectInput
        disabled={editorLocked}
        value={form.allowInheritance ? "true" : "false"}
        onValueChange={(value) => updateForm("allowInheritance", value === "true")}
      >
        <SelectItem value="true">{pt("status.inheritanceAllowed", "Allowed")}</SelectItem>
        <SelectItem value="false">{pt("status.inheritanceDisallowed", "Not allowed")}</SelectItem>
      </SelectInput>
    </Field>
  );

  return (
    <>
      <PageHeader
        breadcrumbs={[{
          label: scopeKind === "WORKSPACE"
            ? pt("titles.workspace", "Prompts")
            : pt("titles.platform", "Platform Prompts"),
          href: listHref,
        }, { label: title }]}
        title={title}
        actions={(
          isReadOnlyDetail ? (
            <>
              <Button size="lg" variant="outline" onClick={() => navigate(listHref)}>{pt("actions.back", "Back")}</Button>
              <Button size="lg" onClick={() => navigate(editUrl)}>{isOverrideMode ? pt("actions.edit", "Edit") : pt("actions.edit", "Edit")}</Button>
            </>
          ) : (
            <>
              <Tooltip>
                <TooltipTrigger
                  render={(
                    <Button
                      type="button"
                      size="lg"
                      variant="outline"
                      onClick={() => void translateAndSyncOtherLocales()}
                      disabled={editorLocked || !form.displayName.trim() || !form.promptBody.trim()}
                      aria-busy={translating}
                    >
                      {translating ? <LoaderCircle size={16} className="mr-1 animate-spin" aria-hidden="true" /> : null}
                      {translating ? pt("actions.translating", "Translating...") : pt("actions.translate", "Translate and sync")}
                    </Button>
                  )}
                />
                <TooltipContent>{pt("hints.translationSync", "Uses the current language to translate and overwrite the other languages.")}</TooltipContent>
              </Tooltip>
              <Button size="lg" variant="outline" disabled={editorLocked} onClick={() => navigate(cancelHref)}>{pt("actions.cancel", "Cancel")}</Button>
              <Button
                size="lg"
                disabled={editorLocked}
                aria-busy={submitting}
                onClick={() => void savePrompt()}
              >
                <Save size={16} aria-hidden="true" />
                {submitting ? pt("actions.saving", "Saving...") : pt("actions.save", "Save")}
              </Button>
            </>
          )
        )}
      />
      <div className="w-full">
        {scopeKind === "WORKSPACE" && !workspaceCode ? (
          <SurfaceCard className="text-sm text-[var(--color-on-surface-variant)]">{pt("empty.workspaceEditor", "Select a workspace to edit prompts.")}</SurfaceCard>
        ) : (
          <SurfaceCard className="space-y-5">
            {record ? (
              <div className="grid gap-4 border-b border-[var(--color-outline-variant)] pb-5 sm:grid-cols-2 lg:grid-cols-3">
                <div>
                  <p className="text-xs font-medium text-[var(--color-on-surface-variant)]">{pt("fields.source", "Source")}</p>
                  <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{record.sourceLabel}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-[var(--color-on-surface-variant)]">{pt("fields.sourceType", "Source type")}</p>
                  <p className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{pt(`sources.${record.sourceKind}`, record.sourceKind)}</p>
                </div>
                {record.sourceWorkspacePath.length ? (
                  <div>
                    <p className="text-xs font-medium text-[var(--color-on-surface-variant)]">{pt("fields.workspacePath", "Workspace path")}</p>
                    <p className="mt-1 break-words text-sm text-[var(--color-on-surface-variant)]">{record.sourceWorkspacePath.map((item) => item.name).join(" / ")}</p>
                  </div>
                ) : null}
              </div>
            ) : null}

            <div className="grid gap-4 md:grid-cols-2 md:items-end">
              <Field label={requiredLabel(pt("fields.language", "Language"))} hint={pt("hints.translationReview", "Edit one language at a time. Translation uses the current language to overwrite and synchronize the other languages.")}>
                <SelectInput
                  aria-label={pt("fields.language", "Language")}
                  disabled={loading || submitting || translating}
                  value={activeLocale}
                  onValueChange={(value) => selectLocale(value as PromptLocale)}
                >
                  {promptLocales.map((locale) => {
                    const translation = form.translations.find((item) => item.locale === locale.value);
                    const statusHint = translation?.status === "UPDATE_AVAILABLE"
                      ? ` · ${pt("status.translationUpdateAvailable", "Needs translation")}`
                      : "";
                    return (
                      <SelectItem key={locale.value} value={locale.value}>
                        {locale.label}{dirtyLocales.includes(locale.value) ? " *" : ""}{statusHint}
                      </SelectItem>
                    );
                  })}
                </SelectInput>
              </Field>

              {mode === "detail" ? (
                <Field label={pt("fields.promptCode", "Prompt code")}>
                  <div className="relative w-full min-w-0">
                    <TextInput aria-label={pt("fields.promptCode", "Prompt code")} className="w-full min-w-0 pr-12 font-mono" disabled readOnly value={record?.promptCode ?? promptCode} />
                    <Button className="absolute right-1 top-1/2 -translate-y-1/2" size="icon" variant="ghost" disabled={loading || submitting || translating} aria-label={pt("aria.copyPromptCode", "Copy prompt code")} onClick={() => { void navigator.clipboard?.writeText(record?.promptCode ?? promptCode); toast.success(pt("messages.codeCopied", "Prompt code copied")); }}><Clipboard size={15} aria-hidden="true" /></Button>
                  </div>
                </Field>
              ) : categoryField}

              <Field label={requiredLabel(pt("fields.name", "Name"))}>
                <TextInput aria-invalid={Boolean(formErrors.displayName)} aria-required="true" disabled={editorLocked} value={form.displayName} onChange={(event) => updateForm("displayName", event.target.value)} />
                {formErrors.displayName ? <p className="text-sm text-destructive-foreground">{formErrors.displayName}</p> : null}
              </Field>

              {mode === "detail" ? categoryField : enabledField}
              {mode === "detail" ? enabledField : null}
              {scopeKind === "WORKSPACE" && (mode === "create" || record?.sourceKind === "LOCAL") ? inheritanceField : null}
            </div>

            <Field label={pt("fields.description", "Description")}>
              <TextArea disabled={editorLocked} aria-label={pt("fields.description", "Description")} className="min-h-24 resize-y" value={form.description} onChange={(event) => updateForm("description", event.target.value)} />
            </Field>

            <div className="grid gap-1.5">
              <div className="flex flex-wrap items-center justify-between gap-[12px]">
                <span className="flex items-center gap-1.5 text-sm font-medium text-[var(--color-on-surface)]">
                  {requiredLabel(pt("fields.promptBody", "Prompt body"))}
                  <InfoTooltip ariaLabel={pt("aria.promptBodyVariablesHelp", "Prompt variable usage help")}>
                    {pt("hints.promptBodyVariables", "Type {{ to search and select an available variable management entry or runtime variable. Sensitive variables cannot be inserted into prompts.")}
                  </InfoTooltip>
                </span>
                <Button
                  size="sm"
                  type="button"
                  variant="ghost"
                  aria-label={pt("aria.promptBodyFullscreen", "Open prompt body fullscreen")}
                  onClick={() => setPromptBodyFullscreenOpen(true)}
                >
                  <Maximize2 size={14} aria-hidden="true" />
                  {pt("actions.fullscreen", "Fullscreen")}
                </Button>
              </div>
              {isReadOnlyDetail ? (
                <MarkdownContent
                  ariaLabel={pt("aria.promptBodyMarkdown", "Prompt body markdown")}
                  className="max-h-96 min-h-32 overflow-y-auto rounded-lg border border-[var(--color-outline-variant)] px-4 py-3"
                  content={form.promptBody || pt("empty.noPromptBody", "No prompt body")}
                />
              ) : (
                <div>
                  <TextArea
                    ref={promptBodyInputRef}
                    aria-expanded={Boolean(promptVariablePickerTrigger)}
                    aria-required="true"
                    disabled={editorLocked}
                    aria-label={pt("fields.promptBody", "Prompt body")}
                    className="min-h-64 resize-y font-mono leading-6"
                    value={form.promptBody}
                    onKeyDown={(event) => {
                      if (event.key === "Escape") setPromptVariablePickerTrigger(null);
                    }}
                    onChange={(event) => updatePromptBody(
                      event.target.value,
                      event.target.selectionStart ?? event.target.value.length,
                      true,
                    )}
                  />
                  <PromptVariablePicker
                    anchorElement={promptBodyFullscreenOpen
                      ? promptBodyFullscreenInputRef.current
                      : promptBodyInputRef.current}
                    dataSource={variableDataSource}
                    labels={{
                      empty: pt("empty.noPromptVariableOptions", "No variable management entries or runtime variables are available"),
                      loading: pt("status.variableOptionsLoading", "Loading variables..."),
                      managed: pt("status.managedVariable", "Variable management"),
                      noDescription: pt("empty.noVariableDescription", "No description"),
                      runtime: pt("status.runtimeVariable", "Runtime"),
                      runtimeDescription: (provider, fallback) => pt(`runtimeVariables.${provider}.description`, fallback),
                      runtimeSourceLabel: (provider, fallback) => pt(`runtimeVariables.${provider}.sourceLabel`, fallback),
                      searchPlaceholder: pt("hints.variableOptionSearch", "Search variables"),
                    }}
                    scopeKind={scopeKind}
                    trigger={promptVariablePickerTrigger}
                    workspaceCode={workspaceCode}
                    onDismiss={() => setPromptVariablePickerTrigger(null)}
                    onSelect={selectPromptVariable}
                  />
                </div>
              )}
              {formErrors.promptBody ? <p className="text-sm text-destructive-foreground">{formErrors.promptBody}</p> : null}
              {analysis.invalidVariables.length ? (
                <p className="text-sm text-destructive-foreground">{pt("validation.invalidVariables", "Invalid variables: {{variables}}", { variables: analysis.invalidVariables.join(", ") })}</p>
              ) : null}
              {formErrors.variables ? <p className="text-sm text-destructive-foreground">{formErrors.variables}</p> : null}
            </div>

            <div className="border-t border-[var(--color-outline-variant)] pt-5">
              <div className="mb-4 flex items-center justify-between gap-3">
                <h2 className="text-base font-semibold">{pt("fields.variables", "Variables")}</h2>
                <span className="text-sm text-[var(--color-on-surface-variant)]">{form.variables.length}</span>
              </div>
              {form.variables.length ? (
                <div className="overflow-hidden rounded-md border border-[var(--color-outline-variant)]">
                  <div className="hidden bg-[var(--color-surface-container-low)] px-4 py-2 text-xs font-semibold text-[var(--color-on-surface-variant)] md:grid md:grid-cols-[minmax(130px,0.7fr)_minmax(180px,1fr)_150px_minmax(200px,1.3fr)] md:gap-4">
                    <span>{pt("fields.variable", "Variable")}</span>
                    <span>{pt("fields.variableBinding", "Value source")}</span>
                    <span>{pt("fields.type", "Type")}</span>
                    <span>{pt("fields.description", "Description")}</span>
                  </div>
                  <div className="divide-y divide-[var(--color-outline-variant)]">
                    {form.variables.map((variable) => (
                      <div key={variable.name} className="grid gap-4 px-4 py-3 md:grid-cols-[minmax(130px,0.7fr)_minmax(180px,1fr)_150px_minmax(200px,1.3fr)] md:items-start">
                        <div className="space-y-2 md:space-y-0">
                          <span className="block text-sm font-medium md:hidden">{pt("fields.variable", "Variable")}</span>
                          <code className="flex min-h-10 min-w-0 items-center break-all font-mono text-sm font-medium">{variable.name}</code>
                        </div>
                        <div className="space-y-2 md:space-y-0">
                          <span className="block text-sm font-medium md:hidden">{pt("fields.variableBinding", "Value source")}</span>
                          {variable.sourceKind === "MANAGED" && variable.managedVariableKey ? (
                            <div className="flex min-h-10 min-w-0 items-center">
                              <code className="break-all font-mono text-sm text-[var(--color-on-surface-variant)]">{variable.managedVariableKey}</code>
                            </div>
                          ) : variable.sourceKind === "RUNTIME" ? (
                            <span className="flex min-h-10 flex-col justify-center gap-0.5 text-sm text-primary">
                              <span>{pt("status.runtimeVariable", "System runtime variable")}</span>
                              {variable.runtimeProvider ? (
                                <span className="break-all font-mono text-xs">{variable.runtimeProvider}</span>
                              ) : null}
                            </span>
                          ) : (
                            <span className="flex min-h-10 items-center text-sm text-[var(--color-on-surface-variant)]">—</span>
                          )}
                        </div>
                        <div className="space-y-2 md:space-y-0">
                          <span className="block text-sm font-medium md:hidden">{pt("fields.type", "Type")}</span>
                          <SelectInput
                            aria-label={pt("aria.variableType", "Type {{name}}", { name: variable.name })}
                            disabled={editorLocked || variable.sourceKind === "RUNTIME"}
                            value={variable.type}
                            onValueChange={(value) => updateVariable(variable.name, { type: value as PromptVariableType })}
                          >
                            {promptVariableTypeOptions.map((option) => (
                              <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
                            ))}
                          </SelectInput>
                        </div>
                        <div className="space-y-2 md:space-y-0">
                          <span className="block text-sm font-medium md:hidden">{pt("fields.description", "Description")}</span>
                          <TextInput
                            aria-label={pt("aria.variableDescription", "Description {{name}}", { name: variable.name })}
                            disabled={editorLocked || variable.sourceKind === "RUNTIME"}
                            maxLength={promptVariableDescriptionMaxLength}
                            value={variable.sourceKind === "RUNTIME" && variable.runtimeProvider
                              ? pt(`runtimeVariables.${variable.runtimeProvider}.description`, variable.description)
                              : variable.description}
                            onChange={(event) => updateVariable(variable.name, { description: event.target.value })}
                          />
                          {!variable.description.trim() ? <p className="mt-2 text-xs text-destructive-foreground">{pt("validation.variableDescriptionRequired", "Variable description is required")}</p> : null}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ) : <p className="text-sm text-[var(--color-on-surface-variant)]">{pt("empty.noVariables", "No variables detected")}</p>}
            </div>
          </SurfaceCard>
        )}
      </div>
      {promptBodyFullscreenOpen ? createPortal(
        <div className="fixed inset-0 z-[120] flex items-center justify-center bg-black/55 p-4 backdrop-blur-sm">
          <div
            className="flex h-[min(88vh,64rem)] w-[min(96vw,72rem)] flex-col rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_20px_60px_rgba(0,0,0,0.28)]"
            role="dialog"
            aria-modal="true"
            aria-label={isReadOnlyDetail ? pt("aria.promptBodyFullscreenPreview", "Prompt body fullscreen preview") : pt("aria.promptBodyFullscreenEditor", "Prompt body fullscreen editor")}
          >
            <div className="flex items-center justify-between gap-3 border-b border-[var(--color-outline-variant)] px-5 py-4">
              <h2 className="text-base font-semibold text-[var(--color-on-surface)]">{pt("fields.promptBody", "Prompt body")}</h2>
              <Button
                size="icon"
                type="button"
                variant="ghost"
                aria-label={pt("aria.closePromptBodyFullscreen", "Close prompt body fullscreen")}
                onClick={() => {
                  setPromptBodyFullscreenOpen(false);
                  setPromptVariablePickerTrigger(null);
                }}
              >
                <X size={16} aria-hidden="true" />
              </Button>
            </div>
            <div className="min-h-0 flex-1 px-5 py-4">
              {isReadOnlyDetail ? (
                <div className="h-full overflow-y-auto pr-1">
                  <MarkdownContent ariaLabel={pt("aria.promptBodyMarkdownFullscreen", "Prompt body markdown fullscreen")} className="max-w-none" content={form.promptBody || pt("empty.noPromptBody", "No prompt body")} />
                </div>
              ) : (
                <TextArea
                  ref={promptBodyFullscreenInputRef}
                  aria-expanded={Boolean(promptVariablePickerTrigger)}
                  aria-label={pt("aria.promptBodyFullscreenEditor", "Prompt body fullscreen editor")}
                  className="h-full min-h-0 w-full resize-none"
                  disabled={editorLocked}
                  value={form.promptBody}
                  onKeyDown={(event) => {
                    if (event.key === "Escape") setPromptVariablePickerTrigger(null);
                  }}
                  onChange={(event) => updatePromptBody(
                    event.target.value,
                    event.target.selectionStart ?? event.target.value.length,
                    true,
                  )}
                />
              )}
            </div>
          </div>
        </div>,
        document.body,
      ) : null}
    </>
  );
}

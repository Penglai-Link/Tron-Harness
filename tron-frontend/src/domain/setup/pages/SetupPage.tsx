import { useCallback, useEffect, useMemo, useState } from "react";
import { Check, ChevronRight, Eye, EyeOff } from "lucide-react";
import { useNavigate } from "react-router";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Field, TextArea, TextInput } from "@/components/forms";
import { useToast } from "@/components/ui/toast";
import { useAuthStore } from "@/store/authStore";
import { completeSetup, createSetupSession, fetchSetupStatus, type SetupStatus } from "@/domain/setup/api";
import { useLocale, type SupportedLocale } from "@/i18n";
import { isApiError } from "@/shared/api/httpClient";
import { registerProfile } from "@/domain/profile/api";
import {
  verifyPlatformModelDraft,
  type ModelMutationPayload,
} from "@/domain/model/api/modelApi";
import {
  createModelProviderAuthorization,
  pollModelProviderAuthorization,
} from "@/domain/model/api/modelProviderApi";
import {
  ModelFormFields,
  emptyModelFormState,
  modelFormToPayload,
  type ModelFormFieldErrors,
  type ModelFormState,
} from "@/domain/model/components";
import {
  getDefaultApiPathForProtocol,
  getDefaultModelCapabilitiesForType,
} from "@/domain/model/constants/modelPageOptions";
import type { ModelType } from "@/domain/model/model";

type SetupStep = "admin" | "crypto" | "workspace" | "models" | "review";
type SetupModelType = Exclude<ModelType, "IMAGE">;

const INITIAL_PASSWORD_PATTERN = /^(?=.{12,}$)(?=.*[A-Z])(?=.*[a-z])(?=.*\d)(?=.*[^A-Za-z0-9]).+$/;

interface SetupDraft {
  newPassword: string;
  confirmPassword: string;
  encryptionKey: string;
  displayName?: string;
  workspaceName?: string;
  workspaceDescription?: string;
  modelAccess: "Manual" | "PersonAi";
  personAiAuthorization?: PersonAiSetupAuthorization;
  models: Partial<Record<SetupModelType, Partial<ModelFormState>>>;
  rerankConfigured?: boolean;
  verified: Record<SetupModelType, boolean>;
  step?: SetupStep;
}

interface PersonAiSetupAuthorization {
  sessionId: string;
  loginUrl: string;
  status: string;
  verified: boolean;
}

// The setup wizard deliberately keeps its draft only in the running frontend
// process. A reload starts a new setup attempt; navigating between steps or a
// setup-session auth update must not discard the current draft.
const setupDraft: SetupDraft = {
  newPassword: "",
  confirmPassword: "",
  encryptionKey: "",
  modelAccess: "PersonAi",
  models: {},
  rerankConfigured: false,
  verified: { TEXT: false, EMBEDDING: false, RERANK: false },
};

function clearSetupDraft() {
  setupDraft.newPassword = "";
  setupDraft.confirmPassword = "";
  setupDraft.encryptionKey = "";
  setupDraft.displayName = undefined;
  setupDraft.workspaceName = undefined;
  setupDraft.workspaceDescription = undefined;
  setupDraft.modelAccess = "PersonAi";
  setupDraft.personAiAuthorization = undefined;
  setupDraft.models = {};
  setupDraft.rerankConfigured = false;
  setupDraft.verified = { TEXT: false, EMBEDDING: false, RERANK: false };
  setupDraft.step = undefined;
}

function resolveSetupError(
  error: unknown,
  translate: (key: string) => string,
  fallbackKey: string,
) {
  if (isApiError(error)) {
    const messageKeys: Record<string, string> = {
      SETUP_ADMIN_ALREADY_ACTIVATED: "errors.adminAlreadyActivated",
      SETUP_ADMIN_PASSWORD_INVALID: "errors.adminValidation",
      SETUP_ADMIN_PASSWORD_REQUIRED: "errors.adminValidation",
      SETUP_NOT_READY: "errors.setupNotReady",
      SETUP_WORKSPACE_ALREADY_EXISTS: "errors.workspaceAlreadyExists",
      SETUP_WORKSPACE_MULTIPLE: "errors.workspaceAlreadyExists",
      SETUP_CRYPTO_INVALID: "errors.cryptoFormat",
      SETUP_PERSON_AI_NOT_READY: "errors.personAiNotReady",
    };
    const messageKey = error.code ? messageKeys[error.code] : undefined;
    if (messageKey) return translate(messageKey);
  }
  return error instanceof Error ? error.message : translate(fallbackKey);
}

const STEP_ORDER: Array<{ id: SetupStep; labelKey: string }> = [
  { id: "admin", labelKey: "steps.admin.label" },
  { id: "crypto", labelKey: "steps.crypto.label" },
  { id: "workspace", labelKey: "steps.workspace.label" },
  { id: "models", labelKey: "steps.models.label" },
  { id: "review", labelKey: "steps.review.label" },
];

function emptyModel(type: SetupModelType, displayName: string): ModelFormState {
  const protocol = type === "RERANK"
    ? "RERANK"
    : type === "EMBEDDING"
      ? "OPENAI_COMPATIBLE_EMBEDDING"
      : "OPENAI_COMPATIBLE";
  const textModel = type === "TEXT";
  return {
    ...emptyModelFormState(),
    displayName,
    apiKeyRef: "",
    apiPath: getDefaultApiPathForProtocol(protocol, type),
    modelCapabilities: getDefaultModelCapabilitiesForType(type),
    modelType: type,
    protocol,
    supportedReasoningEfforts: textModel ? ["Low", "Medium", "High"] : [],
    defaultReasoningEffort: textModel ? "Medium" : null,
    structuredOutputEnabled: textModel,
    thinkingEnabled: textModel,
  };
}

function makeKey() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return btoa(String.fromCharCode(...bytes));
}

function modelPayload(draft: ModelFormState): ModelMutationPayload {
  return modelFormToPayload(draft, draft.apiKeyRef, {
    enabled: true,
    allowInheritance: false,
  });
}

export function SetupPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("setup");
  const { changeLocale, currentLocale } = useLocale();
  const authStatus = useAuthStore((state) => state.status);
  const acceptAuthGrant = useAuthStore((state) => state.acceptAuthGrant);
  const [status, setStatus] = useState<SetupStatus | null>(null);
  const [step, setStepState] = useState<SetupStep>(() => setupDraft.step ?? "admin");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [newPassword, setNewPasswordState] = useState(() => setupDraft.newPassword);
  const [confirmPassword, setConfirmPasswordState] = useState(() => setupDraft.confirmPassword);
  const [visiblePasswordFields, setVisiblePasswordFields] = useState({
    newPassword: false,
    confirmPassword: false,
  });
  const [displayName, setDisplayNameState] = useState(() => setupDraft.displayName ?? "");
  const [encryptionKey, setEncryptionKeyState] = useState(() => setupDraft.encryptionKey);
  const [workspaceName, setWorkspaceNameState] = useState(() => setupDraft.workspaceName ?? "");
  const [workspaceDescription, setWorkspaceDescriptionState] = useState(() => setupDraft.workspaceDescription ?? "");
  const [modelAccess, setModelAccessState] = useState<"Manual" | "PersonAi">(() => setupDraft.modelAccess);
  const [rerankConfigured, setRerankConfiguredState] = useState(() => setupDraft.rerankConfigured ?? false);
  const [personAiAuthorization, setPersonAiAuthorizationState] = useState<PersonAiSetupAuthorization | undefined>(
    () => setupDraft.personAiAuthorization,
  );
  const [personAiPolling, setPersonAiPolling] = useState(false);
  const [personAiPollError, setPersonAiPollError] = useState("");
  const [models, setModels] = useState<Record<SetupModelType, ModelFormState>>(() => ({
    TEXT: { ...emptyModel("TEXT", t("models.defaultText")), ...setupDraft.models.TEXT },
    EMBEDDING: { ...emptyModel("EMBEDDING", t("models.defaultEmbedding")), ...setupDraft.models.EMBEDDING },
    RERANK: { ...emptyModel("RERANK", t("models.defaultRerank")), ...setupDraft.models.RERANK },
  }));
  const [verified, setVerified] = useState<Record<SetupModelType, boolean>>(() => ({ ...setupDraft.verified }));
  const [modelValidationAttempted, setModelValidationAttempted] = useState<Record<SetupModelType, boolean>>({ TEXT: false, EMBEDDING: false, RERANK: false });

  function setStep(nextStep: SetupStep) {
    setupDraft.step = nextStep;
    setStepState(nextStep);
  }

  function setNewPassword(value: string) {
    setupDraft.newPassword = value;
    setNewPasswordState(value);
  }

  function setConfirmPassword(value: string) {
    setupDraft.confirmPassword = value;
    setConfirmPasswordState(value);
  }

  function setDisplayName(value: string) {
    setupDraft.displayName = value;
    setDisplayNameState(value);
  }

  function setEncryptionKey(value: string) {
    setupDraft.encryptionKey = value;
    setEncryptionKeyState(value);
  }

  function setWorkspaceName(value: string) {
    setupDraft.workspaceName = value;
    setWorkspaceNameState(value);
  }

  function setWorkspaceDescription(value: string) {
    setupDraft.workspaceDescription = value;
    setWorkspaceDescriptionState(value);
  }

  function setModelAccess(value: "Manual" | "PersonAi") {
    setupDraft.modelAccess = value;
    setModelAccessState(value);
  }

  function setRerankConfigured(value: boolean) {
    setupDraft.rerankConfigured = value;
    setRerankConfiguredState(value);
    if (!value) {
      setupDraft.verified.RERANK = false;
      setVerified((current) => ({ ...current, RERANK: false }));
      setModelValidationAttempted((current) => ({ ...current, RERANK: false }));
    }
  }

  function setPersonAiAuthorization(value: PersonAiSetupAuthorization | undefined) {
    setupDraft.personAiAuthorization = value;
    setPersonAiAuthorizationState(value);
  }

  const steps = useMemo(() => STEP_ORDER.map((item) => ({
    ...item,
    label: t(item.labelKey),
  })), [t]);

  useEffect(() => {
    fetchSetupStatus()
      .then((next) => {
        setStatus(next);
        if (setupDraft.displayName === undefined) {
          setDisplayName(next.admin.displayName === "Bootstrap Admin" ? "Admin" : next.admin.displayName);
        }
        if (setupDraft.step === undefined) setStep("admin");
      })
      .catch((error: unknown) => toast.error(error instanceof Error ? error.message : t("errors.load")))
      .finally(() => setLoading(false));
  }, [navigate, toast]);

  useEffect(() => {
    if (!status) return;
    if (!status.required) {
      navigate("/login", { replace: true });
      return;
    }
    if (!status.admin.passwordResetRequired && authStatus === "unauthenticated") {
      navigate("/login?redirect=%2Fsetup", { replace: true });
    }
  }, [authStatus, navigate, status]);

  const stepIndex = steps.findIndex((item) => item.id === step);
  const currentStep = steps[stepIndex] ?? steps[0];
  const passwordPolicyValid = INITIAL_PASSWORD_PATTERN.test(newPassword);
  const passwordMismatch = newPassword.length > 0
    && confirmPassword.length > 0
    && newPassword !== confirmPassword;
  const canAdvance = useMemo(() => {
    if (step === "admin") {
      const adminAlreadyActivated = Boolean(status && !status.admin.passwordResetRequired);
      const passwordDraftIsEmpty = newPassword.length === 0 && confirmPassword.length === 0;
      return (adminAlreadyActivated && passwordDraftIsEmpty)
        || (passwordPolicyValid && newPassword === confirmPassword);
    }
    if (step === "crypto") return encryptionKey.length > 0;
    if (step === "workspace") return Boolean(workspaceName.trim());
    if (step === "models") {
      return modelAccess === "PersonAi"
        ? Boolean(personAiAuthorization?.verified)
        : (["TEXT", "EMBEDDING"] as const).every((type) => verified[type]);
    }
    return true;
  }, [confirmPassword, encryptionKey, modelAccess, newPassword, passwordPolicyValid, personAiAuthorization?.verified, status, step, verified, workspaceName]);

  function updateModel(type: SetupModelType, patch: Partial<ModelFormState>) {
    const nextModel = { ...models[type], ...patch };
    setupDraft.models[type] = nextModel;
    setModels((current) => ({ ...current, [type]: { ...current[type], ...patch } }));
    setupDraft.verified[type] = false;
    setVerified((current) => ({ ...current, [type]: false }));
  }

  function modelFieldErrors(draft: ModelFormState): ModelFormFieldErrors {
    const errors: ModelFormFieldErrors = {};
    if (!draft.displayName.trim()) errors.displayName = t("errors.modelFieldRequired");
    if (!draft.modelName.trim()) errors.modelName = t("errors.modelFieldRequired");
    if (!draft.vendor.trim()) errors.vendor = t("errors.modelFieldRequired");
    if (!draft.baseUrl.trim()) {
      errors.baseUrl = t("errors.modelFieldRequired");
    } else {
      try {
        new URL(draft.baseUrl.trim());
      } catch {
        errors.baseUrl = t("errors.modelBaseUrlInvalid");
      }
    }
    if (!draft.apiPath.trim()) errors.apiPath = t("errors.modelFieldRequired");
    if (!draft.apiKeyRef.trim()) errors.apiKeyRef = t("errors.modelFieldRequired");
    if (draft.modelType === "TEXT" && !draft.contextWindowTokens.trim()) {
      errors.contextWindowTokens = t("errors.modelFieldRequired");
    }
    return errors;
  }

  function togglePasswordField(field: keyof typeof visiblePasswordFields) {
    setVisiblePasswordFields((current) => ({ ...current, [field]: !current[field] }));
  }

  function passwordInput(
    field: keyof typeof visiblePasswordFields,
    value: string,
    onChange: (value: string) => void,
    label: string,
    invalid = false,
    pattern?: string,
  ) {
    const visible = visiblePasswordFields[field];
    const toggleLabel = t(`admin.${visible ? "hidePassword" : "showPassword"}`);

    return <div className="relative w-full min-w-0">
      <TextInput
        type={visible ? "text" : "password"}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        autoComplete="new-password"
        aria-invalid={invalid || undefined}
        pattern={pattern}
        className="pr-10"
      />
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        className="absolute right-1 top-1/2 -translate-y-1/2 text-[var(--color-on-surface-variant)]"
        aria-label={`${toggleLabel} ${label}`}
        aria-pressed={visible}
        title={toggleLabel}
        onClick={() => togglePasswordField(field)}
      >
        {visible ? <EyeOff size={16} aria-hidden="true" /> : <Eye size={16} aria-hidden="true" />}
      </Button>
    </div>;
  }

  async function activateAdmin() {
    // Once the bootstrap admin has already been activated, the password is
    // intentionally unavailable after a reload. Continue without asking the
    // user to re-enter or expose it again.
    const adminAlreadyActivated = Boolean(status && !status.admin.passwordResetRequired);
    const passwordDraftIsEmpty = newPassword.length === 0 && confirmPassword.length === 0;
    if (adminAlreadyActivated && passwordDraftIsEmpty) {
      setStep("crypto");
      return;
    }
    if (!passwordPolicyValid || newPassword !== confirmPassword) {
      return;
    }
    if (adminAlreadyActivated) {
      setStep("crypto");
      return;
    }
    setSaving(true);
    try {
      const grant = await createSetupSession(newPassword);
      acceptAuthGrant(grant);
      setStep("crypto");
    } catch (error) {
      toast.error(resolveSetupError(error, t, "errors.adminActivation"));
    } finally {
      setSaving(false);
    }
  }

  function continueFromCrypto() {
    const key = encryptionKey || makeKey();
    if (!/^[A-Za-z0-9+/]{22}==?$/.test(key) || atob(key).length !== 16) {
      toast.error(t("errors.cryptoFormat"));
      return;
    }
    setEncryptionKey(key);
    setStep("workspace");
  }

  function continueFromWorkspace() {
    if (!workspaceName.trim()) return toast.error(t("errors.workspaceName"));
    setStep("models");
  }

  async function verifyModel(type: SetupModelType) {
    const draft = models[type];
    setModelValidationAttempted((current) => ({ ...current, [type]: true }));
    if (Object.keys(modelFieldErrors(draft)).length > 0) return;
    setSaving(true);
    try {
      const payload = modelPayload(draft);
      await verifyPlatformModelDraft(payload);
      setupDraft.verified[type] = true;
      setVerified((current) => ({ ...current, [type]: true }));
      toast.success(t("success.model", { type }));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("errors.modelVerification", { type }));
    } finally {
      setSaving(false);
    }
  }

  async function startPersonAiAuthorization() {
    const key = encryptionKey || makeKey();
    if (!/^[A-Za-z0-9+/]{22}==?$/.test(key) || atob(key).length !== 16) {
      toast.error(t("errors.cryptoFormat"));
      return;
    }
    setEncryptionKey(key);
    setSaving(true);
    setPersonAiPollError("");
    try {
      const authorization = await createModelProviderAuthorization("PersonAi");
      setPersonAiAuthorization({
        sessionId: authorization.sessionId,
        loginUrl: authorization.loginUrl,
        status: authorization.status,
        verified: false,
      });
      window.open(authorization.loginUrl, "_blank", "noopener,noreferrer");
      toast.info(t("models.personAi.authorizationOpened", "已打开 Person-AI 授权页；完成登录后会自动检查授权状态。"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("models.personAi.authorizationFailed", "无法创建 Person-AI 授权。"));
    } finally {
      setSaving(false);
    }
  }

  const checkPersonAiAuthorization = useCallback(async (showPending = true) => {
    const authorization = personAiAuthorization;
    if (!authorization) return false;
    setPersonAiPolling(true);
    try {
      const result = await pollModelProviderAuthorization("PersonAi", authorization.sessionId);
      const verified = result.completed && result.textVerified === true && result.embeddingVerified === true;
      if (verified && result.connection?.personAiUserId && result.connection.personAiUsername) {
        await registerProfile(result.connection.personAiUserId, result.connection.personAiUsername);
      }
      setPersonAiAuthorization({
        sessionId: result.session.sessionId,
        loginUrl: result.session.loginUrl,
        status: result.session.status,
        verified,
      });
      setPersonAiPollError("");
      if (verified) {
        toast.success(t("models.personAi.verified", "Person-AI 聊天和 Embedding 已验证。"));
      } else if (showPending) {
        toast.info(t("models.personAi.pending", "授权还未完成，请在 Person-AI 页面完成登录。"));
      }
      return verified;
    } catch (error) {
      const message = error instanceof Error
        ? error.message
        : t("models.personAi.authorizationFailed", "Person-AI 授权检查失败。");
      setPersonAiPollError(message);
      if (showPending) toast.error(message);
      return false;
    } finally {
      setPersonAiPolling(false);
    }
  }, [encryptionKey, personAiAuthorization, t, toast]);

  useEffect(() => {
    if (!personAiAuthorization || personAiAuthorization.verified || !encryptionKey.trim()) return;
    let active = true;
    const poll = () => {
      void checkPersonAiAuthorization(false).then((verified) => {
        if (!active || !verified) return;
      });
    };
    poll();
    const timer = window.setInterval(poll, 3_000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [checkPersonAiAuthorization, personAiAuthorization?.verified]);

  async function finish() {
    setSaving(true);
    try {
      const key = encryptionKey || makeKey();
      if (!/^[A-Za-z0-9+/]{22}==?$/.test(key) || atob(key).length !== 16) {
        throw new Error(t("errors.cryptoFormat"));
      }
      if (!workspaceName.trim()) throw new Error(t("errors.workspaceName"));
      if (modelAccess === "Manual" && !(["TEXT", "EMBEDDING"] as const).every((type) => verified[type])) {
        throw new Error(t("errors.modelVerificationRequired"));
      }
      if (modelAccess === "PersonAi" && !personAiAuthorization?.verified) {
        throw new Error(t("errors.personAiNotReady", "请先完成 Person-AI 授权与验证。"));
      }

      const payload: Parameters<typeof completeSetup>[0] = {
        newPassword,
        displayName: displayName.trim(),
        encryptionKeyBase64: key,
        workspaceName: workspaceName.trim(),
        workspaceDescription: workspaceDescription.trim() || null,
        modelAccess,
      };
      if (modelAccess === "Manual") {
        payload.models = {
          text: modelPayload(models.TEXT),
          embedding: modelPayload(models.EMBEDDING),
          ...(rerankConfigured ? { rerank: modelPayload(models.RERANK) } : {}),
        };
      }
      const completed = await completeSetup(payload);
      if (!completed.initialized) {
        throw new Error(t("errors.setupPersistence"));
      }
      clearSetupDraft();
      await useAuthStore.getState().logout();
      navigate("/login", { replace: true });
    } catch (error) {
      toast.error(resolveSetupError(error, (key) => t(key), "errors.setupConfirmation"));
    } finally {
      setSaving(false);
    }
  }

  if (loading || !status) return <main className="flex min-h-screen items-center justify-center bg-[var(--color-background)] text-sm text-[var(--color-on-surface-variant)]">{t("loading")}</main>;

  return (
    <main className="relative min-h-screen bg-[var(--color-background)] px-5 py-6 text-[var(--color-on-surface)] sm:px-8 lg:px-12">
      <div className="absolute right-5 top-10 z-10 flex items-center gap-1 rounded-full border border-[var(--color-outline-variant)] bg-[var(--color-background)] p-1 text-xs sm:right-8 lg:right-12" role="group" aria-label={t("language.label")}>
        {(["zh-CN", "en-US"] as SupportedLocale[]).map((locale) => (
          <button
            key={locale}
            type="button"
            onClick={() => void changeLocale(locale)}
            aria-pressed={currentLocale === locale}
            className={`rounded-full px-2.5 py-1 transition-colors ${currentLocale === locale ? "bg-[var(--color-on-surface)] text-[var(--color-background)]" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-surface-container)]"}`}
          >
            {locale === "zh-CN" ? t("language.chinese") : t("language.english")}
          </button>
        ))}
      </div>
      <div className="mx-auto grid min-h-[calc(100vh-3rem)] max-w-6xl grid-cols-1 gap-10 lg:grid-cols-[240px_minmax(0,1fr)] lg:gap-16">
        <aside className="flex flex-col justify-between py-4">
          <div>
            <div className="w-full text-left text-2xl font-semibold tracking-[0.08em] text-[var(--color-on-surface)]">{t("title")}</div>
            <nav className="mt-12 space-y-1" aria-label={t("aria.steps")}>
              {steps.map((item, index) => {
                const done = index < stepIndex;
                const active = item.id === step;
                return <div key={item.id} aria-current={active ? "step" : undefined} className={`flex w-full items-center gap-3 rounded-xl px-3 py-3 text-left ${active ? "bg-[var(--color-surface-container)]" : ""}`}>
                  <span className={`flex size-7 items-center justify-center rounded-full text-xs ${done ? "bg-[var(--color-primary)] text-[var(--color-on-primary)]" : active ? "border border-[var(--color-primary)] text-[var(--color-primary)]" : "border border-[var(--color-outline-variant)] text-[var(--color-on-surface-variant)]"}`}>{done ? <Check size={14} /> : index + 1}</span>
                  <span className="text-sm font-medium">{item.label}</span>
                </div>;
              })}
            </nav>
          </div>
        </aside>

        <section className="flex max-w-3xl flex-col justify-start py-4 lg:pb-4 lg:pt-[88px]">
          <div className="mb-10 flex items-start justify-between gap-5 border-b border-[var(--color-outline-variant)] pb-6">
            <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">{currentStep.label}</h1>
          </div>

          <div className="space-y-7">
            {step === "admin" && (
              <div className="space-y-5">
                <div className="grid gap-4 sm:grid-cols-2">
                  <Field label={t("admin.account")}><TextInput value={status.admin.username} disabled /></Field>
                  <Field required label={t("admin.displayName")}><TextInput value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></Field>
                </div>
                <div className="grid gap-4 sm:grid-cols-2">
                  <Field
                    required
                    error={newPassword.length > 0 && !passwordPolicyValid ? t("admin.passwordPolicyError") : undefined}
                    label={t("admin.newPassword")}
                  >
                    {passwordInput(
                      "newPassword",
                      newPassword,
                      setNewPassword,
                      t("admin.newPassword"),
                      newPassword.length > 0 && !passwordPolicyValid,
                      INITIAL_PASSWORD_PATTERN.source,
                    )}
                  </Field>
                  <Field required error={passwordMismatch ? t("admin.passwordMismatch") : undefined} label={t("admin.confirmPassword")}>
                    {passwordInput("confirmPassword", confirmPassword, setConfirmPassword, t("admin.confirmPassword"), passwordMismatch)}
                  </Field>
                </div>
                <div className="border-l-2 border-[var(--color-primary)] pl-4 text-sm leading-6 text-[var(--color-on-surface-variant)]">
                  {t("admin.passwordStrengthHint")}
                </div>
              </div>
            )}
            {step === "crypto" && (
              <div className="space-y-5">
                <Field label={t("crypto.key")} hint={t("crypto.keyHint")} hintAriaLabel={t("aria.openFieldDescription")}>
                  <TextInput type="password" value={encryptionKey} onChange={(event) => setEncryptionKey(event.target.value)} placeholder={t("crypto.generatedPlaceholder")} autoComplete="off" />
                </Field>
                <div className="border-l-2 border-[var(--color-primary)] pl-4 text-sm leading-6 text-[var(--color-on-surface-variant)]">
                  {t("crypto.usageHint")}
                </div>
              </div>
            )}
            {step === "workspace" && (
              <div className="space-y-5">
                <Field required label={t("workspace.name")}><TextInput value={workspaceName} onChange={(event) => setWorkspaceName(event.target.value)} placeholder={t("workspace.namePlaceholder")} /></Field>
                <Field label={t("workspace.description")}><TextArea value={workspaceDescription} onChange={(event) => setWorkspaceDescription(event.target.value)} placeholder={t("workspace.descriptionPlaceholder")} rows={4} /></Field>
              </div>
            )}
            {step === "models" && (
              <div className="space-y-8">
                <div className="grid gap-3 sm:grid-cols-2" role="radiogroup" aria-label={t("models.access.label", "模型接入方式")}>
                  {([
                    ["PersonAi", t("models.access.personAi", "接入 Person-AI"), t("models.access.personAiHint", "浏览器授权后自动同步聊天模型和 Embedding。")],
                    ["Manual", t("models.access.manual", "手动配置模型"), t("models.access.manualHint", "填写并验证文本和向量模型；重排模型可选。")],
                  ] as const).map(([access, label, hint]) => (
                    <button
                      aria-checked={modelAccess === access}
                      className={`rounded-xl border p-4 text-left transition-colors ${modelAccess === access ? "border-[var(--color-primary)] bg-[var(--color-secondary-container)]" : "border-[var(--color-outline-variant)] hover:bg-[var(--color-surface-container)]"}`}
                      key={access}
                      onClick={() => setModelAccess(access)}
                      role="radio"
                      type="button"
                    >
                      <p className="font-medium">{label}</p>
                      <p className="mt-1 text-sm leading-5 text-[var(--color-on-surface-variant)]">{hint}</p>
                    </button>
                  ))}
                </div>
                {modelAccess === "PersonAi" ? (
                  <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-5">
                    <h2 className="font-medium">{t("models.personAi.title", "Person-AI 平台连接")}</h2>
                    <p className="mt-2 text-sm leading-6 text-[var(--color-on-surface-variant)]">
                      {t("models.personAi.description", "系统会使用平台管理员授权的账号，同步远程聊天模型，并验证高质量自动聊天与 Embedding。")}
                    </p>
                    {personAiAuthorization ? (
                      <div className="mt-4 rounded-lg bg-[var(--color-surface-container)] px-3 py-2 text-sm">
                        <p>{personAiAuthorization.verified
                          ? t("models.personAi.ready", "授权完成，模型目录已可用。")
                          : t("models.personAi.waiting", "等待 Person-AI 授权完成…")}</p>
                        {!personAiAuthorization.verified && personAiPollError ? <p className="mt-1 text-[var(--color-error)]">{personAiPollError}</p> : null}
                      </div>
                    ) : null}
                    <div className="mt-5 flex flex-wrap gap-3">
                      <Button type="button" disabled={saving || personAiPolling} onClick={() => void startPersonAiAuthorization()}>
                        {personAiAuthorization ? t("models.personAi.reauthorize", "重新打开授权") : t("models.personAi.authorize", "连接 Person-AI")}
                      </Button>
                      {personAiAuthorization && !personAiAuthorization.verified ? (
                        <Button type="button" variant="secondary" disabled={saving || personAiPolling} onClick={() => void checkPersonAiAuthorization()}>
                          {personAiPolling ? t("models.personAi.checking", "检查中…") : t("models.personAi.check", "检查授权状态")}
                        </Button>
                      ) : null}
                    </div>
                  </div>
                ) : (
                  <>
                    {(["TEXT", "EMBEDDING"] as const).map((type) => (
                      <div key={type} className="border-b border-[var(--color-outline-variant)] pb-7">
                        <div className="mb-4 flex items-center justify-between">
                          <h2 className="font-medium">{type === "TEXT" ? t("models.text") : t("models.embedding")}</h2>
                          {verified[type] && <span className="flex items-center gap-1 text-xs text-[var(--color-primary)]"><Check size={14} /> {t("models.verified")}</span>}
                        </div>
                        <ModelFormFields
                          state={models[type]}
                          fieldErrors={modelValidationAttempted[type] ? modelFieldErrors(models[type]) : undefined}
                          onChange={(nextState) => updateModel(type, nextState)}
                          allowedProtocols={type === "TEXT" ? ["OPENAI_COMPATIBLE", "RESPONSES"] : undefined}
                          allowedCapabilityGroups={type === "TEXT" ? ["Multimodal", "Text"] : undefined}
                          fixedProtocol={type === "EMBEDDING" ? "OPENAI_COMPATIBLE_EMBEDDING" : undefined}
                          hideDescription
                          apiKeyValue={models[type].apiKeyRef}
                          onApiKeyChange={(value) => updateModel(type, { apiKeyRef: value })}
                        />
                        <div className="mt-5 flex justify-end">
                          <Button type="button" variant="default" disabled={saving} onClick={() => void verifyModel(type)}>{t("models.verify")}</Button>
                        </div>
                      </div>
                    ))}
                    <div className="pb-7">
                      <div className="mb-4 flex items-center justify-between gap-4">
                        <h2 className="font-medium">{t("models.rerank")}</h2>
                        <Button type="button" variant="secondary" disabled={saving} onClick={() => setRerankConfigured(!rerankConfigured)}>
                          {rerankConfigured ? t("models.rerankRemove") : t("models.rerankConfigure")}
                        </Button>
                      </div>
                      {rerankConfigured ? (
                        <>
                          <ModelFormFields
                            state={models.RERANK}
                            fieldErrors={modelValidationAttempted.RERANK ? modelFieldErrors(models.RERANK) : undefined}
                            onChange={(nextState) => updateModel("RERANK", nextState)}
                            fixedProtocol="RERANK"
                            hideDescription
                            apiKeyValue={models.RERANK.apiKeyRef}
                            onApiKeyChange={(value) => updateModel("RERANK", { apiKeyRef: value })}
                          />
                          <div className="mt-5 flex justify-end">
                            <Button type="button" variant="default" disabled={saving} onClick={() => void verifyModel("RERANK")}>{t("models.verify")}</Button>
                          </div>
                        </>
                      ) : null}
                    </div>
                  </>
                )}
              </div>
            )}
            {step === "review" && (
              <div className="grid gap-3 sm:grid-cols-2">
                {[[t("review.admin"), displayName], [t("review.workspace"), workspaceName], [t("review.encryption"), status.crypto.algorithm], [t("review.models"), modelAccess === "PersonAi" ? t("models.personAi.ready", "Person-AI 已授权") : t("review.modelSummary")]].map(([label, value]) => (
                  <div key={label} className="flex items-center justify-between gap-5 rounded-lg bg-[var(--color-surface-container)] px-4 py-3 text-sm"><span className="text-[var(--color-on-surface-variant)]">{label}</span><span className="text-right font-medium">{value}</span></div>
                ))}
              </div>
            )}
          </div>

          <div className="mt-10 flex items-center justify-between gap-4">
            <Button type="button" variant="ghost" disabled={stepIndex === 0 || saving} onClick={() => setStep(steps[stepIndex - 1].id)}>{t("actions.previous")}</Button>
            <div className="flex items-center gap-3">
              {step === "admin" ? <Button type="button" disabled={!canAdvance || saving} onClick={() => void activateAdmin()}>{saving ? t("actions.processing") : t("actions.next")}<ChevronRight size={16} /></Button>
                : step === "crypto" ? <Button type="button" disabled={saving} onClick={continueFromCrypto}>{t("actions.next")}<ChevronRight size={16} /></Button>
                  : step === "workspace" ? <Button type="button" disabled={saving} onClick={continueFromWorkspace}>{t("actions.next")}<ChevronRight size={16} /></Button>
                    : step === "models" ? <Button type="button" disabled={!canAdvance || saving} onClick={() => setStep("review")}>{t("actions.next")}<ChevronRight size={16} /></Button>
                      : <Button type="button" disabled={saving} onClick={() => void finish()}>{t("actions.confirmInitialization")}</Button>}
            </div>
          </div>
        </section>
      </div>
    </main>
  );
}

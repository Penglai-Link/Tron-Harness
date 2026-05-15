import {
  startTransition,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  ArrowRight,
  LockKeyhole,
  Orbit,
  ShieldCheck,
} from "lucide-react";

import { useTranslation } from "react-i18next";
import {
  useLocation,
  useNavigate,
} from "react-router";

import {
  buildOAuth2AuthorizeUrl,
  fetchAuthProviders,
} from "@/domain/auth/api";
import { DEFAULT_AUTH_REDIRECT } from "@/domain/auth/constants";
import { resolveAuthRedirect } from "@/domain/auth/model";
import {
  Field,
  TextInput,
  useToast,
} from "@/shared";
import { Button } from "@/shared/ui/Button";
import { isApiError } from "@/shared/api/httpClient";
import { useAuthStore } from "@/store/authStore";

export function LoginPage() {
  const { t } = useTranslation(["auth", "common"]);
  const location = useLocation();
  const navigate = useNavigate();
  const toast = useToast();
  const loginWithInternalAccount = useAuthStore((state) => state.loginWithInternalAccount);
  const redirectTarget = useMemo(
    () => resolveAuthRedirect(location.search, DEFAULT_AUTH_REDIRECT),
    [location.search],
  );
  const [login, setLogin] = useState("auth.operator");
  const [password, setPassword] = useState("TronAuthPass123!");
  const [submitting, setSubmitting] = useState(false);
  const [providersLoading, setProvidersLoading] = useState(true);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [oauthProviders, setOauthProviders] = useState<Array<{ code: string; name: string }>>([]);

  useEffect(() => {
    const controller = new AbortController();
    setProvidersLoading(true);
    setProviderError(null);

    fetchAuthProviders(controller.signal)
      .then((payload) => {
        setOauthProviders(
          payload.providers
            .filter((provider) => provider.type === "oauth2")
            .map((provider) => ({ code: provider.code, name: provider.name })),
        );
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        const message = error instanceof Error ? error.message : t("auth:providerUnavailable");
        setProviderError(message);
      })
      .finally(() => {
        if (!controller.signal.aborted) setProvidersLoading(false);
      });

    return () => controller.abort();
  }, [t]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setFormError(null);

    try {
      const session = await loginWithInternalAccount({
        login: login.trim(),
        password,
      });

      if (session.user.passwordResetRequired) {
        toast.warning(t("auth:passwordResetRequired"));
      }

      startTransition(() => {
        navigate(redirectTarget, { replace: true });
      });
    } catch (error) {
      const message = isApiError(error)
        ? error.message
        : error instanceof Error
          ? error.message
          : t("auth:signInFailed");
      setFormError(message);
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  function handleOAuth2SignIn(providerCode: string) {
    window.location.assign(buildOAuth2AuthorizeUrl(providerCode, redirectTarget));
  }

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(188,220,255,0.28),_transparent_42%),linear-gradient(180deg,_#f7fafc_0%,_#eef3f8_100%)] px-6 py-10 text-[var(--color-on-surface)]">
      <section className="mx-auto grid min-h-[calc(100vh-5rem)] max-w-6xl gap-8 lg:grid-cols-[minmax(0,1.1fr)_420px] lg:items-center">
        <div className="rounded-[28px] border border-white/70 bg-[rgba(255,255,255,0.78)] p-8 shadow-[0_28px_80px_rgba(42,71,110,0.12)] backdrop-blur lg:p-10">
          <div className="inline-flex items-center gap-2 rounded-full border border-[var(--color-outline-variant)] bg-white px-3 py-1 text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
            <ShieldCheck size={15} aria-hidden="true" />
            {t("auth:eyebrow")}
          </div>
          <h1 className="mt-6 max-w-xl text-4xl font-semibold leading-tight text-slate-950 lg:text-5xl">
            {t("auth:title")}
          </h1>
          <p className="mt-4 max-w-2xl text-base leading-7 text-slate-600">
            {t("auth:subtitle")}
          </p>
          <div className="mt-8 grid gap-4 sm:grid-cols-3">
            <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-white/80 p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{t("auth:highlights.internal.label")}</p>
              <p className="mt-2 text-sm leading-6 text-slate-700">{t("auth:highlights.internal.body")}</p>
            </div>
            <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-white/80 p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{t("auth:highlights.oauth.label")}</p>
              <p className="mt-2 text-sm leading-6 text-slate-700">{t("auth:highlights.oauth.body")}</p>
            </div>
            <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-white/80 p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{t("auth:highlights.redirect.label")}</p>
              <p className="mt-2 text-sm leading-6 text-slate-700">{t("auth:highlights.redirect.body")}</p>
            </div>
          </div>
        </div>

        <section className="rounded-[28px] border border-white/80 bg-white p-8 shadow-[0_28px_80px_rgba(42,71,110,0.14)]">
          <div className="mb-8 flex items-start justify-between gap-4">
            <div>
              <p className="text-sm font-medium text-[var(--color-on-surface-variant)]">{t("auth:cardTitle")}</p>
              <h2 className="mt-2 text-2xl font-semibold text-slate-950">{t("auth:cardSubtitle")}</h2>
            </div>
            <div className="flex size-12 items-center justify-center rounded-2xl bg-slate-950 text-white">
              <LockKeyhole size={20} aria-hidden="true" />
            </div>
          </div>

          <form className="space-y-5" onSubmit={handleSubmit}>
            <Field label={t("auth:fields.usernameOrEmail")} error={formError ?? undefined}>
              <TextInput
                autoComplete="username"
                placeholder={t("auth:usernamePlaceholder")}
                value={login}
                onChange={(event) => setLogin(event.target.value)}
              />
            </Field>
            <Field label={t("auth:fields.password")}>
              <TextInput
                autoComplete="current-password"
                placeholder={t("auth:passkeyPlaceholder")}
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
            </Field>
            <Button className="w-full" disabled={submitting} type="submit">
              {submitting ? t("auth:signingIn") : t("common:actions.signIn")}
              <ArrowRight size={16} aria-hidden="true" />
            </Button>
          </form>

          <div className="my-6 flex items-center gap-3">
            <div className="h-px flex-1 bg-[var(--color-outline-variant)]" />
            <span className="text-[12px] font-medium uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">{t("auth:alternativeDivider")}</span>
            <div className="h-px flex-1 bg-[var(--color-outline-variant)]" />
          </div>

          <div className="space-y-3">
            {providersLoading ? (
              <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-sm text-[var(--color-on-surface-variant)]">
                {t("auth:loadingProviders")}
              </div>
            ) : null}
            {!providersLoading && providerError ? (
              <div className="rounded-2xl border border-[var(--color-error-container)] bg-[var(--color-error-container)] px-4 py-3 text-sm text-[var(--color-on-error-container)]">
                {providerError}
              </div>
            ) : null}
            {!providersLoading && !providerError && oauthProviders.length === 0 ? (
              <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-sm text-[var(--color-on-surface-variant)]">
                {t("auth:noExternalProviders")}
              </div>
            ) : null}
            {oauthProviders.map((provider) => (
              <Button
                className="w-full justify-between"
                key={provider.code}
                type="button"
                variant="secondary"
                onClick={() => handleOAuth2SignIn(provider.code)}
              >
                <span className="flex items-center gap-2">
                  <Orbit size={16} aria-hidden="true" />
                  {provider.name}
                </span>
                <ArrowRight size={16} aria-hidden="true" />
              </Button>
            ))}
          </div>

          <p className="mt-6 text-sm leading-6 text-[var(--color-on-surface-variant)]">
            {t("auth:redirectNotice", { redirect: redirectTarget })}
          </p>
        </section>
      </section>
    </main>
  );
}

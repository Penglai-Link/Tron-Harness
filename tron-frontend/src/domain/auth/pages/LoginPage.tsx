import {
  startTransition,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  ArrowRight,
  LoaderCircle,
  LockKeyhole,
  Orbit,
} from "lucide-react";

import { useTranslation } from "react-i18next";
import {
  useLocation,
  useNavigate,
} from "react-router";

import {
  buildOAuth2AuthorizeUrl,
  fetchAuthProviders,
  getAuthDataSource,
} from "@/domain/auth/api";
import { DEFAULT_AUTH_REDIRECT } from "@/domain/auth/constants";
import { resolveAuthRedirect } from "@/domain/auth/model";
import {
  Field,
  PixelBlast,
  TextInput,
  useTheme,
  useToast,
} from "@/shared";
import { Button } from "@/components/ui/button";
import { isApiError } from "@/shared/api/httpClient";
import { useAuthStore } from "@/store/authStore";

export function LoginPage() {
  const { t } = useTranslation(["auth", "common"]);
  const location = useLocation();
  const navigate = useNavigate();
  const { theme } = useTheme();
  const toast = useToast();
  const loginWithInternalAccount = useAuthStore((state) => state.loginWithInternalAccount);
  const loginWithLdapAccount = useAuthStore((state) => state.loginWithLdapAccount);
  const redirectTarget = useMemo(
    () => resolveAuthRedirect(location.search, DEFAULT_AUTH_REDIRECT),
    [location.search],
  );
  const [login, setLogin] = useState(() => getAuthDataSource() === "mock" ? "auth.operator" : "");
  const [password, setPassword] = useState(() => getAuthDataSource() === "mock" ? "TronAuthPass123!" : "");
  const [submitting, setSubmitting] = useState(false);
  const [providersLoading, setProvidersLoading] = useState(true);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [externalProviders, setExternalProviders] = useState<Array<{
    code: string;
    name: string;
    type: "ldap" | "oauth2";
  }>>([]);
  const [oauthSubmittingProviderCode, setOauthSubmittingProviderCode] = useState<string | null>(null);
  const authBusy = submitting || oauthSubmittingProviderCode !== null;

  useEffect(() => {
    const controller = new AbortController();
    setProvidersLoading(true);
    setProviderError(null);

    fetchAuthProviders(controller.signal)
      .then((payload) => {
        setExternalProviders(
          payload.providers
            .filter((provider): provider is typeof provider & { type: "ldap" | "oauth2" } => (
              provider.type === "ldap" || provider.type === "oauth2"
            ))
            .map((provider) => ({
              code: provider.code,
              name: provider.code === "dev"
                ? t("auth:providerLabels.devOAuth2")
                : provider.code === "ldap"
                  ? t("auth:providerLabels.ldap")
                  : provider.name,
              type: provider.type,
            })),
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
    if (authBusy) return;

    setSubmitting(true);
    setOauthSubmittingProviderCode(null);
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

  async function handleExternalSignIn(provider: {
    code: string;
    name: string;
    type: "ldap" | "oauth2";
  }) {
    if (authBusy) {
      return;
    }

    setFormError(null);
    setOauthSubmittingProviderCode(provider.code);

    if (provider.type === "ldap") {
      try {
        const session = await loginWithLdapAccount({
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
        setOauthSubmittingProviderCode(null);
      }
      return;
    }

    try {
      window.location.assign(buildOAuth2AuthorizeUrl(provider.code, redirectTarget));
    } catch (error) {
      const message = error instanceof Error ? error.message : t("auth:providerUnavailable");
      setOauthSubmittingProviderCode(null);
      setProviderError(message);
      toast.error(message);
    }
  }

  const pixelBlastProps = theme === "matrix-dark"
    ? {
        color: "#5cff85",
        edgeFade: 0.22,
        enableRipples: true,
        liquid: true,
        liquidRadius: 1.15,
        liquidStrength: 0.11,
        liquidWobbleSpeed: 4.9,
        patternDensity: 1.08,
        patternScale: 2.35,
        pixelSize: 6,
        pixelSizeJitter: 0.5,
        rippleIntensityScale: 1.35,
        rippleSpeed: 0.42,
        rippleThickness: 0.12,
        speed: 0.62,
        transparent: true,
        variant: "circle" as const,
      }
    : {
        color: "#8ea5d6",
        edgeFade: 0.34,
        enableRipples: true,
        liquid: false,
        patternDensity: 0.94,
        patternScale: 1.85,
        pixelSize: 5,
        pixelSizeJitter: 0.32,
        rippleIntensityScale: 0.9,
        rippleSpeed: 0.34,
        rippleThickness: 0.11,
        speed: 0.4,
        transparent: true,
        variant: "diamond" as const,
      };

  return (
    <main className="auth-page-shell relative min-h-screen overflow-hidden px-6 py-10 text-[var(--color-on-surface)]">
      <div className="absolute inset-0">
        <PixelBlast
          {...pixelBlastProps}
          className="absolute inset-0"
        />
        <div
          className="absolute inset-0 pointer-events-none"
          style={{
            background: theme === "matrix-dark"
              ? "radial-gradient(circle at center, rgba(1, 6, 3, 0.04) 0%, rgba(1, 6, 3, 0.34) 52%, rgba(1, 6, 3, 0.7) 100%)"
              : "radial-gradient(circle at center, rgba(255, 255, 255, 0.08) 0%, rgba(247, 250, 252, 0.4) 52%, rgba(238, 243, 248, 0.76) 100%)",
          }}
        />
      </div>
      <section className="relative z-10 mx-auto flex min-h-[calc(100vh-5rem)] max-w-6xl items-center justify-center">
        <section className="auth-elevated-card w-full max-w-[420px] rounded-[28px] p-8">
          <div className="mb-8 flex items-start justify-between gap-4">
            <div>
              <h1 className="text-2xl font-semibold text-[var(--color-on-surface)]">{t("auth:cardSubtitle")}</h1>
            </div>
            <div className="flex size-12 items-center justify-center rounded-2xl bg-[var(--color-surface-container-highest)] text-[var(--color-primary)]">
              <LockKeyhole size={20} aria-hidden="true" />
            </div>
          </div>

          <form className="space-y-5" onSubmit={handleSubmit}>
            <Field label={t("auth:fields.usernameOrEmail")} error={formError ?? undefined}>
              <TextInput
                aria-label={t("auth:fields.usernameOrEmail")}
                autoComplete="username"
                disabled={authBusy}
                placeholder={t("auth:usernamePlaceholder")}
                value={login}
                onChange={(event) => setLogin(event.target.value)}
              />
            </Field>
            <Field label={t("auth:fields.password")}>
              <TextInput
                aria-label={t("auth:fields.password")}
                autoComplete="current-password"
                disabled={authBusy}
                placeholder={t("auth:passkeyPlaceholder")}
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
            </Field>
            <Button className="w-full" disabled={authBusy} aria-busy={submitting} type="submit">
              {submitting ? <LoaderCircle className="animate-spin" size={16} aria-hidden="true" /> : null}
              {submitting ? t("auth:signingIn") : t("common:actions.signIn")}
              {!submitting ? <ArrowRight size={16} aria-hidden="true" /> : null}
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
            {!providersLoading && !providerError && externalProviders.length === 0 ? (
              <div className="rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-3 text-sm text-[var(--color-on-surface-variant)]">
                {t("auth:noExternalProviders")}
              </div>
            ) : null}
            {externalProviders.map((provider) => {
              const providerSubmitting = oauthSubmittingProviderCode === provider.code;

              return (
                <Button
                  className="w-full justify-between"
                  disabled={authBusy}
                  aria-busy={providerSubmitting}
                  key={provider.code}
                  type="button"
                  variant="secondary"
                  onClick={() => void handleExternalSignIn(provider)}
                >
                  <span className="flex items-center gap-[12px]">
                    {providerSubmitting ? <LoaderCircle className="animate-spin" size={16} aria-hidden="true" /> : <Orbit size={16} aria-hidden="true" />}
                    {provider.name}
                  </span>
                  {providerSubmitting ? <LoaderCircle className="animate-spin" size={16} aria-hidden="true" /> : <ArrowRight size={16} aria-hidden="true" />}
                </Button>
              );
            })}
          </div>
        </section>
      </section>
    </main>
  );
}

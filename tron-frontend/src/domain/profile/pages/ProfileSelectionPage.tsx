import {
  EntryBackdrop,
  EntryLoadingIndicator,
  EntryWordmark,
} from "@/shared/entry/EntryScene";
import "./ProfileSelectionPage.css";
import { Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  activateProfile,
  createPendingProfile,
  createPersonAiAuthorization,
  fetchBootstrapProfileStatus,
  fetchProfiles,
  pollPersonAiAuthorization,
  removeProfile,
  type BootstrapProfileStatus,
  type PersonAiAuthorizationSession,
  type PersonAiProfile,
} from "@/domain/profile/api";
import { useLocale } from "@/i18n";
import { isApiError } from "@/shared/api/httpClient";
import { useManagedAvatar } from "@/shared/hooks/useManagedAvatar";
import { useAuthStore } from "@/store/authStore";
import { useProfileAgreementStore } from "@/store/profileAgreementStore";

type AgreementAction =
  | { type: "authorize" }
  | { type: "new-account" }
  | { type: "enter"; profile: PersonAiProfile };

function firstAvailableProfileId(
  profiles: PersonAiProfile[],
  activeProfileId: string | null
) {
  return (
    profiles.find(
      (profile) =>
        profile.profileId === activeProfileId && profile.status !== "PENDING"
    )?.profileId ??
    profiles.find((profile) => profile.status !== "PENDING")?.profileId ??
    null
  );
}

function profileInitials(username: string) {
  const value = username.trim();
  return value ? value.slice(0, 2).toUpperCase() : "PA";
}

function ProfileAvatar({ profile }: { profile: PersonAiProfile }) {
  const { src } = useManagedAvatar(profile.avatarUrl);
  const [imageFailed, setImageFailed] = useState(false);

  useEffect(() => setImageFailed(false), [profile.avatarUrl]);

  return (
    <span className="tron-login-avatar flex size-8 shrink-0 items-center justify-center overflow-hidden rounded-md text-xs font-medium">
      {src && !imageFailed ? (
        <img
          alt=""
          className="size-full object-cover"
          src={src}
          onError={() => setImageFailed(true)}
        />
      ) : (
        profileInitials(profile.personAiUsername)
      )}
    </span>
  );
}

export function ProfileSelectionPage() {
  const { t } = useTranslation("pages");
  const { changeLocale, currentLocale, supportedLocales } = useLocale();
  const nextLocale =
    supportedLocales.find((locale) => locale.code !== currentLocale)?.code ??
    currentLocale;
  const navigate = useNavigate();
  async function enterWorkspace() {
    if (!useProfileAgreementStore.getState().checked) return;
    navigate("/workspace", { replace: true });
  }
  const [searchParams] = useSearchParams();
  const forceReauthorization = searchParams.get("mode") === "reauth";
  const [profiles, setProfiles] = useState<PersonAiProfile[]>([]);
  const [bootstrap, setBootstrap] = useState<BootstrapProfileStatus | null>(
    null
  );
  const [loading, setLoading] = useState(true);
  const [activating, setActivating] = useState<string | null>(null);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(
    null
  );
  const [authorizing, setAuthorizing] = useState(false);
  const [completingAuthorization, setCompletingAuthorization] = useState(false);
  const [authorizationRetrying, setAuthorizationRetrying] = useState(false);
  const agreementChecked = useProfileAgreementStore((state) => state.checked);
  const setAgreementChecked = useProfileAgreementStore((state) => state.setChecked);
  const [pendingAgreementAction, setPendingAgreementAction] =
    useState<AgreementAction | null>(null);
  const [agreementOpen, setAgreementOpen] = useState(false);
  const [profileToRemove, setProfileToRemove] =
    useState<PersonAiProfile | null>(null);
  const [removingProfile, setRemovingProfile] = useState(false);
  const [authorization, setAuthorization] =
    useState<PersonAiAuthorizationSession | null>(null);
  const authorizationWindowRef = useRef<Window | null>(null);
  const authorizationRequestRef = useRef<AbortController | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setAgreementChecked(false);
    const controller = new AbortController();
    Promise.all([
      fetchProfiles(controller.signal),
      fetchBootstrapProfileStatus(controller.signal),
    ])
      .then(([nextProfiles, nextBootstrap]) => {
        setProfiles(nextProfiles);
        setBootstrap(nextBootstrap);
        setSelectedProfileId(
          firstAvailableProfileId(nextProfiles, nextBootstrap.activeProfileId)
        );
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) {
          setError(
            reason instanceof Error
              ? reason.message
              : t("personAiProfiles.errors.load")
          );
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => {
      controller.abort();
      authorizationRequestRef.current?.abort();
    };
  }, []);

  useEffect(() => {
    if (!authorization) return;
    const controller = new AbortController();
    authorizationRequestRef.current = controller;
    let pollTimer: number | undefined;
    let expiryTimer: number | undefined;
    let failures = 0;
    let completing = false;
    let polling = false;
    // Some upstream sessions omit expiresAt. Keep those sessions bounded too.
    const deadline = authorization.createdAt + 10 * 60_000;
    const fail = (message: string) => {
      controller.abort();
      window.clearTimeout(pollTimer);
      window.clearTimeout(expiryTimer);
      setAuthorization(null);
      setCompletingAuthorization(false);
      setAuthorizationRetrying(false);
      closeAuthorizationWindow();
      setError(message);
    };
    const scheduleExpiry = (expiresAt: number | null) => {
      window.clearTimeout(expiryTimer);
      const remaining = Math.min(expiresAt ?? deadline, deadline) - Date.now();
      if (remaining <= 0) {
        fail(t("personAiProfiles.errors.authorizationExpired"));
        return false;
      }
      expiryTimer = window.setTimeout(
        () => fail(t("personAiProfiles.errors.authorizationExpired")),
        remaining
      );
      return true;
    };
    const poll = async () => {
      if (controller.signal.aborted || polling || completing) return;
      polling = true;
      window.clearTimeout(pollTimer);
      try {
        const signal = AbortSignal.any([
          controller.signal,
          AbortSignal.timeout(45_000),
        ]);
        const result = await pollPersonAiAuthorization(authorization.sessionId, signal);
        if (controller.signal.aborted) return;
        failures = 0;
        setAuthorizationRetrying(false);
        if (!result.completed) {
          if (["expired", "cancelled", "canceled", "denied", "rejected", "failed"]
            .includes(result.session.status.toLowerCase())) {
            fail(t("personAiProfiles.errors.authorizationEnded"));
            return;
          }
          if (scheduleExpiry(result.session.expiresAt)) {
            pollTimer = window.setTimeout(() => void poll(), 1_500);
          }
          return;
        }
        completing = true;
        setCompletingAuthorization(true);
        closeAuthorizationWindow();
        const [nextProfiles, nextBootstrap] = await Promise.all([
          fetchProfiles(signal),
          fetchBootstrapProfileStatus(signal),
        ]);
        if (controller.signal.aborted) return;
        // Keep authorization pending until the session is usable. Clearing it
        // earlier lets auto-entry mistake the old auth status for expired access.
        const authenticated = await useAuthStore
          .getState()
          .ensureSessionFresh(signal);
        if (controller.signal.aborted) return;
        window.clearTimeout(expiryTimer);
        setProfiles(nextProfiles);
        setBootstrap(nextBootstrap);
        setAuthorization(null);
        setCompletingAuthorization(false);
        if (authenticated) {
          void enterWorkspace();
        } else {
          setError(t("personAiProfiles.errors.activate"));
        }
      } catch (reason: unknown) {
        if (controller.signal.aborted) return;
        const transient = isApiError(reason)
          ? reason.status >= 500 || reason.status === 408 || reason.status === 429
          : reason instanceof TypeError || (reason instanceof DOMException && reason.name === "TimeoutError");
        if (!completing && transient && ++failures <= 3) {
          setAuthorizationRetrying(true);
          pollTimer = window.setTimeout(() => void poll(), failures * 1_500);
          return;
        }
        fail(transient
          ? t("personAiProfiles.errors.authorizationUnavailable")
          : reason instanceof Error
          ? reason.message
          : t("personAiProfiles.errors.authorize"));
      } finally {
        polling = false;
      }
    };
    // Background windows may throttle timers while the browser handles consent.
    const resume = () => {
      if (document.visibilityState === "visible") void poll();
    };
    window.addEventListener("focus", resume);
    document.addEventListener("visibilitychange", resume);
    if (scheduleExpiry(authorization.expiresAt)) void poll();
    return () => {
      controller.abort();
      window.removeEventListener("focus", resume);
      document.removeEventListener("visibilitychange", resume);
      window.clearTimeout(pollTimer);
      window.clearTimeout(expiryTimer);
      if (authorizationRequestRef.current === controller) authorizationRequestRef.current = null;
    };
  }, [authorization, navigate, t]);

  function requestAgreementAction(action: AgreementAction) {
    if (!useProfileAgreementStore.getState().checked) {
      setPendingAgreementAction(action);
      return;
    }
    switch (action.type) {
      case "authorize":
        void beginPersonAiAuthorization();
        break;
      case "new-account":
        void beginNewAccountAuthorization();
        break;
      case "enter":
        void selectProfile(action.profile);
        break;
    }
  }

  async function beginPersonAiAuthorization() {
    authorizationRequestRef.current?.abort();
    const controller = new AbortController();
    authorizationRequestRef.current = controller;
    setAuthorizing(true);
    setAuthorizationRetrying(false);
    setCompletingAuthorization(false);
    setError(null);
    try {
      const nextAuthorization = await createPersonAiAuthorization(
        AbortSignal.any([controller.signal, AbortSignal.timeout(30_000)])
      );
      if (controller.signal.aborted) return;
      setAuthorization(nextAuthorization);
      authorizationWindowRef.current = window.open(
        nextAuthorization.loginUrl,
        "_blank",
        "noopener,noreferrer"
      );
    } catch (reason: unknown) {
      if (controller.signal.aborted) return;
      setError(
        isApiError(reason)
          ? reason.message
          : reason instanceof Error
          ? reason.message
          : t("personAiProfiles.errors.authorize")
      );
    } finally {
      setAuthorizing(false);
    }
  }

  async function beginNewAccountAuthorization() {
    setActivating("pending");
    setError(null);
    try {
      const profile = await createPendingProfile();
      setProfiles((current) =>
        current.some((item) => item.profileId === profile.profileId)
          ? current
          : [...current, profile]
      );
      setBootstrap({
        hasProfiles: true,
        activeProfileId: profile.profileId,
        activeProfile: profile,
        avatarUrl: null,
        account: null,
      });
      await beginPersonAiAuthorization();
    } catch (reason: unknown) {
      setError(
        isApiError(reason)
          ? reason.message
          : reason instanceof Error
          ? reason.message
          : t("personAiProfiles.errors.create")
      );
    } finally {
      setActivating(null);
    }
  }

  function closeAuthorizationWindow() {
    if (
      authorizationWindowRef.current &&
      !authorizationWindowRef.current.closed
    ) {
      authorizationWindowRef.current.close();
    }
    authorizationWindowRef.current = null;
  }

  function cancelPersonAiAuthorization() {
    authorizationRequestRef.current?.abort();
    closeAuthorizationWindow();
    setAuthorization(null);
    setCompletingAuthorization(false);
    setAuthorizationRetrying(false);
    setError(null);
  }

  async function selectProfile(profile: PersonAiProfile) {
    setActivating(profile.profileId);
    setError(null);
    try {
      await activateProfile(profile.profileId);
      const authenticated = await useAuthStore.getState().ensureSessionFresh();
      if (!authenticated) {
        navigate("/profiles?mode=reauth", { replace: true });
        return;
      }
      void enterWorkspace();
    } catch (reason: unknown) {
      setError(
        isApiError(reason)
          ? reason.message
          : reason instanceof Error
          ? reason.message
          : t("personAiProfiles.errors.activate")
      );
    } finally {
      setActivating(null);
    }
  }

  async function confirmRemoveProfile() {
    if (!profileToRemove) return;
    setRemovingProfile(true);
    setError(null);
    try {
      await removeProfile(profileToRemove.profileId);
      const [nextProfiles, nextBootstrap] = await Promise.all([
        fetchProfiles(),
        fetchBootstrapProfileStatus(),
      ]);
      setProfiles(nextProfiles);
      setBootstrap(nextBootstrap);
      setSelectedProfileId(
        firstAvailableProfileId(nextProfiles, nextBootstrap.activeProfileId)
      );
      setProfileToRemove(null);
    } catch (reason: unknown) {
      setError(
        isApiError(reason)
          ? reason.message
          : reason instanceof Error
          ? reason.message
          : t("personAiProfiles.errors.remove")
      );
    } finally {
      setRemovingProfile(false);
    }
  }

  const agreementRules = [1, 2, 3, 4, 5, 6].map((index) =>
    t(`personAiProfiles.agreement.rules.rule${index}`)
  );
  const availableProfiles = profiles.filter(
    (profile) => profile.status !== "PENDING"
  );

  return (
    <main
      data-theme-anchor="app-shell"
      data-tron-entry-ready={!loading ? "" : undefined}
      className="tron-login tron-entry-scene"
    >
      <EntryBackdrop />
      <div className="tron-entry-layout tron-login-layout">
        <header className="tron-login-header">
          <EntryWordmark />
          <Button
            variant="ghost"
            type="button"
            className="tron-login-locale inline-flex h-8 shrink-0 items-center gap-1.5 rounded-md px-2 text-xs text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-surface-container)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]"
            aria-label={t("common:locale.label")}
            title={t("common:locale.label")}
            onClick={() => void changeLocale(nextLocale)}
          >
            <span>
              {currentLocale === "zh-CN"
                ? t("common:locale.shortSimplifiedChinese")
                : t("common:locale.english")}
            </span>
          </Button>
        </header>
        <section
          aria-label={t("personAiProfiles.startUsing")}
          className="tron-login-panel"
        >
          {loading ? (
            <div className="tron-login-loading">
              <EntryLoadingIndicator label={t("personAiProfiles.loading")} />
            </div>
          ) : null}
          {!loading &&
          (authorization ||
            forceReauthorization ||
            bootstrap?.activeProfile?.status === "PENDING") ? (
            <div className="tron-login-actions">
              {forceReauthorization && !authorization ? (
                <p className="mb-4 text-sm text-[var(--color-on-surface-variant)]">
                  {t("personAiProfiles.reauthorizationRequired")}
                </p>
              ) : null}
              {authorization ? (
                <div className="space-y-2">
                  <Button className="tron-login-action" type="button" disabled>
                    {t(
                      completingAuthorization
                        ? "personAiProfiles.signingIn"
                        : authorizationRetrying
                        ? "personAiProfiles.retryingAuthorization"
                        : "personAiProfiles.authorizing"
                    )}
                  </Button>
                  {!completingAuthorization ? (
                    <Button
                      className="tron-login-action"
                      type="button"
                      variant="secondary"
                      onClick={() => window.open(authorization.loginUrl, "_blank", "noopener,noreferrer")}
                    >
                      {t("personAiProfiles.reopenAuthorization")}
                    </Button>
                  ) : null}
                  {!completingAuthorization ? (
                    <Button
                      className="tron-login-action"
                      type="button"
                      variant="secondary"
                      onClick={cancelPersonAiAuthorization}
                    >
                      {t("personAiProfiles.cancelAuthorization")}
                    </Button>
                  ) : null}
                </div>
              ) : (
                <Button
                  className="tron-login-action"
                  type="button"
                  disabled={authorizing}
                  onClick={() => requestAgreementAction({ type: "authorize" })}
                >
                  {authorizing
                    ? t("personAiProfiles.authorizing")
                    : forceReauthorization
                    ? t("personAiProfiles.reauthorize")
                    : t("personAiProfiles.authorizeNewAccount")}
                </Button>
              )}
            </div>
          ) : null}
          {!loading &&
          !forceReauthorization &&
          !authorization &&
          bootstrap?.activeProfile?.status !== "PENDING" &&
          availableProfiles.length === 0 ? (
            <div className="tron-login-actions">
              <Button
                className="tron-login-action"
                type="button"
                disabled={activating !== null || authorizing}
                onClick={() => requestAgreementAction({ type: "new-account" })}
              >
                {activating === "pending"
                  ? t("personAiProfiles.switching")
                  : t("personAiProfiles.authorizeNewAccount")}
              </Button>
            </div>
          ) : null}
          {!loading &&
          !forceReauthorization &&
          !authorization &&
          !authorizing &&
          activating !== "pending" &&
          availableProfiles.length > 0 ? (
            <div className="tron-login-actions">
              <div className="tron-login-accounts" data-multiple={availableProfiles.length > 1 || undefined}>
                {availableProfiles.map((profile) => (
                  <div key={profile.profileId} className="tron-login-account">
                    <Button
                      type="button"
                      variant="ghost"
                      className="tron-login-account-select"
                      disabled={activating !== null}
                      onClick={() => setSelectedProfileId(profile.profileId)}
                      aria-pressed={selectedProfileId === profile.profileId}
                      aria-current={bootstrap?.activeProfileId === profile.profileId ? "true" : undefined}
                    >
                      <ProfileAvatar profile={profile} />
                      <span className="min-w-0 flex-1 truncate text-left font-medium">
                        {profile.personAiUsername}
                      </span>
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      className="tron-login-remove"
                      disabled={activating !== null}
                      aria-label={t("personAiProfiles.remove.ariaLabel")}
                      title={t("personAiProfiles.remove.ariaLabel")}
                      onClick={(event) => {
                        event.stopPropagation();
                        setError(null);
                        setProfileToRemove(profile);
                      }}
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  </div>
                ))}
              </div>
              <div className="tron-login-profile-actions">
                <Button
                  className="tron-login-action"
                  type="button"
                  disabled={activating !== null || selectedProfileId === null}
                  onClick={() => {
                    const profile = availableProfiles.find(
                      (item) => item.profileId === selectedProfileId
                    );
                    if (profile) requestAgreementAction({ type: "enter", profile });
                  }}
                >
                  {activating && activating !== "pending"
                    ? t("personAiProfiles.switching")
                    : t("personAiProfiles.startUsing")}
                </Button>
                {bootstrap?.activeProfile?.status !== "PENDING" ? (
                  <Button
                    className="tron-login-action"
                    type="button"
                    variant="secondary"
                    disabled={activating !== null || authorizing}
                    onClick={() => requestAgreementAction({ type: "new-account" })}
                  >
                    {activating === "pending"
                      ? t("personAiProfiles.switching")
                      : t("personAiProfiles.authorizeNewAccount")}
                  </Button>
                ) : null}
              </div>
            </div>
          ) : null}
          {!loading && !authorization ? (
            <div className="tron-login-agreement">
              <input
                id="person-ai-agreement"
                type="checkbox"
                checked={agreementChecked}
                disabled={authorizing || activating !== null}
                onChange={(event) => setAgreementChecked(event.target.checked)}
                className="size-4 accent-[var(--color-primary)]"
              />
              <div>
                <label htmlFor="person-ai-agreement">
                  {t("personAiProfiles.agreement.checkboxLabel")}
                </label>{" "}
                <button
                  type="button"
                  className="font-medium text-[var(--color-on-surface)] underline underline-offset-2"
                  onClick={() => setAgreementOpen(true)}
                >
                  {t("personAiProfiles.agreement.linkLabel")}
                </button>
              </div>
            </div>
          ) : null}
          {error && !profileToRemove ? (
            <p role="alert" className="tron-login-error">
              {error}
            </p>
          ) : null}
        </section>
      </div>
      <Dialog
        open={pendingAgreementAction !== null && !agreementOpen}
        onOpenChange={(open) => {
          if (!open) setPendingAgreementAction(null);
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{t("personAiProfiles.agreement.confirmTitle")}</DialogTitle>
            <DialogDescription>
              {t("personAiProfiles.agreement.confirmDescription")}
            </DialogDescription>
          </DialogHeader>
          <Button
            type="button"
            variant="link"
            className="w-fit px-0"
            onClick={() => {
              setAgreementOpen(true);
            }}
          >
            {t("personAiProfiles.agreement.linkLabel")}
          </Button>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setPendingAgreementAction(null)}
            >
              {t("personAiProfiles.agreement.cancel")}
            </Button>
            <Button
              type="button"
              onClick={() => {
                const action = pendingAgreementAction;
                setPendingAgreementAction(null);
                setAgreementChecked(true);
                if (action) requestAgreementAction(action);
              }}
            >
              {t("personAiProfiles.agreement.confirmAndContinue")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog open={agreementOpen} onOpenChange={setAgreementOpen}>
        <DialogContent className="max-h-[80vh] overflow-y-auto sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>{t("personAiProfiles.agreement.title")}</DialogTitle>
            <DialogDescription>
              {t("personAiProfiles.agreement.description")}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] p-4 text-sm leading-6 text-[var(--color-on-surface-variant)]">
            {agreementRules.map((rule, index) => (
              <p key={rule}>
                <span className="mr-2 font-semibold text-[var(--color-on-surface)]">
                  {index + 1}.
                </span>
                {rule}
              </p>
            ))}
          </div>
          <DialogFooter>
            <Button
              type="button"
              onClick={() => {
                const action = pendingAgreementAction;
                setPendingAgreementAction(null);
                setAgreementChecked(true);
                setAgreementOpen(false);
                if (action) requestAgreementAction(action);
              }}
            >
              {t("personAiProfiles.agreement.acknowledge")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog
        open={profileToRemove !== null}
        onOpenChange={(open) => {
          if (!open) {
            setProfileToRemove(null);
            setError(null);
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{t("personAiProfiles.remove.title")}</DialogTitle>
            <DialogDescription>
              {t("personAiProfiles.remove.description", {
                username:
                  profileToRemove?.personAiUsername ??
                  t("personAiProfiles.remove.unknownAccount"),
              })}
            </DialogDescription>
          </DialogHeader>
          {error ? (
            <p role="alert" className="text-sm text-[var(--color-error)]">
              {error}
            </p>
          ) : null}
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setProfileToRemove(null)}
            >
              {t("personAiProfiles.remove.cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={removingProfile || activating !== null}
              onClick={() => void confirmRemoveProfile()}
            >
              {removingProfile
                ? t("personAiProfiles.remove.confirming")
                : t("personAiProfiles.remove.confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </main>
  );
}

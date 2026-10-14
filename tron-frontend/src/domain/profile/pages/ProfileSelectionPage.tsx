import { useAwakeningBackdrop } from "@/domain/dude/awakening/HomeArrivalVeil";
import { useMotionPreference } from "@/shared/motion/motionPreferences";
import { BlackHole } from "@/domain/dude/awakening/BlackHole";
import { getAwakening } from "@/domain/dude/api/dudeApi";
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
    <span className="flex size-11 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-primary)] text-sm font-semibold text-[var(--color-on-primary)]">
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
  const { beginArrival } = useAwakeningBackdrop();
  const motion = useMotionPreference();
  const enteringRef = useRef(false);
  const enteredAt = useRef(performance.now());
  const entryTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mounted = useRef(true);
  const [scenePhase, setScenePhase] = useState<"approach" | "login" | "dive">(
    "approach"
  );
  useEffect(() => {
    mounted.current = true;
    const reveal = setTimeout(
      () => setScenePhase((phase) => (phase === "approach" ? "login" : phase)),
      2400
    );
    return () => {
      mounted.current = false;
      clearTimeout(reveal);
      if (entryTimer.current) clearTimeout(entryTimer.current);
    };
  }, []);
  async function enterWorkspace() {
    if (enteringRef.current) return;
    enteringRef.current = true;
    try {
      const awakening = await getAwakening();
      if (!mounted.current) return;
      // Auto-login can finish while the approach is still playing. Complete the
      // approach first, then cross the horizon; never jump between camera poses.
      const delay = Math.max(0, 2400 - (performance.now() - enteredAt.current));
      entryTimer.current = setTimeout(() => {
        setScenePhase("dive");
        entryTimer.current = setTimeout(() => {
          if (awakening.started || awakening.state === "Ready") {
            beginArrival(motion !== "system" || matchMedia("(prefers-reduced-motion: reduce)").matches);
          }
          navigate(
            !awakening.started && awakening.state !== "Ready"
              ? "/awakening?arrival=inside"
              : "/workspace",
            {
              replace: true,
            }
          );
        }, 1800);
      }, delay);
    } catch (reason) {
      enteringRef.current = false;
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }
  const authStatus = useAuthStore((state) => state.status);
  const [searchParams] = useSearchParams();
  const forceReauthorization = searchParams.get("mode") === "reauth";
  const forceSelection =
    searchParams.get("mode") === "switch" || forceReauthorization;
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
  const [agreementChecked, setAgreementChecked] = useState(false);
  const [agreementOpen, setAgreementOpen] = useState(false);
  const [profileToRemove, setProfileToRemove] =
    useState<PersonAiProfile | null>(null);
  const [removingProfile, setRemovingProfile] = useState(false);
  const [authorization, setAuthorization] =
    useState<PersonAiAuthorizationSession | null>(null);
  const authorizationWindowRef = useRef<Window | null>(null);
  const authorizationPollTimerRef = useRef<number | null>(null);
  const autoEnteredProfileRef = useRef<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      fetchProfiles(controller.signal),
      fetchBootstrapProfileStatus(controller.signal, false),
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
    return () => controller.abort();
  }, []);

  useEffect(() => {
    if (
      forceSelection ||
      loading ||
      authorization ||
      error ||
      authStatus === "checking" ||
      bootstrap?.activeProfile?.status === "PENDING"
    )
      return;
    const availableProfiles = profiles.filter(
      (profile) => profile.status !== "PENDING"
    );
    if (availableProfiles.length !== 1) return;
    const [profile] = availableProfiles;
    if (autoEnteredProfileRef.current === profile.profileId) return;
    autoEnteredProfileRef.current = profile.profileId;
    if (bootstrap?.activeProfileId === profile.profileId) {
      // A locally ACTIVE profile does not prove that its remote credentials work.
      // Re-entering a protected route while unauthenticated creates a redirect loop.
      if (authStatus === "authenticated") void enterWorkspace();
      else navigate("/profiles?mode=reauth", { replace: true });
      return;
    }
    void selectProfile(profile);
  }, [
    authStatus,
    authorization,
    bootstrap,
    error,
    forceSelection,
    loading,
    navigate,
    profiles,
  ]);

  useEffect(() => {
    if (!authorization) return;
    let active = true;
    const poll = async () => {
      try {
        const result = await pollPersonAiAuthorization(authorization.sessionId);
        if (!active) return;
        if (!result.completed) {
          authorizationPollTimerRef.current = window.setTimeout(() => {
            authorizationPollTimerRef.current = null;
            void poll();
          }, 1_500);
          return;
        }
        setCompletingAuthorization(true);
        closeAuthorizationWindow();
        const [nextProfiles, nextBootstrap] = await Promise.all([
          fetchProfiles(),
          fetchBootstrapProfileStatus(undefined, false),
        ]);
        if (!active) return;
        // Keep authorization pending until the session is usable. Clearing it
        // earlier lets auto-entry mistake the old auth status for expired access.
        const authenticated = await useAuthStore
          .getState()
          .ensureSessionFresh();
        if (!active) return;
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
        if (active) {
          setAuthorization(null);
          setCompletingAuthorization(false);
          closeAuthorizationWindow();
          setError(
            isApiError(reason)
              ? reason.message
              : reason instanceof Error
              ? reason.message
              : t("personAiProfiles.errors.authorize")
          );
        }
      }
    };
    void poll();
    return () => {
      active = false;
      if (authorizationPollTimerRef.current !== null) {
        window.clearTimeout(authorizationPollTimerRef.current);
        authorizationPollTimerRef.current = null;
      }
    };
  }, [authorization, navigate, t]);

  async function beginPersonAiAuthorization() {
    setAuthorizing(true);
    setError(null);
    try {
      const nextAuthorization = await createPersonAiAuthorization();
      setAuthorization(nextAuthorization);
      authorizationWindowRef.current = window.open(
        nextAuthorization.loginUrl,
        "_blank",
        "noopener,noreferrer"
      );
    } catch (reason: unknown) {
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
    closeAuthorizationWindow();
    setAuthorization(null);
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
        fetchBootstrapProfileStatus(undefined, false),
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
      data-camera-phase={scenePhase}
      className="tron-login relative flex min-h-screen items-center justify-center overflow-hidden bg-[var(--color-background)] px-6 text-[var(--color-on-surface)]"
    >
      <BlackHole arriving diving={scenePhase === "dive"} />
      <section
        data-theme-anchor="card"
        inert={scenePhase !== "login"}
        className="tron-login-panel relative z-10 w-full max-w-md rounded-2xl border border-[var(--color-outline)] bg-[var(--color-surface)] p-8 shadow-sm"
      >
        <header className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-xs font-semibold uppercase tracking-[0.18em] text-[var(--color-primary)]">
              Tron
            </p>
            <h1 className="mt-3 text-2xl font-semibold">
              {t("personAiProfiles.title")}
            </h1>
          </div>
          <button
            type="button"
            className="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-md px-2 text-xs text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-surface-container)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]"
            aria-label={t("common:locale.label")}
            title={t("common:locale.label")}
            onClick={() => void changeLocale(nextLocale)}
          >
            <span>
              {currentLocale === "zh-CN"
                ? t("common:locale.shortSimplifiedChinese")
                : t("common:locale.english")}
            </span>
          </button>
        </header>
        {loading ? (
          <p className="mt-8 text-sm text-[var(--color-on-surface-variant)]">
            {t("personAiProfiles.loading")}
          </p>
        ) : null}
        {!loading &&
        !forceReauthorization &&
        !authorization &&
        bootstrap?.activeProfile?.status !== "PENDING" ? (
          <div className="mt-8 flex items-center gap-2 rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] px-3 py-3 text-sm text-[var(--color-on-surface-variant)]">
            <input
              id="person-ai-agreement"
              type="checkbox"
              checked={agreementChecked}
              onChange={(event) => setAgreementChecked(event.target.checked)}
              className="size-4 accent-[var(--color-primary)]"
            />
            <label htmlFor="person-ai-agreement">
              {t("personAiProfiles.agreement.checkboxLabel")}
            </label>
            <button
              type="button"
              className="font-medium text-[var(--color-on-surface)] underline underline-offset-2"
              onClick={() => setAgreementOpen(true)}
            >
              {t("personAiProfiles.agreement.linkLabel")}
            </button>
          </div>
        ) : null}
        {!loading &&
        (authorization ||
          forceReauthorization ||
          bootstrap?.activeProfile?.status === "PENDING") ? (
          <div className="mt-8">
            {forceReauthorization && !authorization ? (
              <p className="mb-4 text-sm text-[var(--color-on-surface-variant)]">
                {t("personAiProfiles.reauthorizationRequired")}
              </p>
            ) : null}
            {authorization ? (
              <div className="space-y-2">
                <Button className="h-10 w-full" type="button" disabled>
                  {t(
                    completingAuthorization
                      ? "personAiProfiles.signingIn"
                      : "personAiProfiles.authorizing"
                  )}
                </Button>
                {!completingAuthorization ? (
                  <Button
                    className="h-10 w-full"
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
                className="h-10 w-full"
                type="button"
                disabled={authorizing}
                onClick={() => void beginPersonAiAuthorization()}
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
        bootstrap?.activeProfile?.status !== "PENDING" &&
        availableProfiles.length === 0 ? (
          <div className="mt-8">
            <Button
              className="h-10 w-full"
              type="button"
              disabled={activating !== null || !agreementChecked}
              onClick={() => void beginNewAccountAuthorization()}
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
          <div className="mt-3 space-y-3">
            {availableProfiles.map((profile) => (
              <div
                key={profile.profileId}
                role="button"
                tabIndex={0}
                onClick={() => setSelectedProfileId(profile.profileId)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ")
                    setSelectedProfileId(profile.profileId);
                }}
                aria-current={
                  bootstrap?.activeProfileId === profile.profileId
                    ? "true"
                    : undefined
                }
                className={`flex w-full items-center gap-4 rounded-xl border-2 bg-[var(--color-surface)] p-4 text-left shadow-sm transition-colors ${
                  selectedProfileId === profile.profileId
                    ? "border-[var(--color-primary)] shadow-[0_0_0_3px_color-mix(in_srgb,var(--color-primary)_10%,transparent)]"
                    : "border-[var(--color-outline-variant)]"
                }`}
              >
                <ProfileAvatar profile={profile} />
                <span className="min-w-0">
                  <p className="truncate font-medium">
                    {profile.personAiUsername}
                  </p>
                </span>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  className="ml-auto shrink-0 text-[var(--color-on-surface)] hover:bg-[var(--color-surface-container)] hover:text-[var(--color-on-surface)]"
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
            <Button
              className="h-10 w-full"
              type="button"
              disabled={activating !== null || selectedProfileId === null}
              onClick={() => {
                const profile = availableProfiles.find(
                  (item) => item.profileId === selectedProfileId
                );
                if (profile) void selectProfile(profile);
              }}
            >
              {activating && activating !== "pending"
                ? t("personAiProfiles.switching")
                : t("personAiProfiles.startUsing")}
            </Button>
            {bootstrap?.activeProfile?.status !== "PENDING" ? (
              <Button
                className="h-10 w-full"
                type="button"
                variant="secondary"
                disabled={activating !== null || !agreementChecked}
                onClick={() => void beginNewAccountAuthorization()}
              >
                {activating === "pending"
                  ? t("personAiProfiles.switching")
                  : t("personAiProfiles.authorizeNewAccount")}
              </Button>
            ) : null}
          </div>
        ) : null}
        {error && !profileToRemove ? (
          <p
            role="alert"
            className="mt-5 text-sm text-[var(--color-destructive)]"
          >
            {error}
          </p>
        ) : null}
      </section>
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
                setAgreementChecked(true);
                setAgreementOpen(false);
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
            <p role="alert" className="text-sm text-[var(--color-destructive)]">
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

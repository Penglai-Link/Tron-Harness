import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  activateProfile,
  createPendingProfile,
  createPersonAiAuthorization,
  fetchBootstrapProfileStatus,
  fetchProfiles,
  pollPersonAiAuthorization,
  type BootstrapProfileStatus,
  type PersonAiAuthorizationSession,
  type PersonAiProfile,
} from "@/domain/profile/api";
import { isApiError } from "@/shared/api/httpClient";

export function ProfileSelectionPage() {
  const { t } = useTranslation("pages");
  const [profiles, setProfiles] = useState<PersonAiProfile[]>([]);
  const [bootstrap, setBootstrap] = useState<BootstrapProfileStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [activating, setActivating] = useState<string | null>(null);
  const [authorizing, setAuthorizing] = useState(false);
  const [authorization, setAuthorization] = useState<PersonAiAuthorizationSession | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([fetchProfiles(controller.signal), fetchBootstrapProfileStatus(controller.signal)])
      .then(([nextProfiles, nextBootstrap]) => {
        setProfiles(nextProfiles);
        setBootstrap(nextBootstrap);
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) {
          setError(reason instanceof Error ? reason.message : t("personAiProfiles.errors.load"));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, []);

  useEffect(() => {
    if (!authorization) return;
    let active = true;
    const poll = async () => {
      try {
        const result = await pollPersonAiAuthorization(authorization.sessionId);
        if (!active) return;
        setAuthorization(result.session);
        if (!result.completed) return;
        const [nextProfiles, nextBootstrap] = await Promise.all([
          fetchProfiles(),
          fetchBootstrapProfileStatus(),
        ]);
        if (!active) return;
        setProfiles(nextProfiles);
        setBootstrap(nextBootstrap);
        setAuthorization(null);
        if (result.profileId && result.restartRequired && window.tronDesktop?.invoke) {
          await window.tronDesktop.invoke("switchProfile", { profileId: result.profileId });
        } else {
          window.location.assign("/home");
        }
      } catch (reason: unknown) {
        if (active) {
          setError(isApiError(reason) ? reason.message : reason instanceof Error ? reason.message : t("personAiProfiles.errors.authorize"));
        }
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 3_000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [authorization, t]);

  async function beginProfile() {
    setActivating("pending");
    setError(null);
    try {
      const profile = await createPendingProfile();
      const invoke = window.tronDesktop?.invoke;
      if (invoke) {
        await invoke("switchProfile", { profileId: profile.profileId });
        return;
      }
      setError(t("personAiProfiles.errors.restart"));
    } catch (reason: unknown) {
      setError(isApiError(reason) ? reason.message : reason instanceof Error ? reason.message : t("personAiProfiles.errors.create"));
    } finally {
      setActivating(null);
    }
  }

  async function beginPersonAiAuthorization() {
    setAuthorizing(true);
    setError(null);
    try {
      const nextAuthorization = await createPersonAiAuthorization();
      setAuthorization(nextAuthorization);
      window.open(nextAuthorization.loginUrl, "_blank", "noopener,noreferrer");
    } catch (reason: unknown) {
      setError(isApiError(reason) ? reason.message : reason instanceof Error ? reason.message : t("personAiProfiles.errors.authorize"));
    } finally {
      setAuthorizing(false);
    }
  }

  async function selectProfile(profile: PersonAiProfile) {
    setActivating(profile.profileId);
    setError(null);
    try {
      await activateProfile(profile.profileId);
      const invoke = window.tronDesktop?.invoke;
      if (invoke) {
        await invoke("switchProfile", { profileId: profile.profileId });
        return;
      }
      setError(t("personAiProfiles.errors.restart"));
    } catch (reason: unknown) {
      setError(isApiError(reason) ? reason.message : reason instanceof Error ? reason.message : t("personAiProfiles.errors.activate"));
    } finally {
      setActivating(null);
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6 text-[var(--color-on-surface)]">
      <section className="w-full max-w-xl rounded-2xl border border-[var(--color-outline-variant)] bg-[var(--color-surface)] p-8 shadow-sm">
        <p className="text-xs font-semibold uppercase tracking-[0.18em] text-[var(--color-primary)]">Tron</p>
        <h1 className="mt-3 text-2xl font-semibold">{t("personAiProfiles.title")}</h1>
        <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">
          {t("personAiProfiles.description")}
        </p>
        {loading ? <p className="mt-8 text-sm">{t("personAiProfiles.loading")}</p> : null}
        {!loading && profiles.length === 0 ? (
          <div className="mt-8 rounded-xl border border-dashed border-[var(--color-outline-variant)] p-5 text-sm text-[var(--color-on-surface-variant)]">
            {t("personAiProfiles.empty")}
            <Button className="mt-4" type="button" disabled={activating !== null} onClick={() => void beginProfile()}>
              {activating === "pending" ? t("personAiProfiles.switching") : t("personAiProfiles.create")}
            </Button>
          </div>
        ) : null}
        {!loading && bootstrap?.activeProfile?.status === "PENDING" ? (
          <div className="mt-8 rounded-xl border border-[var(--color-outline-variant)] p-5">
            <p className="text-sm text-[var(--color-on-surface-variant)]">{t("personAiProfiles.authorizationDescription")}</p>
            <Button className="mt-4" type="button" disabled={authorizing || authorization !== null} onClick={() => void beginPersonAiAuthorization()}>
              {authorization ? t("personAiProfiles.authorizing") : t("personAiProfiles.authorize")}
            </Button>
          </div>
        ) : null}
        <div className="mt-6 space-y-3">
          {profiles.map((profile) => (
            <div key={profile.profileId} className="flex items-center justify-between gap-4 rounded-xl border border-[var(--color-outline-variant)] p-4">
              <div className="min-w-0">
                <p className="truncate font-medium">{profile.personAiUsername}</p>
                <p className="mt-1 truncate text-xs text-[var(--color-on-surface-variant)]">{profile.personAiUserId}</p>
              </div>
              <Button type="button" variant={profile.status === "ACTIVE" ? "secondary" : "default"} disabled={activating !== null || profile.status === "ACTIVE" || profile.status === "PENDING"} onClick={() => void selectProfile(profile)}>
                {activating === profile.profileId ? t("personAiProfiles.switching") : profile.status === "ACTIVE" ? t("personAiProfiles.current") : profile.status === "PENDING" ? t("personAiProfiles.authorize") : t("personAiProfiles.use")}
              </Button>
            </div>
          ))}
        </div>
        {error ? <p role="alert" className="mt-5 text-sm text-[var(--color-destructive)]">{error}</p> : null}
      </section>
    </main>
  );
}

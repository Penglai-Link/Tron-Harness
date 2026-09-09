import {
  useState,
} from "react";
import {
  LoaderCircle,
  LogOut,
  ShieldAlert,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useAuthStore } from "@/store/authStore";
import type { AuthSession } from "@/domain/auth/model";

interface NoWorkspaceAccessPageProps {
  session: AuthSession;
}

export function NoWorkspaceAccessPage({ session }: NoWorkspaceAccessPageProps) {
  const { t } = useTranslation(["auth", "common"]);
  const logout = useAuthStore((state) => state.logout);
  const [loggingOut, setLoggingOut] = useState(false);

  async function handleLogout() {
    if (loggingOut) {
      return;
    }

    setLoggingOut(true);
    try {
      await logout();
    } finally {
      setLoggingOut(false);
    }
  }

  return (
    <main className="min-h-screen bg-[var(--color-background)] px-6 py-8 text-[var(--color-on-surface)]">
      <section className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-3xl flex-col justify-center">
        <div className="rounded-[8px] border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] p-8 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <div className="flex size-12 items-center justify-center rounded-lg bg-[var(--color-surface-container-high)] text-[var(--color-primary)]">
            <ShieldAlert size={22} aria-hidden="true" />
          </div>
          <p className="mt-6 text-sm font-medium uppercase text-[var(--color-on-surface-variant)]">
            {t("auth:noWorkspace.status")}
          </p>
          <h1 className="mt-3 text-3xl font-semibold leading-tight tracking-normal">
            {t("auth:noWorkspace.title")}
          </h1>
          <p className="mt-4 max-w-2xl text-base leading-7 text-[var(--color-on-surface-variant)]">
            {t("auth:noWorkspace.description")}
          </p>

          <dl className="mt-8 grid gap-4 border-t border-[var(--color-outline-variant)] pt-6 sm:grid-cols-2">
            <div>
              <dt className="text-xs font-medium uppercase text-[var(--color-on-surface-variant)]">
                {t("auth:noWorkspace.account")}
              </dt>
              <dd className="mt-1 text-sm font-semibold">{session.user.displayName}</dd>
              <dd className="mt-1 text-sm text-[var(--color-on-surface-variant)]">{session.user.email}</dd>
            </div>
            <div>
              <dt className="text-xs font-medium uppercase text-[var(--color-on-surface-variant)]">
                {t("auth:noWorkspace.nextStep")}
              </dt>
              <dd className="mt-1 text-sm leading-6 text-[var(--color-on-surface-variant)]">
                {t("auth:noWorkspace.nextStepDescription")}
              </dd>
            </div>
          </dl>

          <div className="mt-8 flex flex-wrap items-center gap-3">
            <Button type="button" onClick={handleLogout} disabled={loggingOut} aria-busy={loggingOut}>
              {loggingOut ? <LoaderCircle className="animate-spin" size={16} aria-hidden="true" /> : <LogOut size={16} aria-hidden="true" />}
              {t("common:actions.logout")}
            </Button>
          </div>
        </div>
      </section>
    </main>
  );
}

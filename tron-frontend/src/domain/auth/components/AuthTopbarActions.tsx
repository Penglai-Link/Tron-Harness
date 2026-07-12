import { useState } from "react";
import {
  ChevronRight,
  LogOut,
  UserRound,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Link,
  useLocation,
  useNavigate,
} from "react-router";

import { LOGIN_PATH } from "@/domain/auth/constants";
import { Button } from "@/shared/ui/Button";
import { useAppShellStore } from "@/store/appShellStore";
import { useAuthStore } from "@/store/authStore";

export function AuthUserMenuPanel() {
  const { t } = useTranslation(["auth", "common"]);
  const navigate = useNavigate();
  const location = useLocation();
  const closeFloatingPanels = useAppShellStore((state) => state.closeFloatingPanels);
  const logout = useAuthStore((state) => state.logout);
  const session = useAuthStore((state) => state.session);
  const [submitting, setSubmitting] = useState(false);

  if (!session) {
    return null;
  }

  async function handleLogout() {
    if (submitting) return;

    setSubmitting(true);

    try {
      const logoutTask = logout();
      navigate(LOGIN_PATH, { replace: true, state: { from: location.pathname } });
      await logoutTask;
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="space-y-3">
      <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-3 py-3">
        <p className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[var(--color-on-surface-variant)]">
          {t("auth:userMenu.signedIn")}
        </p>
        <p className="mt-2 text-sm font-medium text-[var(--color-on-surface)]">{session.user.displayName}</p>
        <p className="text-[12px] text-[var(--color-on-surface-variant)]">{session.user.email}</p>
        {session.provider ? (
          <span className="mt-3 inline-flex rounded-full bg-[var(--color-elevated-surface)] px-2.5 py-1 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
            {session.provider.name}
          </span>
        ) : null}
        <Link
          className="mt-3 flex h-9 items-center gap-2 border-t border-[var(--color-outline-variant)] pt-3 text-[13px] font-medium text-[var(--color-on-surface-variant)] transition-colors hover:text-[var(--color-on-surface)]"
          to="/account"
          onClick={closeFloatingPanels}
        >
          <UserRound size={16} aria-hidden="true" />
          <span>{t("auth:userMenu.personalCenter")}</span>
          <ChevronRight className="ml-auto" size={15} aria-hidden="true" />
        </Link>
      </div>
      <Button
        className="w-full justify-center"
        disabled={submitting}
        type="button"
        variant="secondary"
        onClick={() => void handleLogout()}
      >
        <LogOut size={16} aria-hidden="true" />
        {submitting ? t("auth:userMenu.loggingOut") : t("common:actions.logout")}
      </Button>
    </div>
  );
}

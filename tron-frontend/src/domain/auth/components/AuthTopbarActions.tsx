import { useState } from "react";
import { ChevronRight, LogOut, UserRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link, useLocation, useNavigate } from "react-router";

import { LOGIN_PATH } from "@/domain/auth/constants";
import { Button } from "@/shared/ui/Button";
import { useAppShellStore } from "@/store/appShellStore";
import { useAuthStore } from "@/store/authStore";

export function AuthUserMenuPanel() {
  const { t } = useTranslation(["auth", "common"]);
  const navigate = useNavigate();
  const location = useLocation();
  const closeFloatingPanels = useAppShellStore(
    (state) => state.closeFloatingPanels,
  );
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
      navigate(LOGIN_PATH, {
        replace: true,
        state: { from: location.pathname },
      });
      await logoutTask;
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div>
      <div className="border-t border-[var(--color-card-border)] p-2">
        <Link
          className="flex h-10 items-center gap-[12px] rounded-lg px-2 text-[13px] font-medium text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
          to="/account"
          onClick={closeFloatingPanels}
        >
          <UserRound size={16} aria-hidden="true" />
          <span>{t("auth:userMenu.personalCenter")}</span>
          <ChevronRight className="ml-auto" size={15} aria-hidden="true" />
        </Link>
      </div>
      <div className="border-t border-[var(--color-card-border)] p-2">
        <Button
          className="w-full justify-start px-2"
          disabled={submitting}
          type="button"
          variant="ghost"
          onClick={() => void handleLogout()}
        >
          <LogOut size={16} aria-hidden="true" />
          {submitting
            ? t("auth:userMenu.loggingOut")
            : t("common:actions.logout")}
        </Button>
      </div>
    </div>
  );
}

import { useState } from "react";
import { LogOut } from "lucide-react";
import { useLocation, useNavigate } from "react-router";

import { LOGIN_PATH } from "@/domain/auth/constants";
import { Button } from "@/shared/ui/Button";
import { useAuthStore } from "@/store/authStore";

export function AuthUserMenuPanel() {
  const navigate = useNavigate();
  const location = useLocation();
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
          Signed in
        </p>
        <p className="mt-2 text-sm font-medium text-[var(--color-on-surface)]">{session.user.displayName}</p>
        <p className="text-[12px] text-[var(--color-on-surface-variant)]">{session.user.email}</p>
        {session.provider ? (
          <span className="mt-3 inline-flex rounded-full bg-white px-2.5 py-1 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
            {session.provider.name}
          </span>
        ) : null}
      </div>
      <Button
        className="w-full justify-center"
        disabled={submitting}
        type="button"
        variant="secondary"
        onClick={() => void handleLogout()}
      >
        <LogOut size={16} aria-hidden="true" />
        {submitting ? "Logging out..." : "Log out"}
      </Button>
    </div>
  );
}

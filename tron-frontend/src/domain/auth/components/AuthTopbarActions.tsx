import { useState } from "react";
import { LogOut } from "lucide-react";
import { useLocation, useNavigate } from "react-router";

import { LOGIN_PATH } from "@/domain/auth/constants";
import { Button } from "@/shared/ui/Button";
import { useAuthStore } from "@/store/authStore";

export function AuthTopbarActions() {
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
    <>
      <div className="hidden rounded-lg border border-[var(--color-outline-variant)] bg-white px-3 py-2 text-right lg:block">
        <p className="text-sm font-medium text-[var(--color-on-surface)]">{session.user.displayName}</p>
        <p className="text-[12px] text-[var(--color-on-surface-variant)]">{session.user.email}</p>
      </div>
      <Button
        disabled={submitting}
        type="button"
        variant="secondary"
        onClick={() => void handleLogout()}
      >
        <LogOut size={16} aria-hidden="true" />
        {submitting ? "Logging out..." : "Log out"}
      </Button>
    </>
  );
}

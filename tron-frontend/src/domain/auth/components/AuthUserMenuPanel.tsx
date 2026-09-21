import { ArrowRightLeft } from "lucide-react";
import { Link } from "react-router";

import { useAuthStore } from "@/store/authStore";

export function AuthUserMenuPanel() {
  const session = useAuthStore((state) => state.session);

  if (!session) return null;

  return (
    <Link
      aria-label="切换账号"
      className="inline-flex size-8 shrink-0 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus-ring)]"
      title="切换账号"
      to="/profiles?mode=switch"
    >
      <ArrowRightLeft aria-hidden="true" className="size-4" />
    </Link>
  );
}

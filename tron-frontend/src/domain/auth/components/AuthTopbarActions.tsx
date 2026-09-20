import { Link } from "react-router";
import { useAuthStore } from "@/store/authStore";

export function AuthUserMenuPanel() {
  const session = useAuthStore((state) => state.session);

  if (!session) {
    return null;
  }

  return (
    <div>
      <div className="border-t border-[var(--color-card-border)] p-2">
        <Link
          className="flex h-10 items-center gap-[12px] rounded-lg px-2 text-[13px] font-medium text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
          to="/profiles"
        >
          <span>{session.user.username}</span>
          <span className="ml-auto text-xs">切换 PersonAI 档案</span>
        </Link>
      </div>
    </div>
  );
}

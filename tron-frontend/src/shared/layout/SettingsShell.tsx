import type { ReactNode } from "react";
import {
  ArrowLeft,
  type LucideIcon,
  Settings2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Link,
  useLocation,
} from "react-router";

import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/Button";

interface SettingsNavItem {
  href: string;
  icon?: LucideIcon;
  label: string;
  translationKey: string;
}

interface SettingsShellProps {
  activeItem: string;
  children: ReactNode;
  items: readonly SettingsNavItem[];
}

function resolveReturnTo(state: unknown) {
  if (state && typeof state === "object" && "from" in state && typeof state.from === "string" && state.from.length > 0) {
    return state.from;
  }

  return "/dashboard";
}

export function SettingsShell({ activeItem, children, items }: SettingsShellProps) {
  const { t } = useTranslation(["common", "shell"]);
  const location = useLocation();
  const returnTo = resolveReturnTo(location.state);

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <header className="fixed inset-x-0 top-0 z-40 h-14 border-b border-[var(--color-outline-variant)] bg-[var(--color-header-backdrop)] px-4 backdrop-blur sm:px-6">
        <div className="flex h-full items-center justify-between gap-4">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-lg bg-[var(--color-surface-container)] text-[var(--color-primary)]">
              <Settings2 size={18} aria-hidden="true" />
            </div>
            <div className="min-w-0">
              <p className="truncate text-base font-semibold">{t("shell:settings.title")}</p>
            </div>
          </div>
          <Button asChild variant="secondary">
            <Link to={returnTo}>
              <ArrowLeft size={16} aria-hidden="true" />
              {t("shell:settings.back")}
            </Link>
          </Button>
        </div>
      </header>

      <div className="pt-14 lg:grid lg:min-h-screen lg:grid-cols-[260px_minmax(0,1fr)]">
        <aside className="border-b border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-4 py-4 lg:min-h-[calc(100vh-3.5rem)] lg:border-b-0 lg:border-r lg:px-3 lg:py-6">
          <p className="px-2 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
            {t("shell:settings.navigation")}
          </p>
          <nav className="mt-3 flex gap-2 overflow-x-auto lg:block lg:space-y-1" aria-label={t("shell:settings.navigation")}>
            {items.map((item) => {
              const Icon = item.icon;
              const selected = item.label === activeItem;
              return (
                <Link
                  key={item.href}
                  className={cn(
                    "flex shrink-0 items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors",
                    selected
                      ? "bg-[var(--color-surface-container-high)] font-medium text-[var(--color-on-surface)]"
                      : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                  )}
                  to={item.href}
                  state={location.state}
                  aria-current={selected ? "page" : undefined}
                >
                  {Icon ? <Icon size={16} aria-hidden="true" /> : null}
                  <span>{t(item.translationKey)}</span>
                </Link>
              );
            })}
          </nav>
        </aside>

        <main className="min-w-0 px-6 py-6">{children}</main>
      </div>
    </div>
  );
}

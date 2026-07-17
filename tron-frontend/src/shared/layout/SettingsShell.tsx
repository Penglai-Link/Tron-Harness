import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  ArrowLeft,
  ChevronDown,
  ChevronRight,
  type LucideIcon,
  Settings2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Link,
  useLocation,
} from "react-router";

import { cn } from "@/shared/lib/utils";
import { LinkButton } from "@/shared/ui/Button";

interface SettingsNavItem {
  href: string;
  icon?: LucideIcon;
  label: string;
  translationKey: string;
}

interface SettingsNavGroup {
  groupKey: string;
  items: readonly SettingsNavItem[];
  label: string;
  requiredRoleCodes?: readonly string[];
  translationKey: string;
}

interface SettingsShellProps {
  activeItem: string;
  children: ReactNode;
  contentClassName?: string;
  items: readonly SettingsNavGroup[];
  roleCodes?: readonly string[];
}

const settingsReturnToStorageKey = "tron.settings.returnTo";

function resolveReturnTo(state: unknown) {
  if (state && typeof state === "object" && "from" in state && typeof state.from === "string" && state.from.length > 0) {
    return state.from;
  }

  if (typeof window !== "undefined") {
    const storedReturnTo = window.sessionStorage.getItem(settingsReturnToStorageKey);
    if (storedReturnTo) {
      return storedReturnTo;
    }
  }

  return "/dashboard";
}

export function SettingsShell({ activeItem, children, contentClassName, items, roleCodes }: SettingsShellProps) {
  const { t } = useTranslation(["common", "shell"]);
  const location = useLocation();
  const returnTo = resolveReturnTo(location.state);
  const visibleItems = useMemo(
    () => items.filter((group) => {
      if (!group.requiredRoleCodes?.length) {
        return true;
      }

      if (!roleCodes) {
        return true;
      }

      return group.requiredRoleCodes.some((roleCode) => roleCodes.includes(roleCode));
    }),
    [items, roleCodes],
  );
  const flatItems = useMemo(() => visibleItems.flatMap((group) => group.items), [visibleItems]);
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>(() => (
    Object.fromEntries(visibleItems.map((group) => [group.groupKey, false]))
  ));
  const mobileActiveItemRef = useRef<HTMLAnchorElement | null>(null);
  const activeGroupKey = useMemo(
    () => visibleItems.find((group) => group.items.some((item) => item.label === activeItem))?.groupKey ?? null,
    [activeItem, visibleItems],
  );

  useEffect(() => {
    if (typeof window !== "undefined" && returnTo !== "/dashboard") {
      window.sessionStorage.setItem(settingsReturnToStorageKey, returnTo);
    }
  }, [returnTo]);

  useEffect(() => {
    if (!activeGroupKey) return;
    setCollapsedGroups((current) => (
      current[activeGroupKey] ? { ...current, [activeGroupKey]: false } : current
    ));
  }, [activeGroupKey]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      mobileActiveItemRef.current?.scrollIntoView({
        behavior: "auto",
        block: "nearest",
        inline: "center",
      });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [activeItem]);

  function toggleGroup(groupKey: string) {
    if (groupKey === activeGroupKey) {
      return;
    }

    setCollapsedGroups((current) => ({ ...current, [groupKey]: !current[groupKey] }));
  }

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)] lg:h-screen lg:overflow-hidden">
      <header className="fixed inset-x-0 top-0 z-40 h-14 bg-[var(--color-header-backdrop)] px-4 shadow-[0_10px_28px_rgba(15,23,42,0.045)] backdrop-blur sm:px-6">
        <div className="flex h-full items-center justify-between gap-4">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-lg bg-[var(--color-surface-container)] text-[var(--color-primary)]">
              <Settings2 size={18} aria-hidden="true" />
            </div>
            <div className="min-w-0">
              <p className="truncate text-base font-semibold">{t("shell:settings.title")}</p>
            </div>
          </div>
          <LinkButton to={returnTo} variant="secondary">
            <ArrowLeft size={16} aria-hidden="true" />
            {t("shell:settings.back")}
          </LinkButton>
        </div>
      </header>

      <div className="pt-14 lg:grid lg:h-screen lg:grid-cols-[260px_minmax(0,1fr)] lg:overflow-hidden">
        <aside className="bg-[var(--color-elevated-surface)] px-4 py-4 shadow-[0_10px_24px_rgba(15,23,42,0.03)] lg:h-[calc(100vh-3.5rem)] lg:overflow-y-auto lg:px-3 lg:py-6 lg:shadow-[10px_0_28px_rgba(15,23,42,0.04)]">
          <p className="px-2 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
            {t("shell:settings.navigation")}
          </p>
          <nav className="mt-3 flex gap-2 overflow-x-auto lg:hidden" aria-label={t("shell:settings.navigation")}>
            {flatItems.map((item) => {
              const Icon = item.icon;
              const selected = item.label === activeItem;
              return (
                <Link
                  ref={selected ? mobileActiveItemRef : undefined}
                  key={item.href}
                  className={cn(
                    "flex shrink-0 items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors",
                    selected
                      ? "bg-[var(--color-surface-container-high)] font-medium text-[var(--color-on-surface)]"
                      : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                  )}
                  to={item.href}
                  state={{ from: returnTo }}
                  aria-current={selected ? "page" : undefined}
                >
                  {Icon ? <Icon size={16} aria-hidden="true" /> : null}
                  <span>{t(item.translationKey)}</span>
                </Link>
              );
            })}
          </nav>
          <div className="mt-3 hidden space-y-4 lg:block">
            {visibleItems.map((group) => {
              const expanded = !collapsedGroups[group.groupKey];
              const isActiveGroup = group.groupKey === activeGroupKey;
              const GroupChevron = expanded ? ChevronDown : ChevronRight;

              return (
                <section key={group.groupKey}>
                  <button
                    type="button"
                    className={cn(
                      "flex w-full items-center justify-between gap-3 px-2 py-1 text-left transition-colors",
                      isActiveGroup
                        ? "text-[var(--color-on-surface)]"
                        : "text-[var(--color-on-surface-variant)] hover:text-[var(--color-on-surface)]",
                    )}
                    onClick={() => toggleGroup(group.groupKey)}
                    aria-expanded={expanded}
                    aria-controls={`settings-group-${group.groupKey}`}
                  >
                    <span className="text-[11px] font-semibold uppercase tracking-[0.05em]">
                      {t(group.translationKey)}
                    </span>
                    <GroupChevron size={16} aria-hidden="true" />
                  </button>
                  {expanded ? (
                    <div id={`settings-group-${group.groupKey}`} className="mt-1 space-y-1">
                      {group.items.map((item) => {
                        const Icon = item.icon;
                        const selected = item.label === activeItem;

                        return (
                          <Link
                            key={item.href}
                            className={cn(
                              "flex items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors",
                              selected
                                ? "bg-[var(--color-surface-container-high)] font-medium text-[var(--color-on-surface)]"
                                : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                            )}
                            to={item.href}
                            state={{ from: returnTo }}
                            aria-current={selected ? "page" : undefined}
                          >
                            {Icon ? <Icon size={16} aria-hidden="true" /> : null}
                            <span>{t(item.translationKey)}</span>
                          </Link>
                        );
                      })}
                    </div>
                  ) : null}
                </section>
              );
            })}
          </div>
        </aside>

        <main className={cn(
          "min-w-0 lg:h-[calc(100vh-3.5rem)]",
          contentClassName ?? "px-6 py-6 lg:overflow-y-auto",
        )}
        >
          {children}
        </main>
      </div>
    </div>
  );
}

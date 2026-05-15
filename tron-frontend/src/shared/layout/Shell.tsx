import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  type ReactNode,
} from "react";

import {
  Bell,
  Bot,
  ChevronDown,
  CircleUserRound,
  Command,
  PanelLeftClose,
  PanelLeftOpen,
  ShieldCheck,
  SlidersHorizontal,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";

import { tronNavigation } from "@/shared/constants/shellNavigation";
import {
  defaultPrimarySpaceId,
  getSecondarySpaceMocks,
  primarySpaceMocks,
} from "@/shared/constants/workspaceSpaces";
import { WorkspaceSpacesContext } from "@/shared/hooks/useWorkspaceSpaces";
import { Button } from "@/shared/ui/Button";
import { useLocale } from "@/i18n";
import { useAppShellStore } from "@/store/appShellStore";
import { cn } from "@/shared/lib/utils";
import type { TronNavSection } from "@/shared/types/ShellNavigation";
import type { WorkspaceSpacesContextValue } from "@/shared/types/WorkspaceSpace";

interface TronShellProps {
  activeItem?: string;
  children: ReactNode;
  contentClassName?: string;
  rightPanel?: ReactNode;
  searchPlaceholder?: string;
  topbarActions?: ReactNode;
  userMenuContent?: ReactNode;
}

const TronShellFrameContext = createContext(false);

function sectionHasActiveItem(section: TronNavSection, activeItem?: string) {
  return Boolean(activeItem && section.items.some((item) => item.label === activeItem));
}

function InlineTronShellContent({ children, contentClassName, rightPanel }: TronShellProps) {
  return (
    <div className={cn("-mx-6 -my-6 grid min-h-[calc(100vh-3.5rem)] grid-cols-1", rightPanel ? "xl:grid-cols-[minmax(0,1fr)_360px]" : "")}>
      <main className={cn("min-w-0 px-6 py-6", contentClassName)}>{children}</main>
      {rightPanel ? <aside className="border-l border-[var(--color-outline-variant)] bg-white px-5 py-6">{rightPanel}</aside> : null}
    </div>
  );
}

export function TronShell({
  activeItem = "Dashboard",
  children,
  contentClassName,
  rightPanel,
  searchPlaceholder,
  topbarActions,
  userMenuContent,
}: TronShellProps) {
  const { t } = useTranslation(["common", "shell"]);
  const { changeLocale, currentLocale, supportedLocales: locales } = useLocale();
  const shellFrameMounted = useContext(TronShellFrameContext);
  const agentOpen = useAppShellStore((state) => state.agentOpen);
  const ensureSectionOpen = useAppShellStore((state) => state.ensureSectionOpen);
  const openSections = useAppShellStore((state) => state.openSections);
  const secondarySpaceMenuOpen = useAppShellStore((state) => state.secondarySpaceMenuOpen);
  const selectedPrimarySpaceId = useAppShellStore((state) => state.selectedPrimarySpaceId);
  const selectedSecondarySpaceId = useAppShellStore((state) => state.selectedSecondarySpaceId);
  const setAgentOpen = useAppShellStore((state) => state.setAgentOpen);
  const setSidebarCollapsed = useAppShellStore((state) => state.setSidebarCollapsed);
  const sidebarCollapsed = useAppShellStore((state) => state.sidebarCollapsed);
  const toggleAgent = useAppShellStore((state) => state.toggleAgent);
  const toggleSecondarySpaceMenu = useAppShellStore((state) => state.toggleSecondarySpaceMenu);
  const toggleSection = useAppShellStore((state) => state.toggleSection);
  const toggleUtilityPanel = useAppShellStore((state) => state.toggleUtilityPanel);
  const utilityPanel = useAppShellStore((state) => state.utilityPanel);
  const selectPrimarySpaceInStore = useAppShellStore((state) => state.selectPrimarySpace);
  const selectSecondarySpaceInStore = useAppShellStore((state) => state.selectSecondarySpace);

  const primarySpaces = primarySpaceMocks;
  const secondarySpaces = getSecondarySpaceMocks(selectedPrimarySpaceId);
  const selectedPrimarySpace = primarySpaces.find((space) => space.id === selectedPrimarySpaceId);
  const selectedSecondarySpace = secondarySpaces.find((space) => space.id === selectedSecondarySpaceId) ?? secondarySpaces[0];
  const workspaceSpacesValue = useMemo<WorkspaceSpacesContextValue>(
    () => ({
      primarySpaces,
      secondarySpaces,
      selectedPrimarySpace,
      selectedPrimarySpaceId,
      selectedSecondarySpace,
      selectedSecondarySpaceId: selectedSecondarySpace?.id ?? selectedSecondarySpaceId,
    }),
    [primarySpaces, secondarySpaces, selectedPrimarySpace, selectedPrimarySpaceId, selectedSecondarySpace, selectedSecondarySpaceId],
  );

  function selectPrimarySpace(spaceId: string) {
    selectPrimarySpaceInStore(spaceId, getSecondarySpaceMocks(spaceId)[0]?.id ?? "");
  }

  function selectSecondarySpace(spaceId: string) {
    selectSecondarySpaceInStore(spaceId);
  }

  useEffect(() => {
    const activeSection = tronNavigation.find((section) => sectionHasActiveItem(section, activeItem));
    ensureSectionOpen(activeSection?.label);
  }, [activeItem, ensureSectionOpen]);

  if (shellFrameMounted) {
    return <InlineTronShellContent activeItem={activeItem} contentClassName={contentClassName} rightPanel={rightPanel}>{children}</InlineTronShellContent>;
  }

  return (
    <TronShellFrameContext.Provider value={true}>
    <WorkspaceSpacesContext.Provider value={workspaceSpacesValue}>
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <header className="fixed inset-x-0 top-0 z-50 h-14 border-b border-[var(--color-outline-variant)] bg-[rgba(253,248,248,0.92)] px-4 backdrop-blur sm:px-6">
        <div className="flex h-full items-center justify-between gap-4">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <button
              className="inline-flex h-9 shrink-0 items-center gap-2 rounded-lg bg-[var(--color-surface-container)] px-3 text-sm font-medium text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              type="button"
              aria-expanded={utilityPanel === "workspace"}
              onClick={() => toggleUtilityPanel("workspace")}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              {selectedPrimarySpace?.name ?? t("shell:workspace.current")}
              <ChevronDown size={15} aria-hidden="true" />
            </button>
            <Button
              className="hidden lg:inline-flex"
              size="icon"
              type="button"
              variant="ghost"
              aria-label={sidebarCollapsed ? t("shell:aria.expandSidebar") : t("shell:aria.collapseSidebar")}
              aria-pressed={sidebarCollapsed}
              onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            >
              {sidebarCollapsed ? <PanelLeftOpen size={17} aria-hidden="true" /> : <PanelLeftClose size={17} aria-hidden="true" />}
            </Button>
          </div>
          <div className="flex items-center gap-2">
            {topbarActions}
            <Button size="icon" variant="ghost" aria-label={t("shell:aria.openFilters")} aria-pressed={utilityPanel === "filters"} onClick={() => toggleUtilityPanel("filters")}>
              <SlidersHorizontal size={17} aria-hidden="true" />
            </Button>
            <Button size="icon" variant="ghost" aria-label={t("shell:aria.viewNotifications")} aria-pressed={utilityPanel === "notifications"} onClick={() => toggleUtilityPanel("notifications")}>
              <Bell size={17} aria-hidden="true" />
            </Button>
            <Button size="icon" variant="secondary" aria-label={t("shell:aria.openUserMenu")} aria-pressed={utilityPanel === "user"} onClick={() => toggleUtilityPanel("user")}>
              <CircleUserRound size={18} aria-hidden="true" />
            </Button>
          </div>
        </div>
        {utilityPanel ? (
          <div
            className={cn(
              "absolute top-[calc(100%+8px)] w-[min(360px,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-white px-4 py-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
              utilityPanel === "workspace" ? "left-4 sm:left-6" : "right-4 sm:right-6",
            )}
          >
            <p className="font-medium">{t(`shell:utilityPanel.${utilityPanel}`)}</p>
            {utilityPanel === "workspace" ? (
              <div className="mt-3 space-y-2">
                <p className="text-[12px] font-medium text-[var(--color-on-surface-variant)]">{t("shell:workspace.primaryLabel")}</p>
                {primarySpaces.length ? (
                  primarySpaces.map((space) => {
                    const selected = space.id === selectedPrimarySpaceId;
                    return (
                      <button
                        className={cn(
                          "block w-full rounded-lg border px-3 py-2 text-left transition-colors",
                          selected ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]" : "border-transparent hover:bg-[var(--color-hover-surface)]",
                        )}
                        type="button"
                        aria-pressed={selected}
                        key={space.id}
                        onClick={() => selectPrimarySpace(space.id)}
                      >
                        <span className="block text-sm font-medium">{space.name}</span>
                        <span className="mt-0.5 block text-[12px] text-[var(--color-on-surface-variant)]">{space.description}</span>
                        <span className="mt-1 block text-[11px] text-[var(--color-on-surface-variant)]">{space.region} · {t("shell:workspace.status", { status: space.status })}</span>
                      </button>
                    );
                  })
                ) : (
                  <p className="text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.noPrimarySpaces")}</p>
                )}
              </div>
            ) : utilityPanel === "user" ? (
              <div className="mt-3 space-y-4">
                {userMenuContent}
                <div className={cn(userMenuContent ? "border-t border-[var(--color-outline-variant)] pt-4" : "")}>
                  <label className="mb-1 block text-[12px] font-medium text-[var(--color-on-surface-variant)]" htmlFor="tron-locale-select">
                    {t("common:locale.label")}
                  </label>
                  <select
                    className="h-9 w-full rounded-lg border border-[var(--color-outline-variant)] bg-white px-3 text-sm outline-none"
                    id="tron-locale-select"
                    value={currentLocale}
                    onChange={(event) => void changeLocale(event.target.value as typeof currentLocale)}
                  >
                    {locales.map((locale) => (
                      <option key={locale.code} value={locale.code}>
                        {t(locale.labelKey)}
                      </option>
                    ))}
                  </select>
                  <p className="mt-2 text-[12px] text-[var(--color-on-surface-variant)]">{t("common:locale.unavailable")}</p>
                </div>
              </div>
            ) : (
              <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:utilityPanel.sharedState")}</p>
            )}
          </div>
        ) : null}
      </header>

      <aside
        className={cn(
          "fixed bottom-0 left-0 top-14 z-40 hidden flex-col overflow-y-auto border-r border-[var(--color-outline-variant)] bg-white py-4 transition-[width] duration-200 lg:flex",
          sidebarCollapsed ? "w-[76px] px-2" : "w-[280px] px-3",
        )}
        data-collapsed={sidebarCollapsed}
      >
        <div className={cn("mb-6 flex items-center gap-3 px-2", sidebarCollapsed ? "justify-center" : "")}>
          <button
            className={cn("flex w-full items-center gap-3 rounded-lg text-left", sidebarCollapsed ? "justify-center" : "")}
            type="button"
            aria-expanded={secondarySpaceMenuOpen}
            aria-label={selectedSecondarySpace ? `${selectedSecondarySpace.name} ${selectedSecondarySpace.description}` : t("shell:aria.tronMainConsole")}
            onClick={toggleSecondarySpaceMenu}
          >
            <span className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-primary)] text-[var(--color-on-primary)]">
              <Command size={18} aria-hidden="true" />
            </span>
            <span className={cn(sidebarCollapsed ? "sr-only" : "")}>
              <span className="block text-sm font-semibold">{selectedSecondarySpace?.name ?? t("common:brand.name")}</span>
              <span className="block text-xs text-[var(--color-on-surface-variant)]">{selectedSecondarySpace?.description ?? t("common:brand.console")}</span>
            </span>
            {sidebarCollapsed ? null : <ChevronDown className={cn("ml-auto transition-transform", secondarySpaceMenuOpen ? "rotate-180" : "")} size={14} aria-hidden="true" />}
          </button>
        </div>

        <nav className={cn("space-y-4", sidebarCollapsed ? "space-y-2" : "")} aria-label={t("shell:aria.primaryNavigation")}>
          {tronNavigation.map((section) => {
            const open = openSections[section.label];
            return (
              <section key={section.label}>
                {sidebarCollapsed ? null : (
                  <button
                    className="flex h-8 w-full items-center justify-between rounded-lg px-2 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]"
                    type="button"
                    aria-expanded={open}
                    onClick={() => toggleSection(section.label)}
                  >
                    {t(section.translationKey)}
                    <ChevronDown className={cn("transition-transform", open ? "rotate-180" : "")} size={14} aria-hidden="true" />
                  </button>
                )}
                {open || sidebarCollapsed ? (
                  <div className={cn("space-y-1", sidebarCollapsed ? "" : "mt-1")}>
                    {section.items.map((item) => {
                      const Icon = item.icon;
                      const active = item.label === activeItem;
                      const itemLabel = t(item.translationKey);
                      return (
                        <Link
                          className={cn(
                            "flex h-9 items-center rounded-lg text-sm transition-colors",
                            sidebarCollapsed ? "justify-center px-0" : "gap-2 px-3",
                            active
                              ? "bg-[var(--color-surface-container-high)] font-medium text-[var(--color-on-surface)]"
                              : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                          )}
                          aria-current={active ? "page" : undefined}
                          data-active={active}
                          key={item.label}
                          title={sidebarCollapsed ? itemLabel : undefined}
                          to={item.href}
                        >
                          <Icon size={17} aria-hidden="true" />
                          <span className={cn(sidebarCollapsed ? "sr-only" : "")}>{itemLabel}</span>
                        </Link>
                      );
                    })}
                  </div>
                ) : null}
              </section>
            );
          })}
        </nav>
      </aside>

      {secondarySpaceMenuOpen ? (
        <div
          className={cn(
            "fixed top-[4.5rem] z-50 w-[min(340px,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-white px-4 py-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
            sidebarCollapsed ? "left-[84px]" : "left-[292px]",
          )}
        >
          <p className="font-medium">{t("shell:workspace.secondaryLabel")}</p>
          <div className="mt-3 space-y-2">
            {secondarySpaces.length ? (
              secondarySpaces.map((space) => {
                const selected = space.id === selectedSecondarySpace?.id;
                return (
                  <button
                    className={cn(
                      "block w-full rounded-lg border px-3 py-2 text-left transition-colors",
                      selected ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]" : "border-transparent hover:bg-[var(--color-hover-surface)]",
                    )}
                    type="button"
                    aria-pressed={selected}
                    key={space.id}
                    onClick={() => selectSecondarySpace(space.id)}
                  >
                    <span className="block text-sm font-medium">{space.name}</span>
                    <span className="mt-0.5 block text-[12px] text-[var(--color-on-surface-variant)]">{space.description}</span>
                    <span className="mt-1 block text-[11px] text-[var(--color-on-surface-variant)]">
                      {t("shell:workspace.inherited", { count: space.inheritedResources })} · {t("shell:workspace.localOverrides", { count: space.localOverrides })} · {t("shell:workspace.status", { status: space.status })}
                    </span>
                  </button>
                );
              })
            ) : (
              <p className="text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.noSecondarySpaces")}</p>
            )}
          </div>
        </div>
      ) : null}

      <section className={cn("min-w-0 pt-14 transition-[padding-left] duration-200", sidebarCollapsed ? "lg:pl-[76px]" : "lg:pl-[280px]")}>
        <div className={cn("grid min-h-[calc(100vh-3.5rem)] grid-cols-1", rightPanel ? "xl:grid-cols-[minmax(0,1fr)_360px]" : "")}>
          <main className={cn("min-w-0 px-6 py-6", contentClassName)}>{children}</main>
          {rightPanel ? <aside className="border-l border-[var(--color-outline-variant)] bg-white px-5 py-6">{rightPanel}</aside> : null}
        </div>
      </section>

      <button
        className="fixed bottom-6 right-6 z-40 flex h-12 items-center gap-2 rounded-full bg-[var(--color-primary)] px-4 text-sm font-medium text-[var(--color-on-primary)] shadow-[0_18px_45px_rgba(0,0,0,0.18)] transition-transform hover:scale-[1.02] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
        type="button"
        aria-expanded={agentOpen}
        onClick={toggleAgent}
      >
        <Bot size={18} aria-hidden="true" />
        {t("shell:assistant.title")}
      </button>
      {agentOpen ? (
        <section className="fixed bottom-24 right-6 z-40 w-[320px] rounded-2xl border border-[var(--color-card-border)] bg-white p-4 shadow-[0_18px_50px_rgba(0,0,0,0.14)]">
          <p className="text-sm font-semibold">{t("shell:assistant.title")}</p>
          <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">{t("shell:assistant.body")}</p>
          <button className="mt-3 h-8 rounded-lg bg-[var(--color-surface-container)] px-3 text-[13px] font-medium hover:bg-[var(--color-hover-surface)]" type="button" onClick={() => setAgentOpen(false)}>
            {t("common:actions.close")}
          </button>
        </section>
      ) : null}
    </div>
    </WorkspaceSpacesContext.Provider>
    </TronShellFrameContext.Provider>
  );
}

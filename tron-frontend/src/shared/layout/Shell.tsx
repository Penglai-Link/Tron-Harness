import {
  useCallback,
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
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
import {
  Link,
  useLocation,
} from "react-router";

import { fetchWorkspaceSwitcher } from "@/domain/admin/api";
import { isUnauthorizedApiError } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { tronNavigation } from "@/shared/constants/shellNavigation";
import {
  defaultPrimarySpaceId,
  defaultSecondarySpaceId,
  getWorkspaceSwitcherMock,
} from "@/shared/constants/workspaceSpaces";
import { WorkspaceSpacesContext } from "@/shared/hooks/useWorkspaceSpaces";
import { Button } from "@/shared/ui/Button";
import { useLocale } from "@/i18n";
import { useAppShellStore } from "@/store/appShellStore";
import { cn } from "@/shared/lib/utils";
import type { TronNavSection } from "@/shared/types/ShellNavigation";
import type {
  WorkspaceSpacesContextValue,
  WorkspaceSwitcherData,
} from "@/shared/types/WorkspaceSpace";

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

interface SecondarySpaceTreeNode {
  children: SecondarySpaceTreeNode[];
  space: WorkspaceSwitcherData["secondarySpaces"][number];
}

function sectionHasActiveItem(section: TronNavSection, activeItem?: string) {
  return Boolean(activeItem && section.items.some((item) => item.label === activeItem));
}

function buildSecondarySpaceTree(secondarySpaces: WorkspaceSwitcherData["secondarySpaces"]) {
  const roots: SecondarySpaceTreeNode[] = [];
  const stack: SecondarySpaceTreeNode[] = [];

  secondarySpaces.forEach((space) => {
    const node: SecondarySpaceTreeNode = {
      children: [],
      space,
    };

    while (stack.length && stack.at(-1)!.space.depth >= space.depth) {
      stack.pop();
    }

    if (stack.length) {
      stack.at(-1)!.children.push(node);
    } else {
      roots.push(node);
    }

    stack.push(node);
  });

  return roots;
}

function getRelativeSecondarySpaceLabel(label: string, primarySpaceLabel?: string) {
  if (!primarySpaceLabel) {
    return label;
  }

  const prefix = `${primarySpaceLabel} / `;
  return label.startsWith(prefix) ? label.slice(prefix.length) : label;
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
  const { pathname } = useLocation();
  const shellFrameMounted = useContext(TronShellFrameContext);
  const agentOpen = useAppShellStore((state) => state.agentOpen);
  const closeFloatingPanels = useAppShellStore((state) => state.closeFloatingPanels);
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
  const dataSource = getServiceDataSource("admin");
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [workspaceSwitcherLoading, setWorkspaceSwitcherLoading] = useState(false);
  const [workspaceSwitcher, setWorkspaceSwitcher] = useState<WorkspaceSwitcherData>(() => (
    dataSource === "api"
      ? { primarySpaces: [], secondarySpaces: [] }
      : getWorkspaceSwitcherMock()
  ));
  const workspaceSwitcherControllerRef = useRef<AbortController | null>(null);
  const workspaceSwitcherRequestSequenceRef = useRef(0);

  const primarySpaces = workspaceSwitcher.primarySpaces;
  const selectedPrimarySpace = primarySpaces.find((space) => space.workspaceCode === selectedPrimarySpaceId)
    ?? primarySpaces.find((space) => space.workspaceCode === defaultPrimarySpaceId)
    ?? primarySpaces[0];
  const effectivePrimarySpaceId = selectedPrimarySpace?.workspaceCode ?? "";
  const currentPrimarySecondarySpaces = useMemo(
    () => workspaceSwitcher.secondarySpaces.filter((space) => space.primaryWorkspaceCode === effectivePrimarySpaceId),
    [effectivePrimarySpaceId, workspaceSwitcher.secondarySpaces],
  );
  const currentPrimaryRootSpace = useMemo(
    () => currentPrimarySecondarySpaces.find((space) => space.depth === 1 && space.workspaceCode === effectivePrimarySpaceId)
      ?? currentPrimarySecondarySpaces.find((space) => space.depth === 1),
    [currentPrimarySecondarySpaces, effectivePrimarySpaceId],
  );
  const secondarySpaces = useMemo(
    () => currentPrimarySecondarySpaces.filter((space) => space.depth > 1),
    [currentPrimarySecondarySpaces],
  );
  const selectedSecondarySpace = secondarySpaces.find((space) => space.workspaceCode === selectedSecondarySpaceId)
    ?? secondarySpaces.find((space) => space.workspaceCode === defaultSecondarySpaceId)
    ?? secondarySpaces[0];
  const effectiveSecondarySpaceId = selectedSecondarySpace?.workspaceCode ?? "";
  const secondarySpaceLabelPrefix = currentPrimaryRootSpace?.label ?? selectedPrimarySpace?.name;
  const selectedSecondarySpaceLabel = selectedSecondarySpace
    ? getRelativeSecondarySpaceLabel(selectedSecondarySpace.label, secondarySpaceLabelPrefix)
    : t("shell:workspace.secondaryLabel");
  const secondarySpaceTree = useMemo(
    () => buildSecondarySpaceTree(secondarySpaces),
    [secondarySpaces],
  );
  const workspaceSpacesValue = useMemo<WorkspaceSpacesContextValue>(
    () => ({
      primarySpaces,
      secondarySpaces,
      selectedPrimarySpace,
      selectedPrimarySpaceId: effectivePrimarySpaceId,
      selectedSecondarySpace,
      selectedSecondarySpaceId: effectiveSecondarySpaceId,
    }),
    [effectivePrimarySpaceId, effectiveSecondarySpaceId, primarySpaces, secondarySpaces, selectedPrimarySpace, selectedSecondarySpace],
  );

  function selectPrimarySpace(spaceId: string) {
    const firstSecondary = workspaceSwitcher.secondarySpaces.find(
      (space) => space.primaryWorkspaceCode === spaceId && space.depth > 1,
    );
    selectPrimarySpaceInStore(spaceId, firstSecondary?.workspaceCode ?? "");
  }

  function selectSecondarySpace(space: WorkspaceSwitcherData["secondarySpaces"][number]) {
    selectPrimarySpaceInStore(space.primaryWorkspaceCode, space.workspaceCode);
  }

  const refreshWorkspaceSwitcher = useCallback(async () => {
    if (dataSource !== "api") {
      workspaceSwitcherControllerRef.current?.abort();
      workspaceSwitcherControllerRef.current = null;
      workspaceSwitcherRequestSequenceRef.current += 1;
      setWorkspaceSwitcher(getWorkspaceSwitcherMock());
      setWorkspaceSwitcherLoading(false);
      return;
    }

    const requestId = workspaceSwitcherRequestSequenceRef.current + 1;
    workspaceSwitcherRequestSequenceRef.current = requestId;
    workspaceSwitcherControllerRef.current?.abort();
    const controller = new AbortController();
    workspaceSwitcherControllerRef.current = controller;
    setWorkspaceSwitcherLoading(true);

    try {
      const data = await fetchWorkspaceSwitcher(controller.signal);
      if (requestId !== workspaceSwitcherRequestSequenceRef.current || controller.signal.aborted) return;
      setWorkspaceSwitcher(data);
    } catch (error: unknown) {
      if (requestId !== workspaceSwitcherRequestSequenceRef.current || controller.signal.aborted) return;
      if (isUnauthorizedApiError(error)) return;
      console.warn("[shell] Workspace switcher refresh failed.", error);
    } finally {
      if (workspaceSwitcherControllerRef.current === controller) {
        workspaceSwitcherControllerRef.current = null;
      }
      if (!controller.signal.aborted && requestId === workspaceSwitcherRequestSequenceRef.current) {
        setWorkspaceSwitcherLoading(false);
      }
    }
  }, [dataSource]);

  useEffect(() => {
    void refreshWorkspaceSwitcher();

    return () => workspaceSwitcherControllerRef.current?.abort();
  }, [refreshWorkspaceSwitcher]);

  useEffect(() => {
    const activeSection = tronNavigation.find((section) => sectionHasActiveItem(section, activeItem));
    ensureSectionOpen(activeSection?.label);
  }, [activeItem, ensureSectionOpen]);

  useEffect(() => {
    if (!effectivePrimarySpaceId) {
      return;
    }

    if (effectivePrimarySpaceId === selectedPrimarySpaceId && effectiveSecondarySpaceId === selectedSecondarySpaceId) {
      return;
    }

    selectPrimarySpaceInStore(effectivePrimarySpaceId, effectiveSecondarySpaceId);
  }, [
    effectivePrimarySpaceId,
    effectiveSecondarySpaceId,
    selectPrimarySpaceInStore,
    selectedPrimarySpaceId,
    selectedSecondarySpaceId,
  ]);

  useEffect(() => {
    setMobileSidebarOpen(false);
  }, [pathname]);

  useEffect(() => {
    if (!mobileSidebarOpen) {
      return;
    }

    const previousOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMobileSidebarOpen(false);
      }
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [mobileSidebarOpen]);

  function toggleMobileSidebar() {
    if (!mobileSidebarOpen) {
      closeFloatingPanels();
    }

    setMobileSidebarOpen((open) => !open);
  }

  function closeMobileSidebar() {
    closeFloatingPanels();
    setMobileSidebarOpen(false);
  }

  function handleSidebarNavigationClick() {
    closeFloatingPanels();
    closeMobileSidebar();
  }

  function handleWorkspaceSwitcherClick() {
    void refreshWorkspaceSwitcher();
    toggleUtilityPanel("workspace");
  }

  function handleSecondarySpaceSelectorClick() {
    if (!secondarySpaceMenuOpen) {
      void refreshWorkspaceSwitcher();
    }

    toggleSecondarySpaceMenu();
  }

  function renderSecondarySpaceTreeNode(node: SecondarySpaceTreeNode, nestingLevel = 0): ReactNode {
    const { space } = node;
    const selected = space.workspaceCode === selectedSecondarySpace?.workspaceCode;
    const relativeLabel = getRelativeSecondarySpaceLabel(space.label, secondarySpaceLabelPrefix);

    return (
      <div key={space.workspaceCode} className="space-y-1">
        <button
          className={cn(
            "block w-full rounded-lg border px-3 py-2 text-left transition-colors",
            selected
              ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]"
              : "border-transparent hover:bg-[var(--color-hover-surface)]",
          )}
          type="button"
          aria-label={relativeLabel}
          aria-pressed={selected}
          onClick={() => selectSecondarySpace(space)}
          style={{ marginLeft: `${nestingLevel * 18}px` }}
        >
          <span className="block text-sm font-medium">{relativeLabel}</span>
          {space.description ? <span className="mt-0.5 block text-[12px] text-[var(--color-on-surface-variant)]">{space.description}</span> : null}
        </button>
        {node.children.length ? (
          <div className="space-y-1">
            {node.children.map((child) => renderSecondarySpaceTreeNode(child, nestingLevel + 1))}
          </div>
        ) : null}
      </div>
    );
  }

  function renderSecondarySpaceMenuContent() {
    return (
      <>
        <p className="font-medium">{t("shell:workspace.selectorTitle")}</p>
        <p className="mt-1 text-[12px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.selectorDescription")}</p>
        <div className="mt-3">
          {workspaceSwitcherLoading ? <p className="mb-3 text-[12px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.refreshing")}</p> : null}
          {secondarySpaceTree.length ? (
            <div className="space-y-2" role="tree" aria-label={t("shell:workspace.secondaryLabel")}>
              {secondarySpaceTree.map((root) => renderSecondarySpaceTreeNode(root))}
            </div>
          ) : (
            <p className="text-[13px] text-[var(--color-on-surface-variant)]">
              {selectedPrimarySpace ? t("shell:workspace.rootOnlyHint") : t("shell:workspace.noSecondarySpaces")}
            </p>
          )}
        </div>
      </>
    );
  }

  function renderSidebarContent(mobile = false) {
    const compact = mobile ? false : sidebarCollapsed;

    return (
      <>
        <div className={cn("mb-6 flex items-center gap-3 px-2", compact ? "justify-center" : "")}>
          <button
            className={cn("flex w-full items-center gap-3 rounded-lg text-left", compact ? "justify-center" : "")}
            type="button"
            aria-expanded={secondarySpaceMenuOpen}
            aria-haspopup="dialog"
            aria-label={secondarySpaceMenuOpen ? t("shell:aria.closeWorkspaceSelector") : t("shell:aria.openWorkspaceSelector")}
            onClick={handleSecondarySpaceSelectorClick}
          >
            <span className="flex size-9 items-center justify-center rounded-xl bg-[var(--color-primary)] text-[var(--color-on-primary)]">
              <Command size={18} aria-hidden="true" />
            </span>
            <span className={cn(compact ? "sr-only" : "")}>
              <span className="block text-sm font-semibold">{selectedSecondarySpaceLabel}</span>
              <span className="block text-xs text-[var(--color-on-surface-variant)]">
                {selectedSecondarySpace?.description ?? (selectedPrimarySpace ? t("shell:workspace.rootOnlyHint") : t("common:brand.console"))}
              </span>
            </span>
            {compact ? null : <ChevronDown className={cn("ml-auto transition-transform", secondarySpaceMenuOpen ? "rotate-180" : "")} size={14} aria-hidden="true" />}
          </button>
        </div>

        <nav className={cn("space-y-4", compact ? "space-y-2" : "")} aria-label={t("shell:aria.primaryNavigation")}>
          {tronNavigation.map((section) => {
            const open = openSections[section.label];
            return (
              <section key={section.label}>
                {compact ? null : (
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
                {open || compact ? (
                  <div className={cn("space-y-1", compact ? "" : "mt-1")}>
                    {section.items.map((item) => {
                      const Icon = item.icon;
                      const active = item.label === activeItem;
                      const itemLabel = t(item.translationKey);
                      return (
                        <Link
                          className={cn(
                            "flex h-9 items-center rounded-lg text-sm transition-colors",
                            compact ? "justify-center px-0" : "gap-2 px-3",
                            active
                              ? "bg-[var(--color-surface-container-high)] font-medium text-[var(--color-on-surface)]"
                              : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                          )}
                          aria-current={active ? "page" : undefined}
                          data-active={active}
                          key={item.label}
                          onClick={mobile ? handleSidebarNavigationClick : undefined}
                          title={compact ? itemLabel : undefined}
                          to={item.href}
                        >
                          <Icon size={17} aria-hidden="true" />
                          <span className={cn(compact ? "sr-only" : "")}>{itemLabel}</span>
                        </Link>
                      );
                    })}
                  </div>
                ) : null}
              </section>
            );
          })}
        </nav>
      </>
    );
  }

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
            <Button
              className="lg:hidden"
              size="icon"
              type="button"
              variant="ghost"
              aria-controls="tron-mobile-sidebar"
              aria-expanded={mobileSidebarOpen}
              aria-label={mobileSidebarOpen ? t("shell:aria.closeNavigation") : t("shell:aria.openNavigation")}
              onClick={toggleMobileSidebar}
            >
              {mobileSidebarOpen ? <PanelLeftClose size={17} aria-hidden="true" /> : <PanelLeftOpen size={17} aria-hidden="true" />}
            </Button>
            <button
              className="inline-flex h-9 shrink-0 items-center gap-2 rounded-lg bg-[var(--color-surface-container)] px-3 text-sm font-medium text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              type="button"
              aria-busy={workspaceSwitcherLoading}
              aria-expanded={utilityPanel === "workspace"}
              onClick={handleWorkspaceSwitcherClick}
            >
              <ShieldCheck size={16} aria-hidden="true" />
              {selectedPrimarySpace?.name ?? t("shell:utilityPanel.workspace")}
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
                {workspaceSwitcherLoading ? <p className="text-[12px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.refreshing")}</p> : null}
                {primarySpaces.length ? (
                  primarySpaces.map((space) => {
                    const selected = space.workspaceCode === effectivePrimarySpaceId;
                    return (
                      <button
                        className={cn(
                          "block w-full rounded-lg border px-3 py-2 text-left transition-colors",
                          selected ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]" : "border-transparent hover:bg-[var(--color-hover-surface)]",
                        )}
                        type="button"
                        aria-pressed={selected}
                        key={space.workspaceCode}
                        onClick={() => selectPrimarySpace(space.workspaceCode)}
                      >
                        <span className="block text-sm font-medium">{space.name}</span>
                        <span className="mt-0.5 block text-[12px] text-[var(--color-on-surface-variant)]">{space.description}</span>
                        <span className="mt-1 block text-[11px] text-[var(--color-on-surface-variant)]">{space.childCount} child spaces · {t("shell:workspace.status", { status: space.status })}</span>
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

      {mobileSidebarOpen ? (
        <>
          <button
            className="fixed inset-x-0 bottom-0 top-14 z-30 bg-[rgba(15,23,42,0.24)] lg:hidden"
            type="button"
            aria-label={t("common:actions.close")}
            onClick={closeMobileSidebar}
          />
          <aside
            className="fixed bottom-0 left-0 top-14 z-40 flex w-[min(20rem,calc(100vw-1rem))] flex-col overflow-y-auto border-r border-[var(--color-outline-variant)] bg-white px-3 py-4 shadow-[0_18px_45px_rgba(15,23,42,0.18)] lg:hidden"
            id="tron-mobile-sidebar"
          >
            {renderSidebarContent(true)}
          </aside>
        </>
      ) : null}

      <aside
        className={cn(
          "fixed bottom-0 left-0 top-14 z-40 hidden flex-col overflow-y-auto border-r border-[var(--color-outline-variant)] bg-white py-4 transition-[width] duration-200 lg:flex",
          sidebarCollapsed ? "w-[76px] px-2" : "w-[280px] px-3",
        )}
        data-collapsed={sidebarCollapsed}
      >
        {renderSidebarContent()}
      </aside>

      {secondarySpaceMenuOpen ? (
        <>
          <div
            className={cn(
              "fixed inset-x-4 top-[4.75rem] z-50 max-h-[calc(100vh-6rem)] overflow-y-auto rounded-xl border border-[var(--color-card-border)] bg-white px-4 py-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)] lg:inset-x-auto lg:top-[4.5rem] lg:w-[min(380px,calc(100vw-2rem))]",
              sidebarCollapsed ? "lg:left-[84px]" : "lg:left-[292px]",
            )}
            id="tron-secondary-space-selector"
            role="dialog"
            aria-label={t("shell:workspace.selectorTitle")}
          >
            {renderSecondarySpaceMenuContent()}
          </div>
        </>
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

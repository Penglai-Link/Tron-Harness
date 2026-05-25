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
  PanelLeftClose,
  PanelLeftOpen,
  Settings2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Link,
  useLocation,
  useNavigate,
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
import { useTheme } from "@/shared/theme/ThemeProvider";
import { SelectInput } from "@/shared/ui/Forms";
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
export interface ConsoleShellConfig {
  contentClassName?: string;
  rightPanel?: ReactNode;
  searchPlaceholder?: string;
  topbarActions?: ReactNode;
}

interface ConsoleShellConfigController {
  resetConfig: () => void;
  setConfig: (config: ConsoleShellConfig) => void;
}

export const ConsoleShellConfigContext = createContext<ConsoleShellConfigController | null>(null);

export function useConsoleShellConfig(config: ConsoleShellConfig) {
  const shellConfigController = useContext(ConsoleShellConfigContext);

  useEffect(() => {
    if (!shellConfigController) {
      return;
    }

    shellConfigController.setConfig(config);
    return () => shellConfigController.resetConfig();
  }, [
    config.contentClassName,
    config.rightPanel,
    config.searchPlaceholder,
    config.topbarActions,
    shellConfigController,
  ]);
}

interface WorkspaceTreeNode {
  children: WorkspaceTreeNode[];
  space: WorkspaceSwitcherData["secondarySpaces"][number];
}

function sectionHasActiveItem(section: TronNavSection, activeItem?: string) {
  return Boolean(activeItem && section.items.some((item) => item.label === activeItem));
}

function buildWorkspaceTree(secondarySpaces: WorkspaceSwitcherData["secondarySpaces"]) {
  const roots: WorkspaceTreeNode[] = [];
  const stack: WorkspaceTreeNode[] = [];

  secondarySpaces.forEach((space) => {
    const node: WorkspaceTreeNode = {
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

function findWorkspaceNodePath(nodes: WorkspaceTreeNode[], targetWorkspaceCode?: string): WorkspaceTreeNode[] {
  if (!targetWorkspaceCode) {
    return [];
  }

  for (const node of nodes) {
    if (node.space.workspaceCode === targetWorkspaceCode) {
      return [node];
    }

    const childPath = findWorkspaceNodePath(node.children, targetWorkspaceCode);
    if (childPath.length) {
      return [node, ...childPath];
    }
  }

  return [];
}

function compactWorkspacePath(label: string) {
  return label.replace(/\s*\/\s*/g, "/");
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
  const { setTheme, theme, themes } = useTheme();
  const { pathname, search } = useLocation();
  const navigate = useNavigate();
  const shellFrameMounted = useContext(TronShellFrameContext);
  const shellConfigController = useContext(ConsoleShellConfigContext);
  const agentOpen = useAppShellStore((state) => state.agentOpen);
  const closeFloatingPanels = useAppShellStore((state) => state.closeFloatingPanels);
  const ensureSectionOpen = useAppShellStore((state) => state.ensureSectionOpen);
  const openSections = useAppShellStore((state) => state.openSections);
  const selectedPrimarySpaceId = useAppShellStore((state) => state.selectedPrimarySpaceId);
  const selectedSecondarySpaceId = useAppShellStore((state) => state.selectedSecondarySpaceId);
  const setAgentOpen = useAppShellStore((state) => state.setAgentOpen);
  const setSidebarCollapsed = useAppShellStore((state) => state.setSidebarCollapsed);
  const sidebarCollapsed = useAppShellStore((state) => state.sidebarCollapsed);
  const toggleSection = useAppShellStore((state) => state.toggleSection);
  const toggleUtilityPanel = useAppShellStore((state) => state.toggleUtilityPanel);
  const utilityPanel = useAppShellStore((state) => state.utilityPanel);
  const selectPrimarySpaceInStore = useAppShellStore((state) => state.selectPrimarySpace);
  const syncSelectedWorkspace = useAppShellStore((state) => state.syncSelectedWorkspace);
  const setWorkspaceSwitcherSnapshot = useAppShellStore((state) => state.setWorkspaceSwitcherSnapshot);
  const workspaceSwitcher = useAppShellStore((state) => state.workspaceSwitcherSnapshot);
  const dataSource = getServiceDataSource("admin");
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [workspaceSwitcherLoading, setWorkspaceSwitcherLoading] = useState(false);
  const workspaceSwitcherControllerRef = useRef<AbortController | null>(null);
  const workspaceSwitcherRequestSequenceRef = useRef(0);
  const [expandedWorkspaceCodes, setExpandedWorkspaceCodes] = useState<string[]>([]);

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
  const selectedSecondarySpace = selectedSecondarySpaceId
    ? secondarySpaces.find((space) => space.workspaceCode === selectedSecondarySpaceId)
      ?? secondarySpaces.find((space) => space.workspaceCode === defaultSecondarySpaceId)
      ?? secondarySpaces[0]
    : undefined;
  const effectiveSecondarySpaceId = selectedSecondarySpace?.workspaceCode ?? "";
  const selectedWorkspaceNode = selectedSecondarySpace ?? currentPrimaryRootSpace;
  const selectedWorkspacePathLabel = selectedWorkspaceNode
    ? compactWorkspacePath(selectedWorkspaceNode.label)
    : selectedPrimarySpace?.name ?? t("shell:utilityPanel.workspace");
  const workspaceTree = useMemo(
    () => buildWorkspaceTree(workspaceSwitcher.secondarySpaces),
    [workspaceSwitcher.secondarySpaces],
  );
  const defaultExpandedWorkspaceCodes = useMemo(
    () => findWorkspaceNodePath(workspaceTree, selectedWorkspaceNode?.workspaceCode)
      .filter((node) => node.children.length > 0)
      .map((node) => node.space.workspaceCode),
    [selectedWorkspaceNode?.workspaceCode, workspaceTree],
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

  const refreshWorkspaceSwitcher = useCallback(async () => {
    if (dataSource !== "api") {
      workspaceSwitcherControllerRef.current?.abort();
      workspaceSwitcherControllerRef.current = null;
      workspaceSwitcherRequestSequenceRef.current += 1;
      setWorkspaceSwitcherSnapshot(getWorkspaceSwitcherMock());
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
      setWorkspaceSwitcherSnapshot(data);
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
  }, [dataSource, setWorkspaceSwitcherSnapshot]);

  useEffect(() => {
    if (dataSource !== "api") {
      setWorkspaceSwitcherSnapshot(getWorkspaceSwitcherMock());
      return () => workspaceSwitcherControllerRef.current?.abort();
    }

    return () => workspaceSwitcherControllerRef.current?.abort();
  }, [dataSource, setWorkspaceSwitcherSnapshot]);

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

    syncSelectedWorkspace(effectivePrimarySpaceId, effectiveSecondarySpaceId);
  }, [
    effectivePrimarySpaceId,
    effectiveSecondarySpaceId,
    selectedPrimarySpaceId,
    selectedSecondarySpaceId,
    syncSelectedWorkspace,
  ]);

  useEffect(() => {
    setExpandedWorkspaceCodes(defaultExpandedWorkspaceCodes);
  }, [defaultExpandedWorkspaceCodes]);

  useEffect(() => {
    if (!shellFrameMounted || !shellConfigController) {
      return;
    }

    shellConfigController.setConfig({
      contentClassName,
      rightPanel,
      searchPlaceholder,
      topbarActions,
    });

    return () => shellConfigController.resetConfig();
  }, [
    contentClassName,
    rightPanel,
    searchPlaceholder,
    shellConfigController,
    shellFrameMounted,
    topbarActions,
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
    setAgentOpen(false);
    if (utilityPanel !== "workspace") {
      void refreshWorkspaceSwitcher();
    }
    toggleUtilityPanel("workspace");
  }

  function handleUtilityPanelClick(panel: string) {
    setAgentOpen(false);
    toggleUtilityPanel(panel);
  }

  function handleAgentLauncherClick() {
    if (!agentOpen) {
      closeFloatingPanels();
    }

    setAgentOpen(!agentOpen);
  }

  function selectWorkspaceNode(space: WorkspaceSwitcherData["secondarySpaces"][number]) {
    const nextSecondarySpaceId = space.depth > 1 ? space.workspaceCode : "";
    selectPrimarySpaceInStore(space.primaryWorkspaceCode, nextSecondarySpaceId);
  }

  function toggleWorkspaceExpanded(workspaceCode: string) {
    setExpandedWorkspaceCodes((current) => (
      current.includes(workspaceCode)
        ? current.filter((code) => code !== workspaceCode)
        : [...current, workspaceCode]
    ));
  }

  function renderWorkspaceTreeNode(node: WorkspaceTreeNode, nestingLevel = 0): ReactNode {
    const { space } = node;
    const workspacePathLabel = compactWorkspacePath(space.label);
    const hasChildren = node.children.length > 0;
    const expanded = hasChildren && expandedWorkspaceCodes.includes(space.workspaceCode);
    const selected = space.workspaceCode === selectedWorkspaceNode?.workspaceCode;
    const rowClassName = cn(
      "flex min-w-0 flex-1 items-center rounded-lg border px-2.5 py-1.5 text-left transition-colors",
      selected
        ? "border-[var(--color-primary)] bg-[var(--color-surface-container)]"
        : "border-transparent hover:bg-[var(--color-hover-surface)]",
    );
    const content = (
      <>
        <span className="truncate text-sm font-medium">{space.name}</span>
      </>
    );

    return (
      <div key={space.workspaceCode} className="space-y-0.5">
        <div className="flex items-center gap-1" style={{ paddingLeft: `${nestingLevel * 12}px` }}>
          {hasChildren ? (
            <button
              className="flex size-6 shrink-0 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
              type="button"
              aria-label={`${expanded ? "Collapse" : "Expand"} ${workspacePathLabel}`}
              aria-expanded={expanded}
              onClick={() => toggleWorkspaceExpanded(space.workspaceCode)}
            >
              <ChevronDown className={cn("transition-transform", expanded ? "rotate-180" : "-rotate-90")} size={14} aria-hidden="true" />
            </button>
          ) : (
            <span className="block size-6 shrink-0" aria-hidden="true" />
          )}
          <button
            className={rowClassName}
            type="button"
            aria-label={workspacePathLabel}
            aria-pressed={selected}
            onClick={() => selectWorkspaceNode(space)}
          >
            {content}
          </button>
        </div>
        {hasChildren && expanded ? (
          <div className="space-y-0.5">
            {node.children.map((child) => renderWorkspaceTreeNode(child, nestingLevel + 1))}
          </div>
        ) : null}
      </div>
    );
  }

  function renderWorkspaceSwitcherContent() {
    return (
      <>
        <p className="font-medium">{t("shell:workspace.selectorTitle")}</p>
        <div className="mt-2">
          {workspaceSwitcherLoading ? <p className="mb-2 text-[12px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.refreshing")}</p> : null}
          {workspaceTree.length ? (
            <div className="max-h-[min(60vh,24rem)] space-y-1 overflow-y-auto pr-1" role="tree" aria-label={t("shell:workspace.selectorTitle")}>
              {workspaceTree.map((root) => renderWorkspaceTreeNode(root))}
            </div>
          ) : (
            <p className="text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:workspace.noPrimarySpaces")}</p>
          )}
        </div>
      </>
    );
  }

  function renderSidebarContent(mobile = false) {
    const compact = mobile ? false : sidebarCollapsed;

    return (
      <>
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
    return <>{children}</>;
  }

  return (
    <TronShellFrameContext.Provider value={true}>
    <WorkspaceSpacesContext.Provider value={workspaceSpacesValue}>
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
      <header className="fixed inset-x-0 top-0 z-50 h-14 bg-[var(--color-header-backdrop)] px-4 shadow-[0_10px_28px_rgba(15,23,42,0.045)] backdrop-blur sm:px-6">
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
              className="inline-flex h-9 max-w-[min(60vw,28rem)] shrink-0 items-center gap-2 rounded-lg bg-[var(--color-surface-container)] px-3 text-sm font-medium text-[var(--color-on-surface)] hover:bg-[var(--color-hover-surface)]"
              type="button"
              id="tron-workspace-trigger"
              aria-busy={workspaceSwitcherLoading}
              aria-expanded={utilityPanel === "workspace"}
              aria-haspopup="dialog"
              aria-label={utilityPanel === "workspace" ? t("shell:aria.closeWorkspaceSelector") : t("shell:aria.openWorkspaceSelector")}
              onClick={handleWorkspaceSwitcherClick}
              title={selectedWorkspacePathLabel}
            >
              <span className="truncate">{selectedWorkspacePathLabel}</span>
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
            <Button
              size="icon"
              variant="ghost"
              aria-label={t("shell:aria.openSystemSettings")}
              onClick={() => {
                closeFloatingPanels();
                navigate("/settings", { state: { from: `${pathname}${search}` } });
              }}
            >
              <Settings2 size={17} aria-hidden="true" />
            </Button>
            <Button size="icon" variant="ghost" aria-label={t("shell:aria.viewNotifications")} aria-pressed={utilityPanel === "notifications"} onClick={() => handleUtilityPanelClick("notifications")}>
              <Bell size={17} aria-hidden="true" />
            </Button>
            <Button className="rounded-full hover:bg-transparent" size="icon" variant="ghost" aria-label={t("shell:aria.openUserMenu")} aria-pressed={utilityPanel === "user"} onClick={() => handleUtilityPanelClick("user")}>
              <CircleUserRound size={18} aria-hidden="true" />
            </Button>
            <Button className={cn("rounded-full", agentOpen ? "bg-[var(--color-surface-container)] text-[var(--color-on-surface)]" : "")} size="icon" type="button" variant="ghost" aria-label={t("shell:assistant.title")} aria-expanded={agentOpen} aria-pressed={agentOpen} title={t("shell:assistant.title")} onClick={handleAgentLauncherClick}>
              <Bot size={17} aria-hidden="true" />
            </Button>
          </div>
        </div>
        {utilityPanel ? (
          <div
            id={utilityPanel === "workspace" ? "tron-workspace-selector-panel" : undefined}
            className={cn(
              "absolute top-[calc(100%+8px)] rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2.5 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
              utilityPanel === "workspace"
                ? "left-4 w-[min(360px,calc(100vw-2rem))] sm:left-6"
                : "right-4 w-[min(360px,calc(100vw-2rem))] sm:right-6",
            )}
            role={utilityPanel === "workspace" ? "dialog" : undefined}
            aria-label={utilityPanel === "workspace" ? t("shell:workspace.selectorTitle") : undefined}
          >
            {utilityPanel === "workspace" ? null : <p className="font-medium">{t(`shell:utilityPanel.${utilityPanel}`)}</p>}
            {utilityPanel === "workspace" ? (
              <div>
                {renderWorkspaceSwitcherContent()}
              </div>
            ) : utilityPanel === "user" ? (
              <div className="mt-3 space-y-4">
                {userMenuContent}
                <div className={cn("space-y-4", userMenuContent ? "border-t border-[var(--color-outline-variant)] pt-4" : "")}>
                  <div>
                    <label className="mb-1 block text-[12px] font-medium text-[var(--color-on-surface-variant)]" htmlFor="tron-theme-select">
                      {t("shell:theme.label")}
                    </label>
                    <SelectInput
                      id="tron-theme-select"
                      value={theme}
                      onChange={(event) => setTheme(event.target.value === "matrix-dark" ? "matrix-dark" : "light")}
                    >
                      {themes.map((themeOption) => (
                        <option key={themeOption.value} value={themeOption.value}>
                          {t(themeOption.labelKey)}
                        </option>
                      ))}
                    </SelectInput>
                    <p className="mt-2 text-[12px] text-[var(--color-on-surface-variant)]">{t("shell:theme.hint")}</p>
                  </div>
                  <div className="border-t border-[var(--color-outline-variant)] pt-4">
                    <label className="mb-1 block text-[12px] font-medium text-[var(--color-on-surface-variant)]" htmlFor="tron-locale-select">
                      {t("common:locale.label")}
                    </label>
                    <SelectInput
                      id="tron-locale-select"
                      value={currentLocale}
                      onChange={(event) => void changeLocale(event.target.value as typeof currentLocale)}
                    >
                      {locales.map((locale) => (
                        <option key={locale.code} value={locale.code}>
                          {t(locale.labelKey)}
                        </option>
                      ))}
                    </SelectInput>
                    <p className="mt-2 text-[12px] text-[var(--color-on-surface-variant)]">{t("common:locale.unavailable")}</p>
                  </div>
                </div>
              </div>
            ) : (
              <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:utilityPanel.sharedState")}</p>
            )}
          </div>
        ) : null}
        {agentOpen ? (
          <section className="absolute right-4 top-[calc(100%+8px)] z-40 w-[min(320px,calc(100vw-2rem))] rounded-2xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-4 text-sm shadow-[0_18px_50px_rgba(0,0,0,0.14)] sm:right-6">
            <p className="font-semibold">{t("shell:assistant.title")}</p>
            <p className="mt-2 text-[13px] leading-5 text-[var(--color-on-surface-variant)]">{t("shell:assistant.body")}</p>
            <button className="mt-3 h-8 rounded-lg bg-[var(--color-surface-container)] px-3 text-[13px] font-medium hover:bg-[var(--color-hover-surface)]" type="button" onClick={() => setAgentOpen(false)}>
              {t("common:actions.close")}
            </button>
          </section>
        ) : null}
      </header>

      {mobileSidebarOpen ? (
        <>
          <button
            className="fixed inset-x-0 bottom-0 top-14 z-30 bg-[var(--color-backdrop-scrim)] lg:hidden"
            type="button"
            aria-label={t("common:actions.close")}
            onClick={closeMobileSidebar}
          />
          <aside
            className="fixed bottom-0 left-0 top-14 z-40 flex w-[min(20rem,calc(100vw-1rem))] flex-col overflow-y-auto bg-[var(--color-elevated-surface)] px-3 py-4 shadow-[12px_0_34px_rgba(15,23,42,0.12),0_18px_45px_rgba(15,23,42,0.12)] lg:hidden"
            id="tron-mobile-sidebar"
          >
            {renderSidebarContent(true)}
          </aside>
        </>
      ) : null}

      <aside
        className={cn(
          "fixed bottom-0 left-0 top-14 z-40 hidden flex-col overflow-y-auto bg-[var(--color-elevated-surface)] py-4 shadow-[10px_0_28px_rgba(15,23,42,0.04)] transition-[width] duration-200 lg:flex",
          sidebarCollapsed ? "w-[76px] px-2" : "w-[280px] px-3",
        )}
        data-collapsed={sidebarCollapsed}
      >
        {renderSidebarContent()}
      </aside>

      <section className={cn("min-w-0 pt-14 transition-[padding-left] duration-200", sidebarCollapsed ? "lg:pl-[76px]" : "lg:pl-[280px]")}>
        <div className={cn("grid min-h-[calc(100vh-3.5rem)] grid-cols-1", rightPanel ? "xl:grid-cols-[minmax(0,1fr)_360px]" : "")}>
          <main className={cn("min-w-0 px-6 py-6", contentClassName)}>{children}</main>
          {rightPanel ? <aside className="border-l border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-5 py-6">{rightPanel}</aside> : null}
        </div>
      </section>

    </div>
    </WorkspaceSpacesContext.Provider>
    </TronShellFrameContext.Provider>
  );
}

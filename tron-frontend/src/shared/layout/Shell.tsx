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
  defaultFloatingState,
  readFloatingState,
  writeFloatingState,
  type ChatFloatingState,
} from "@tron/chat-core";
import {
  Bell,
  Bot,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleUserRound,
  MoreHorizontal,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
  Settings2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Link,
  useLocation,
  useNavigate,
} from "react-router";

import { fetchWorkspaceSwitcher } from "@/domain/admin/api";
import { useAuthSession } from "@/domain/auth";
import { MainAgentChatWindow } from "@/domain/agent/components/MainAgentChatWindow";
import { isUnauthorizedApiError } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { tronNavigation } from "@/shared/constants/shellNavigation";
import {
  getEmptyWorkspaceSwitcher,
  getWorkspaceSwitcherMock,
} from "@/shared/constants/workspaceSpaces";
import { WorkspaceSpacesContext } from "@/shared/hooks/useWorkspaceSpaces";
import { Button } from "@/shared/ui/Button";
import { useLocale } from "@/i18n";
import { useAppShellStore } from "@/store/appShellStore";
import {
  buildWorkspaceTree,
  findWorkspaceNodePath,
  type WorkspaceTreeNode,
} from "@/shared/lib/workspaceTree";
import { cn } from "@/shared/lib/utils";
import { useTheme } from "@/shared/theme/ThemeProvider";
import { SelectInput } from "@/shared/ui/Forms";
import { useDismissibleLayer } from "@/shared/hooks/useDismissibleLayer";
import type { TronNavSection } from "@/shared/types/ShellNavigation";
import type {
  WorkspaceSpacesContextValue,
  WorkspaceSwitcherData,
} from "@/shared/types/WorkspaceSpace";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { usePanel } from "../hooks/usePanel";


interface TronShellProps {
  activeItem?: string;
  children: ReactNode;
  contentClassName?: string;
  rightPanel?: ReactNode;
  roleCodes?: string[];
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

type WorkspaceSwitcherTreeNode = WorkspaceTreeNode<WorkspaceSwitcherData["secondarySpaces"][number]>;

const TRON_DESKTOP_SIDEBAR_EXPANDED_WIDTH_CLASS = "w-[240px]";
const TRON_DESKTOP_SIDEBAR_EXPANDED_PADDING_CLASS = "lg:pl-[240px]";
const MAIN_AGENT_WINDOW_STORAGE_KEY = "tron.main-agent.window";

export const ConsoleShellConfigContext = createContext<ConsoleShellConfigController | null>(null);

export function useConsoleShellConfig(config: ConsoleShellConfig) {
  const shellConfigController = useContext(ConsoleShellConfigContext);

  useEffect(() => {
    if (!shellConfigController) {
      return;
    }

    shellConfigController.setConfig(config);
  }, [
    config.contentClassName,
    config.rightPanel,
    config.searchPlaceholder,
    config.topbarActions,
    shellConfigController,
  ]);

  useEffect(() => {
    if (!shellConfigController) {
      return;
    }

    return () => shellConfigController.resetConfig();
  }, [shellConfigController]);
}

export function useOpenRightSidebar() {
  const setRightSidebarCollapsed = useAppShellStore((state) => state.setRightSidebarCollapsed);

  return useCallback(() => {
    setRightSidebarCollapsed(false);
  }, [setRightSidebarCollapsed]);
}

function sectionHasActiveItem(section: TronNavSection, activeItem?: string) {
  return Boolean(activeItem && section.items.some((item) => item.label === activeItem));
}

function compactWorkspacePath(label: string) {
  return label.replace(/\s*\/\s*/g, "/");
}

export function TronShell({
  activeItem = "Dashboard",
  children,
  contentClassName,
  rightPanel,
  roleCodes,
  searchPlaceholder,
  topbarActions,
  userMenuContent,
}: TronShellProps) {
  const { t } = useTranslation(["common", "shell"]);
  const { changeLocale, currentLocale, supportedLocales: locales } = useLocale();
  const { setTheme, theme, themes } = useTheme();
  const { session: authSession } = useAuthSession();
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
  const rightSidebarCollapsed = useAppShellStore((state) => state.rightSidebarCollapsed);
  const setAgentOpen = useAppShellStore((state) => state.setAgentOpen);
  const setRightSidebarCollapsed = useAppShellStore((state) => state.setRightSidebarCollapsed);
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
  const [rightDrawerOpen, setRightDrawerOpen] = useState(false);
  const [sidebarTooltip, setSidebarTooltip] = useState<{ label: string; top: number } | null>(null);
  const [topbarToolsMenuOpen, setTopbarToolsMenuOpen] = useState(false);
  const [workspaceSwitcherLoading, setWorkspaceSwitcherLoading] = useState(false);
  const [mainAgentWindowState, setMainAgentWindowState] = useState<ChatFloatingState>(() => ({
    ...readFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, defaultFloatingState),
    open: false,
  }));
  const workspaceSwitcherControllerRef = useRef<AbortController | null>(null);
  const workspaceSwitcherRequestSequenceRef = useRef(0);
  const utilityPanelRef = useRef<HTMLDivElement | null>(null);
  const workspaceTriggerRef = useRef<HTMLButtonElement | null>(null);
  const notificationsTriggerRef = useRef<HTMLDivElement | null>(null);
  const userMenuTriggerRef = useRef<HTMLDivElement | null>(null);
  const topbarToolsMenuRef = useRef<HTMLDivElement | null>(null);
  const topbarToolsMenuContainerRef = useRef<HTMLDivElement | null>(null);
  const [expandedWorkspaceCodes, setExpandedWorkspaceCodes] = useState<string[]>([]);

  const { panelRef, panelResize } = usePanel()

  const visibleNavigation = useMemo(
    () => tronNavigation.filter((section) => {
      if (!section.requiredRoleCodes?.length) {
        return true;
      }

      if (!roleCodes) {
        return true;
      }

      return section.requiredRoleCodes.some((roleCode) => roleCodes.includes(roleCode));
    }),
    [roleCodes],
  );

  const primarySpaces = workspaceSwitcher.primarySpaces;
  const selectedPrimarySpace = primarySpaces.find((space) => space.workspaceCode === selectedPrimarySpaceId)
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
    : undefined;
  const effectiveSecondarySpaceId = selectedSecondarySpace?.workspaceCode ?? "";
  const selectedWorkspaceNode = selectedSecondarySpace ?? currentPrimaryRootSpace;
  const selectedWorkspacePathLabel = selectedWorkspaceNode
    ? compactWorkspacePath(selectedWorkspaceNode.label)
    : selectedPrimarySpace?.name ?? t("shell:utilityPanel.workspace");
  const activeWorkspaceCode = effectiveSecondarySpaceId || effectivePrimarySpaceId;
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
  const hasRightPanel = Boolean(rightPanel);
  const showDesktopRightSidebar = hasRightPanel && !rightSidebarCollapsed;

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
    void refreshWorkspaceSwitcher();
    return () => workspaceSwitcherControllerRef.current?.abort();
  }, [refreshWorkspaceSwitcher]);

  useEffect(() => {
    const activeSection = visibleNavigation.find((section) => sectionHasActiveItem(section, activeItem));
    ensureSectionOpen(activeSection?.label);
  }, [activeItem, ensureSectionOpen, visibleNavigation]);

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
    writeFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, mainAgentWindowState);
  }, [mainAgentWindowState]);

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
  }, [
    contentClassName,
    rightPanel,
    searchPlaceholder,
    shellConfigController,
    shellFrameMounted,
    topbarActions,
  ]);

  useEffect(() => {
    if (!shellFrameMounted || !shellConfigController) {
      return;
    }

    return () => shellConfigController.resetConfig();
  }, [shellConfigController, shellFrameMounted]);

  useEffect(() => {
    setMobileSidebarOpen(false);
    setRightDrawerOpen(false);
    setSidebarTooltip(null);
    setTopbarToolsMenuOpen(false);
  }, [pathname]);

  useEffect(() => {
    setSidebarTooltip(null);
  }, [sidebarCollapsed]);

  useEffect(() => {
    setRightSidebarCollapsed(true);
  }, [pathname, setRightSidebarCollapsed]);

  useEffect(() => {
    if (!hasRightPanel) {
      setRightDrawerOpen(false);
    }
  }, [hasRightPanel]);

  useEffect(() => {
    if (!mobileSidebarOpen && !rightDrawerOpen) {
      return;
    }

    const previousOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMobileSidebarOpen(false);
        setRightDrawerOpen(false);
      }
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [mobileSidebarOpen, rightDrawerOpen]);

  useDismissibleLayer({
    open: Boolean(utilityPanel),
    onDismiss: () => {
      if (!utilityPanel) return;
      toggleUtilityPanel(utilityPanel);
    },
    popupRef: utilityPanelRef,
    triggerElement:
      utilityPanel === "workspace"
        ? workspaceTriggerRef.current
        : utilityPanel === "notifications"
          ? notificationsTriggerRef.current
          : utilityPanel === "user"
            ? userMenuTriggerRef.current
            : null,
  });

  useDismissibleLayer({
    open: topbarToolsMenuOpen,
    onDismiss: () => setTopbarToolsMenuOpen(false),
    popupRef: topbarToolsMenuRef,
    triggerElement: topbarToolsMenuContainerRef.current,
  });

  function toggleMobileSidebar() {
    if (!mobileSidebarOpen) {
      closeFloatingPanels();
      setRightDrawerOpen(false);
    }

    setMobileSidebarOpen((open) => !open);
  }

  function closeMobileSidebar() {
    closeFloatingPanels();
    setMobileSidebarOpen(false);
  }

  function openRightDrawer() {
    closeFloatingPanels();
    setTopbarToolsMenuOpen(false);
    setMobileSidebarOpen(false);
    setRightDrawerOpen(true);
  }

  function closeRightDrawer() {
    setRightDrawerOpen(false);
  }

  function handleSidebarNavigationClick() {
    closeFloatingPanels();
    closeMobileSidebar();
  }

  function showCollapsedNavTooltip(label: string, element: HTMLElement) {
    const rect = element.getBoundingClientRect();
    setSidebarTooltip({ label, top: rect.top + rect.height / 2 });
  }

  function hideCollapsedNavTooltip() {
    setSidebarTooltip(null);
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
    setTopbarToolsMenuOpen(false);
    toggleUtilityPanel(panel);
  }

  function handleAgentLauncherClick() {
    setTopbarToolsMenuOpen(false);
    if (!agentOpen) {
      closeFloatingPanels();
    }

    setAgentOpen(!agentOpen);
    setMainAgentWindowState((current) => ({
      ...current,
      open: !agentOpen,
    }));
  }

  function handleMainAgentWindowStateChange(nextState: ChatFloatingState) {
    setMainAgentWindowState(nextState);
    if (agentOpen !== nextState.open) {
      setAgentOpen(nextState.open);
    }
  }

  function openSystemSettings() {
    setTopbarToolsMenuOpen(false);
    closeFloatingPanels();
    navigate("/settings/general", { state: { from: `${pathname}${search}` } });
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

  function renderWorkspaceTreeNode(node: WorkspaceSwitcherTreeNode, nestingLevel = 0): ReactNode {
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
        <div>
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
          {visibleNavigation.map((section) => {
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
                          aria-label={compact ? itemLabel : undefined}
                          aria-current={active ? "page" : undefined}
                          data-active={active}
                          key={item.label}
                          onBlur={compact ? hideCollapsedNavTooltip : undefined}
                          onFocus={(event) => {
                            if (compact) showCollapsedNavTooltip(itemLabel, event.currentTarget);
                          }}
                          onMouseEnter={(event) => {
                            if (compact) showCollapsedNavTooltip(itemLabel, event.currentTarget);
                          }}
                          onMouseLeave={compact ? hideCollapsedNavTooltip : undefined}
                          onClick={mobile ? handleSidebarNavigationClick : undefined}
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
        <div className="isolate min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]">
          <header className="fixed inset-x-0 top-0 z-[60] h-14 bg-[var(--color-header-backdrop)] px-4 shadow-[0_10px_28px_rgba(15,23,42,0.045)] backdrop-blur sm:px-6">
            <div className="flex h-full items-center justify-between gap-4">
              <div className="flex min-w-0 flex-1 items-center gap-3">
                <Button
                  className="-ml-3 lg:hidden"
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
                  ref={workspaceTriggerRef}
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
              </div>
              <div className="flex shrink-0 items-center gap-2">
                {topbarActions}
                <Button
                  className="hidden min-[900px]:inline-flex"
                  size="icon"
                  variant="ghost"
                  aria-label={t("shell:aria.openSystemSettings")}
                  onClick={openSystemSettings}
                >
                  <Settings2 size={17} aria-hidden="true" />
                </Button>
                <div ref={notificationsTriggerRef} className="hidden min-[900px]:contents">
                  <Button className="hidden min-[900px]:inline-flex" size="icon" variant="ghost" aria-label={t("shell:aria.viewNotifications")} aria-pressed={utilityPanel === "notifications"} onClick={() => handleUtilityPanelClick("notifications")}>
                    <Bell size={17} aria-hidden="true" />
                  </Button>
                </div>
                <div ref={userMenuTriggerRef} className="hidden min-[900px]:contents">
                  <Button className="hidden rounded-full hover:bg-transparent min-[900px]:inline-flex" size="icon" variant="ghost" aria-label={t("shell:aria.openUserMenu")} aria-pressed={utilityPanel === "user"} onClick={() => handleUtilityPanelClick("user")}>
                    <CircleUserRound size={18} aria-hidden="true" />
                  </Button>
                </div>
                <Button className={cn("hidden rounded-full min-[900px]:inline-flex", agentOpen ? "bg-[var(--color-surface-container)] text-[var(--color-on-surface)]" : "")} size="icon" type="button" variant="ghost" aria-label={t("shell:assistant.title")} aria-expanded={agentOpen} aria-pressed={agentOpen} title={t("shell:assistant.title")} onClick={handleAgentLauncherClick}>
                  <Bot size={17} aria-hidden="true" />
                </Button>
                <div ref={topbarToolsMenuContainerRef} className="relative min-[900px]:hidden">
                  <Button
                    size="icon"
                    type="button"
                    variant="ghost"
                    aria-expanded={topbarToolsMenuOpen}
                    aria-haspopup="menu"
                    aria-label={t("shell:aria.moreActions")}
                    onClick={() => setTopbarToolsMenuOpen((open) => !open)}
                  >
                    <MoreHorizontal size={18} aria-hidden="true" />
                  </Button>
                  {topbarToolsMenuOpen ? (
                    <div ref={topbarToolsMenuRef} className="absolute right-0 top-[calc(100%+8px)] z-[70] w-48 rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-2 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]" role="menu">
                      <button className="flex h-9 w-full items-center gap-2 rounded-lg px-2 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={openSystemSettings}>
                        <Settings2 size={16} aria-hidden="true" />
                        {t("shell:utilityPanel.settings")}
                      </button>
                      <button className="flex h-9 w-full items-center gap-2 rounded-lg px-2 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={() => handleUtilityPanelClick("notifications")}>
                        <Bell size={16} aria-hidden="true" />
                        {t("shell:utilityPanel.notifications")}
                      </button>
                      <button className="flex h-9 w-full items-center gap-2 rounded-lg px-2 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={() => handleUtilityPanelClick("user")}>
                        <CircleUserRound size={16} aria-hidden="true" />
                        {t("shell:utilityPanel.user")}
                      </button>
                      <button className="flex h-9 w-full items-center gap-2 rounded-lg px-2 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={handleAgentLauncherClick}>
                        <Bot size={16} aria-hidden="true" />
                        {t("shell:assistant.title")}
                      </button>
                    </div>
                  ) : null}
                </div>
                {hasRightPanel ? (
                  <Button
                    className="lg:hidden"
                    size="icon"
                    type="button"
                    variant="ghost"
                    aria-controls="tron-mobile-right-sidebar"
                    aria-expanded={rightDrawerOpen}
                    aria-label={rightDrawerOpen ? t("shell:aria.closeRightSidebar") : t("shell:aria.openRightSidebar")}
                    data-testid="tron-mobile-right-sidebar-toggle"
                    onClick={rightDrawerOpen ? closeRightDrawer : openRightDrawer}
                  >
                    {rightDrawerOpen ? <PanelRightClose size={17} aria-hidden="true" /> : <PanelRightOpen size={17} aria-hidden="true" />}
                  </Button>
                ) : null}
              </div>
            </div>
            {utilityPanel ? (
              <div
                ref={utilityPanelRef}
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
                    <div className={cn("grid grid-cols-2 gap-3", userMenuContent ? "border-t border-[var(--color-outline-variant)] pt-4" : "")}>
                      <div className="min-w-0">
                        <label className="mb-1 block text-[12px] font-medium text-[var(--color-on-surface-variant)]" htmlFor="tron-theme-select">
                          {t("shell:theme.label")}
                        </label>
                        <SelectInput
                          className="w-full"
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
                      </div>
                      <div className="min-w-0">
                        <label className="mb-1 block text-[12px] font-medium text-[var(--color-on-surface-variant)]" htmlFor="tron-locale-select">
                          {t("common:locale.label")}
                        </label>
                        <SelectInput
                          className="w-full"
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
                      </div>
                    </div>
                  </div>
                ) : (
                  <p className="mt-1 text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:utilityPanel.sharedState")}</p>
                )}
              </div>
            ) : null}
          </header>

          {agentOpen ? (
            <MainAgentChatWindow
              currentUserDisplayName={authSession?.user.displayName ?? ""}
              workspaceCode={activeWorkspaceCode}
              windowState={{ ...mainAgentWindowState, open: true }}
              onWindowStateChange={handleMainAgentWindowStateChange}
            />
          ) : null}

          {mobileSidebarOpen ? (
            <>
              <button
                className="fixed inset-x-0 bottom-0 top-14 z-40 bg-[var(--color-backdrop-scrim)] lg:hidden"
                type="button"
                aria-label={t("common:actions.close")}
                onClick={closeMobileSidebar}
              />
              <aside
                className="fixed bottom-0 left-0 top-14 z-50 flex w-[min(20rem,calc(100vw-1rem))] flex-col overflow-y-auto bg-[var(--color-elevated-surface)] px-3 py-4 shadow-[12px_0_34px_rgba(15,23,42,0.12),0_18px_45px_rgba(15,23,42,0.12)] lg:hidden"
                id="tron-mobile-sidebar"
              >
                {renderSidebarContent(true)}
              </aside>
            </>
          ) : null}

          {hasRightPanel && rightDrawerOpen ? (
            <>
              <button
                className="fixed inset-x-0 bottom-0 top-14 z-40 bg-[var(--color-backdrop-scrim)] lg:hidden"
                type="button"
                aria-label={t("shell:aria.closeRightSidebar")}
                data-testid="tron-mobile-right-sidebar-backdrop"
                onClick={closeRightDrawer}
              />
              <aside
                className="fixed bottom-0 right-0 top-14 z-50 flex w-[min(22rem,calc(100vw-1rem))] flex-col overflow-y-auto bg-[var(--color-elevated-surface)] px-5 py-5 shadow-[-12px_0_34px_rgba(15,23,42,0.12),0_18px_45px_rgba(15,23,42,0.12)] lg:hidden"
                data-testid="tron-mobile-right-sidebar"
                id="tron-mobile-right-sidebar"
              >
                <div className="mb-4 flex justify-end">
                  <Button
                    size="icon"
                    type="button"
                    variant="ghost"
                    aria-label={t("shell:aria.closeRightSidebar")}
                    onClick={closeRightDrawer}
                  >
                    <X size={17} aria-hidden="true" />
                  </Button>
                </div>
                {rightPanel}
              </aside>
            </>
          ) : null}

          <aside
            className={cn(
              "fixed bottom-0 left-0 top-14 z-50 hidden flex-col overflow-visible bg-[var(--color-elevated-surface)] shadow-[10px_0_28px_rgba(15,23,42,0.04)] transition-[width] duration-200 lg:flex",
              sidebarCollapsed ? "w-[76px]" : TRON_DESKTOP_SIDEBAR_EXPANDED_WIDTH_CLASS,
            )}
            data-collapsed={sidebarCollapsed}
          >
            <div className={cn("h-full overflow-y-auto py-4", sidebarCollapsed ? "px-2" : "px-3")}>
              {renderSidebarContent()}
            </div>
            <button
              className="absolute right-[-22px] top-1/2 z-10 inline-flex h-11 w-6 -translate-y-1/2 items-center justify-center rounded-l-none rounded-r-full border-0 bg-[var(--color-elevated-surface)] p-0 text-[var(--color-on-surface-variant)] shadow-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)] focus-visible:ring-offset-2"
              type="button"
              aria-label={sidebarCollapsed ? t("shell:aria.expandSidebar") : t("shell:aria.collapseSidebar")}
              aria-pressed={sidebarCollapsed}
              onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            >
              {sidebarCollapsed ? <ChevronRight size={15} strokeWidth={2.4} aria-hidden="true" /> : <ChevronLeft size={15} strokeWidth={2.4} aria-hidden="true" />}
            </button>
          </aside>

          {sidebarTooltip && sidebarCollapsed ? (
            <div
              className="pointer-events-none fixed left-[84px] z-[70] -translate-y-1/2 whitespace-nowrap rounded-md border border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-2 py-1 text-xs font-medium text-[var(--color-on-surface)] opacity-100 shadow-[0_8px_18px_rgba(15,23,42,0.12)]"
              role="tooltip"
              style={{ top: sidebarTooltip.top }}
            >
              {sidebarTooltip.label}
            </div>
          ) : null}

          <section className={cn("relative z-0 min-w-0 pt-14 transition-[padding-left] duration-200", sidebarCollapsed ? "lg:pl-[76px]" : TRON_DESKTOP_SIDEBAR_EXPANDED_PADDING_CLASS)}>
            <div className={cn("relative z-0 grid min-h-[calc(100vh-3.5rem)] grid-cols-1")}>
              <style>{`
              #resize-panel {
                display: none !important;
              }

              @media (min-width: 1024px) {
                #resize-panel {
                  display: flex !important;
                }
              }
            `}</style>
              <ResizablePanelGroup
                orientation="horizontal"
                className="w-full"
              >
                <ResizablePanel defaultSize="100%">
                  <main className={cn("relative z-0 min-w-0 overflow-x-hidden px-6 py-6 h-[calc(100vh-3.5rem)] overflow-y-auto", contentClassName)}>{children}</main>
                </ResizablePanel>
                {
                  hasRightPanel && <>
                    <ResizableHandle withHandle className="hidden lg:flex" />
                    <ResizablePanel panelRef={panelRef} id="resize-panel" collapsible={true} defaultSize="0%" maxSize={700} onResize={panelResize}>
                      <aside
                        className="relative z-10 hidden h-[calc(100vh-3.5rem)] overflow-y-auto bg-[var(--color-elevated-surface)] lg:block"
                        data-testid="tron-right-sidebar"
                      >
                        {rightPanel}
                      </aside>
                    </ResizablePanel>
                  </>
                }

              </ResizablePanelGroup>
            </div>
          </section>
        </div>
      </WorkspaceSpacesContext.Provider>
    </TronShellFrameContext.Provider>
  );
}

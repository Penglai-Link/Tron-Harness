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
  Bot,
  ChevronDown,
  MoreHorizontal,
  PanelBottomClose,
  PanelBottomOpen,
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
import {
  CREATE_CHAT_SESSION_SHORTCUT_EVENT,
  findMatchingShellShortcutAction,
  isEditableShortcutTarget,
} from "@/shared/lib/keyboardShortcuts";
import { cn } from "@/shared/lib/utils";
import { useTheme } from "@/shared/theme/ThemeProvider";
import { SelectInput } from "@/shared/ui/Forms";
import { useDismissibleLayer } from "@/shared/hooks/useDismissibleLayer";
import { useTabStore } from "@/store/tabStore";
import type { TronNavItem, TronNavSection } from "@/shared/types/ShellNavigation";
import type {
  WorkspaceSpacesContextValue,
  WorkspaceSwitcherData,
} from "@/shared/types/WorkspaceSpace";
import DraggleLine from "../ui/DraggleLine";
import { SelectItem } from "@/components/ui/select";
import { Content } from "./content";


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

const TRON_DESKTOP_SIDEBAR_EXPANDED_WIDTH_CLASS = "w-[280px]";
const TRON_DESKTOP_SIDEBAR_EXPANDED_PADDING_CLASS = "lg:pl-[280px]";
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
    window.dispatchEvent(new Event("tron:open-right-sidebar"));
  }, [setRightSidebarCollapsed]);
}

function navigationItemHasActiveItem(item: TronNavItem, activeItem: string): boolean {
  return item.label === activeItem
    || item.children?.some((child) => navigationItemHasActiveItem(child, activeItem))
    || false;
}

function sectionHasActiveItem(section: TronNavSection, activeItem?: string) {
  return Boolean(activeItem && section.items.some((item) => navigationItemHasActiveItem(item, activeItem)));
}

function findActiveNavigationParentKeys(
  items: TronNavItem[],
  activeItem: string,
  parentKey: string,
): string[] {
  for (const item of items) {
    const itemKey = `${parentKey}/${item.label}`;
    if (item.label === activeItem) {
      return [];
    }

    if (item.children?.some((child) => navigationItemHasActiveItem(child, activeItem))) {
      return [itemKey, ...findActiveNavigationParentKeys(item.children, activeItem, itemKey)];
    }
  }

  return [];
}

function compactWorkspacePath(label: string) {
  return label.replace(/\s*\/\s*/g, "/");
}

export function TronShell({
  activeItem = "Apps",
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
  const shortcutBindings = useAppShellStore((state) => state.shortcutBindings);
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
  const [openNavigationItems, setOpenNavigationItems] = useState<Record<string, boolean>>({});
  const [rightDrawerOpen, setRightDrawerOpen] = useState(false);
  const [rightDragglePanelOpen, setRightDragglePanelOpen] = useState(false);

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
  const userMenuTriggerRef = useRef<HTMLButtonElement | null>(null);
  const topbarToolsMenuRef = useRef<HTMLDivElement | null>(null);
  const topbarToolsMenuContainerRef = useRef<HTMLDivElement | null>(null);
  const [expandedWorkspaceCodes, setExpandedWorkspaceCodes] = useState<string[]>([]);
  const [showBottomPanel, setBottomPanel] = useState(false)

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

    if (!activeSection) {
      return;
    }

    const activeParentKeys = findActiveNavigationParentKeys(activeSection.items, activeItem, activeSection.label);
    if (!activeParentKeys.length) {
      return;
    }

    setOpenNavigationItems((current) => {
      if (activeParentKeys.every((key) => Object.prototype.hasOwnProperty.call(current, key))) {
        return current;
      }

      return activeParentKeys.reduce((next, key) => (
        Object.prototype.hasOwnProperty.call(next, key) ? next : { ...next, [key]: true }
      ), current);
    });
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
    if (!hasRightPanel || rightSidebarCollapsed) {
      return;
    }

    window.requestAnimationFrame(() => {
      window.dispatchEvent(new Event("tron:open-right-sidebar"));
    });
  }, [hasRightPanel, rightPanel, rightSidebarCollapsed]);

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

  function toggleSidebarPanel() {
    if (window.innerWidth < 1024) {
      toggleMobileSidebar();
      return;
    }
    setSidebarCollapsed(!sidebarCollapsed);
  }

  function toggleRightSidebarPanel() {
    if (!hasRightPanel) return;
    if (window.innerWidth < 1024) {
      if (rightDrawerOpen) {
        closeRightDrawer();
      } else {
        openRightDrawer();
      }
      return;
    }
    window.dispatchEvent(new Event("tron:toggle-right-sidebar"));
  }

  function toggleBottomPanel() {
    setBottomPanel((open) => !open);
  }

  function closeCurrentWorkspaceTab() {
    const { activeTabId, removeTab, tabs } = useTabStore.getState();
    const activeTab = tabs.find((tab) => tab.id === activeTabId);
    if (!activeTab?.closable) return false;

    const closingIndex = tabs.findIndex((tab) => tab.id === activeTab.id);
    const remainingTabs = tabs.filter((tab) => tab.id !== activeTab.id);
    const nextTab = remainingTabs[Math.min(closingIndex, remainingTabs.length - 1)];

    removeTab(activeTab.id);
    navigate(nextTab?.path ?? "/apps");
    return true;
  }

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (
        event.defaultPrevented
        || event.isComposing
        || event.repeat
        || isEditableShortcutTarget(event.target)
      ) {
        return;
      }

      const action = findMatchingShellShortcutAction(shortcutBindings, event);
      if (!action) return;

      if (action === "toggleLeftSidebar") {
        event.preventDefault();
        toggleSidebarPanel();
        return;
      }
      if (action === "toggleRightSidebar") {
        if (!hasRightPanel) return;
        event.preventDefault();
        toggleRightSidebarPanel();
        return;
      }
      if (action === "toggleBottomPanel") {
        event.preventDefault();
        toggleBottomPanel();
        return;
      }
      if (action === "newSession") {
        if (pathname !== "/chats" && !pathname.startsWith("/chats/")) return;
        event.preventDefault();
        window.dispatchEvent(new Event(CREATE_CHAT_SESSION_SHORTCUT_EVENT));
        return;
      }
      if (action === "closeCurrentTab" && closeCurrentWorkspaceTab()) {
        event.preventDefault();
      }
    };

    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, [
    hasRightPanel,
    mobileSidebarOpen,
    navigate,
    pathname,
    rightDrawerOpen,
    shortcutBindings,
    sidebarCollapsed,
  ]);

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

  function renderUserPanelContent() {
    return (
      <>
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
              onValueChange={(event) => {
                setTheme(event! === "matrix-dark" ? "matrix-dark" : "light");
              }}
            >
              {themes.map((themeOption) => (
                <SelectItem key={themeOption.value} value={themeOption.value}>
                  {t(themeOption.labelKey)}
                </SelectItem>
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
              onValueChange={(event) => void changeLocale(event as typeof currentLocale)}
            >
              {locales.map((locale) => (
                <SelectItem key={locale.code} value={locale.code}>
                  {t(locale.labelKey)}
                </SelectItem>
              ))}
            </SelectInput>
          </div>
        </div>
      </>
    );
  }

  function renderSidebarUserSection(mobile = false) {
    const compact = mobile ? false : sidebarCollapsed;
    const displayName = authSession?.user.displayName || t("shell:utilityPanel.user");
    const email = authSession?.user.email ?? "";
    const initial = displayName.trim().charAt(0).toUpperCase() || "U";

    return (
      <div className={cn("relative shrink-0 border-t border-[var(--color-outline-variant)]", mobile ? "pt-3" : "py-2")}>
        <button
          ref={userMenuTriggerRef}
          className={cn(
            "flex min-w-0 w-full items-center rounded-xl text-left transition-colors hover:bg-[var(--color-hover-surface)]",
            compact ? "justify-center px-2 py-2" : "gap-3 px-2 py-2",
          )}
          type="button"
          aria-label={t("shell:aria.openUserMenu")}
          aria-expanded={utilityPanel === "user"}
          aria-haspopup="dialog"
          onClick={() => handleUtilityPanelClick("user")}
        >
          <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-[var(--color-primary)] text-sm font-semibold text-[var(--color-on-primary)]" aria-hidden="true">
            {initial}
          </span>
          {compact ? null : (
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm font-medium text-[var(--color-on-surface)]">{displayName}</span>
              {email ? <span className="block truncate text-[12px] text-[var(--color-on-surface-variant)]">{email}</span> : null}
            </span>
          )}
        </button>
        {utilityPanel === "user" ? (
          <div
            ref={utilityPanelRef}
            className={cn(
              "absolute bottom-[calc(100%+8px)] z-[70] max-h-[min(70vh,36rem)] overflow-y-auto rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2.5 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
              compact ? "left-[calc(100%+8px)] w-[min(360px,calc(100vw-1rem))]" : "left-0 w-[min(360px,calc(100vw-1rem))]",
            )}
            role="dialog"
            aria-label={t("shell:utilityPanel.user")}
          >
            <p className="font-medium">{t("shell:utilityPanel.user")}</p>
            <div className="mt-3 space-y-4">{renderUserPanelContent()}</div>
          </div>
        ) : null}
      </div>
    );
  }

  function renderSidebarContent(mobile = false) {
    const compact = mobile ? false : sidebarCollapsed;

    function renderNavigationItem(item: TronNavItem, itemKey: string, nestingLevel = 0): ReactNode {
      const Icon = item.icon;
      const active = item.label === activeItem;
      const itemLabel = t(item.translationKey);
      const hasChildren = Boolean(item.children?.length);
      const open = hasChildren && (compact || openNavigationItems[itemKey]);
      const childOffset = compact ? 0 : nestingLevel * 12;

      return (
        <div className="space-y-1" key={itemKey}>
          <div className="flex min-w-0 items-center" style={{ paddingLeft: `${childOffset}px` }}>
            {hasChildren && !compact ? (
              <button
                className="flex h-9 min-w-0 flex-1 items-center gap-2 rounded-lg px-3 text-sm text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]"
                type="button"
                aria-expanded={open}
                aria-label={t(open ? "shell:aria.collapseNavigationItem" : "shell:aria.expandNavigationItem", { item: itemLabel })}
                onClick={() => {
                  setOpenNavigationItems((current) => ({ ...current, [itemKey]: !current[itemKey] }));
                }}
              >
                <Icon className="shrink-0" size={17} aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate text-left">{itemLabel}</span>
                <ChevronDown className={cn("shrink-0 transition-transform", open ? "rotate-180" : "")} size={14} aria-hidden="true" />
              </button>
            ) : (
              <Link
                className={cn(
                  "flex h-9 min-w-0 flex-1 items-center rounded-lg text-sm transition-colors",
                  compact ? "justify-center px-0" : "gap-2 px-3",
                  active
                    ? "bg-[var(--color-surface-container-high)] font-medium text-[var(--color-on-surface)]"
                    : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]",
                )}
                aria-label={compact ? itemLabel : undefined}
                aria-current={active ? "page" : undefined}
                data-active={active}
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
                <span className={cn("truncate", compact ? "sr-only" : "")}>{itemLabel}</span>
              </Link>
            )}
          </div>
          {open ? (
            <div className="space-y-1">
              {item.children?.map((child) => renderNavigationItem(child, `${itemKey}/${child.label}`, nestingLevel + 1))}
            </div>
          ) : null}
        </div>
      );
    }

    return (
      <>
        <nav className={cn("space-y-4 flex-1", compact ? "space-y-2" : "")} aria-label={t("shell:aria.primaryNavigation")}>
          {visibleNavigation.map((section) => {
            const open = openSections[section.label];
            return (
              <section key={section.label} className="flex-1">
                {compact ? null : (
                  <button
                    className="flex h-8 w-full items-center justify-between rounded-lg px-2 text-[15px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]"
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
                    {section.items.map((item) => renderNavigationItem(item, `${section.label}/${item.label}`))}
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
                  onClick={toggleSidebarPanel}
                >
                  {mobileSidebarOpen ? <PanelLeftClose size={17} aria-hidden="true" /> : <PanelLeftOpen size={17} aria-hidden="true" />}
                </Button>
                <Button
                  className="-ml-3 hidden lg:flex"
                  size="icon"
                  type="button"
                  variant="ghost"
                  onClick={toggleSidebarPanel}
                >
                  {sidebarCollapsed ? <PanelLeftOpen size={17} aria-hidden="true" /> : <PanelLeftClose size={17} aria-hidden="true" />}
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
                <Button className={cn("hidden min-[900px]:inline-flex", agentOpen ? "bg-[var(--color-surface-container)] text-[var(--color-on-surface)]" : "")} size="icon" type="button" variant="ghost" aria-label={t("shell:assistant.title")} aria-expanded={agentOpen} aria-pressed={agentOpen} title={t("shell:assistant.title")} onClick={handleAgentLauncherClick}>
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
                      <button className="flex h-9 w-full items-center gap-2 rounded-lg px-2 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={handleAgentLauncherClick}>
                        <Bot size={16} aria-hidden="true" />
                        {t("shell:assistant.title")}
                      </button>
                    </div>
                  ) : null}
                </div>
                <Button
                  className={cn(
                    "hidden min-[900px]:inline-flex",
                    showBottomPanel
                      ? "bg-[var(--color-surface-container)] text-[var(--color-on-surface)]"
                      : "",
                  )}
                  size="icon"
                  type="button"
                  variant="ghost"
                  onClick={toggleBottomPanel}
                >
                  {showBottomPanel ? <PanelBottomClose size={17} /> : <PanelBottomOpen size={17} />}
                </Button>
                {hasRightPanel ? (
                  <>
                    <Button
                      className="lg:hidden"
                      size="icon"
                      type="button"
                      variant="ghost"
                      aria-controls="tron-mobile-right-sidebar"
                      aria-expanded={rightDrawerOpen}
                      aria-label={rightDrawerOpen ? t("shell:aria.closeRightSidebar") : t("shell:aria.openRightSidebar")}
                      data-testid="tron-mobile-right-sidebar-toggle"
                      onClick={toggleRightSidebarPanel}
                    >
                      {rightDrawerOpen ? <PanelRightClose size={17} aria-hidden="true" /> : <PanelRightOpen size={17} aria-hidden="true" />}
                    </Button>
                    <Button
                      className="hidden lg:flex"
                      size="icon"
                      type="button"
                      variant="ghost"
                      aria-controls="tron-mobile-right-sidebar"
                      aria-expanded={rightDragglePanelOpen}
                      aria-label={rightDragglePanelOpen ? t("shell:aria.closeRightSidebar") : t("shell:aria.openRightSidebar")}
                      data-testid="tron-mobile-right-sidebar-toggle"
                      onClick={toggleRightSidebarPanel}
                    >
                      {rightDragglePanelOpen ? <PanelRightClose size={17} aria-hidden="true" /> : <PanelRightOpen size={17} aria-hidden="true" />}
                    </Button>
                  </>
                ) : null}
              </div>
            </div>
            {utilityPanel === "workspace" ? (
              <div
                ref={utilityPanelRef}
                id="tron-workspace-selector-panel"
                className="absolute left-4 top-[calc(100%+8px)] w-[min(360px,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2.5 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)] sm:left-6"
                role="dialog"
                aria-label={t("shell:workspace.selectorTitle")}
              >
                {renderWorkspaceSwitcherContent()}
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
                className="fixed bottom-0 left-0 top-14 z-50 flex w-[min(20rem,calc(100vw-1rem))] flex-col overflow-visible bg-[var(--color-elevated-surface)] px-3 py-4 shadow-[12px_0_34px_rgba(15,23,42,0.12),0_18px_45px_rgba(15,23,42,0.12)] lg:hidden"
                id="tron-mobile-sidebar"
              >
                <div className="flex min-h-0 flex-1 flex-col">
                  <div className="min-h-0 flex-1 overflow-y-auto">
                    {renderSidebarContent(true)}
                  </div>
                  {renderSidebarUserSection(true)}
                </div>
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
            <div className="flex h-full min-h-0 flex-col px-2 pt-2">
              <div className="min-h-0 flex-1 overflow-y-auto">
                {renderSidebarContent()}
              </div>
              {renderSidebarUserSection()}
            </div>
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

          <div className={cn("relative z-0 min-w-0 pt-14 transition-[padding-left] duration-200 h-screen flex flex-col", sidebarCollapsed ? "lg:pl-[76px]" : TRON_DESKTOP_SIDEBAR_EXPANDED_PADDING_CLASS)}>
            <Content
              contentClassName={contentClassName}
              onRightPanelOpenChange={setRightDragglePanelOpen}
              rightPanel={rightPanel}
              showBottomPanel={showBottomPanel}
              changeBottomPanel={setBottomPanel}
            >
              {children}
            </Content>
          </div>

        </div>
      </WorkspaceSpacesContext.Provider>
    </TronShellFrameContext.Provider>
  );
}

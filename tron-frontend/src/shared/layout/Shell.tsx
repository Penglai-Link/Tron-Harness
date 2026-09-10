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
  ChevronsUpDown,
  MoreHorizontal,
  PanelBottomClose,
  PanelBottomOpen,
  PanelRightClose,
  PanelRightOpen,
  Settings2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  useLocation,
  useNavigate,
} from "react-router";

import { fetchWorkspaceSwitcher } from "@/domain/admin/api";
import { fetchAccountProfile } from "@/domain/account/api";
import { fetchNavigationDomains, fetchNavigationMenuTree } from "@/domain/navigation/api";
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
import { Button } from "@/components/ui/button";
import { useLocale } from "@/i18n";
import { useAppShellStore } from "@/store/appShellStore";
import { useNavigationStore } from "@/store/navigationStore";
import {
  buildWorkspaceTree,
  findWorkspaceNodePath,
  type WorkspaceTreeNode,
} from "@/shared/lib/workspaceTree";
import { CREATE_CHAT_SESSION_SHORTCUT_EVENT } from "@/shared/lib/keyboardShortcuts";
import { cn } from "@/shared/lib/utils";
import { mapNavigationTree } from "@/shared/lib/navigationMenuMapper";
import { useTheme } from "@/shared/theme/ThemeProvider";
import { SelectInput } from "@/components/forms";
import { useDismissibleLayer } from "@/shared/hooks/useDismissibleLayer";
import { useManagedAvatar } from "@/shared/hooks/useManagedAvatar";
import { useShellShortcuts } from "@/shared/hooks/useShellShortcuts";
import { DEFAULT_TAB_ID, useTabStore } from "@/store/tabStore";
import { useAuthStore } from "@/store/authStore";
import type { TronNavItem, TronNavSection } from "@/shared/types/ShellNavigation";
import type {
  WorkspaceSpacesContextValue,
  WorkspaceSwitcherData,
} from "@/shared/types/WorkspaceSpace";
import DraggleLine from "@/components/draggle-line";
import { SelectItem } from "@/components/ui/select";
import {
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Content } from "./content";
import RightPanelContainer from "./content/RightPanelContainer";
import { TronSidebar } from "./TronSidebar";
import { useTabPanelRegistry } from "./content/TabPanelRegistry";


interface TronShellProps {
  activeItem?: string;
  children: ReactNode;
  contentClassName?: string;
  roleCodes?: string[];
  searchPlaceholder?: string;
  topbarActions?: ReactNode;
  userMenuContent?: ReactNode;
}

const TronShellFrameContext = createContext(false);

type WorkspaceSwitcherTreeNode = WorkspaceTreeNode<WorkspaceSwitcherData["secondarySpaces"][number]>;

const MAIN_AGENT_WINDOW_STORAGE_KEY = "tron.main-agent.window";

export function useOpenRightSidebar() {
  const activeTabId = useTabStore((state) => state.activeTabId ?? DEFAULT_TAB_ID);
  const updateRightPanelState = useTabStore((state) => state.updateRightPanelState);

  return useCallback(() => {
    if (activeTabId) {
      updateRightPanelState(activeTabId, { open: true });
    }
  }, [activeTabId, updateRightPanelState]);
}

function navigationHrefMatches(itemHref: string, activeHref?: string) {
  return Boolean(itemHref && activeHref && (activeHref === itemHref || activeHref.startsWith(`${itemHref}/`)));
}

function navigationItemHasActiveItem(item: TronNavItem, activeItem: string, activeHref?: string): boolean {
  return item.label === activeItem
    || navigationHrefMatches(item.href, activeHref)
    || item.children?.some((child) => navigationItemHasActiveItem(child, activeItem, activeHref))
    || false;
}

function sectionHasActiveItem(section: TronNavSection, activeItem?: string, activeHref?: string) {
  return Boolean(activeItem && section.items.some((item) => navigationItemHasActiveItem(item, activeItem, activeHref)));
}

function findActiveNavigationParentKeys(
  items: TronNavItem[],
  activeItem: string,
  activeHref: string,
  parentKey: string,
): string[] {
  for (const item of items) {
    const itemKey = `${parentKey}/${item.label}`;
    if (item.children?.some((child) => navigationItemHasActiveItem(child, activeItem, activeHref))) {
      return [itemKey, ...findActiveNavigationParentKeys(item.children, activeItem, activeHref, itemKey)];
    }

    if (item.label === activeItem || navigationHrefMatches(item.href, activeHref)) {
      return [];
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
  roleCodes,
  searchPlaceholder,
  topbarActions,
  userMenuContent,
}: TronShellProps) {
  const { t } = useTranslation(["common", "shell"]);
  const { changeLocale, currentLocale, supportedLocales: locales } = useLocale();
  const { setTheme, theme, themes } = useTheme();
  const { session: authSession } = useAuthSession();
  const updateSessionUser = useAuthStore((state) => state.updateSessionUser);
  const { src: avatarSrc } = useManagedAvatar(authSession?.user.avatarUrl);
  const { pathname, search } = useLocation();
  const navigate = useNavigate();
  const shellFrameMounted = useContext(TronShellFrameContext);
  const agentOpen = useAppShellStore((state) => state.agentOpen);
  const closeFloatingPanels = useAppShellStore((state) => state.closeFloatingPanels);
  const ensureSectionOpen = useAppShellStore((state) => state.ensureSectionOpen);
  const openSections = useAppShellStore((state) => state.openSections);
  const selectedPrimarySpaceId = useAppShellStore((state) => state.selectedPrimarySpaceId);
  const selectedSecondarySpaceId = useAppShellStore((state) => state.selectedSecondarySpaceId);
  const navigationDomains = useNavigationStore((state) => state.domains);
  const navigationMenuTree = useNavigationStore((state) => state.menuTree);
  const selectedDomainCode = useNavigationStore((state) => state.selectedDomainCode);
  const setNavigationDomains = useNavigationStore((state) => state.setDomains);
  const setNavigationMenuTree = useNavigationStore((state) => state.setMenuTree);
  const setSelectedDomainCode = useNavigationStore((state) => state.setSelectedDomainCode);
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
  const tabPanelRegistry = useTabPanelRegistry();
  const activeTabId = useTabStore((state) => state.activeTabId ?? DEFAULT_TAB_ID);
  const activeTab = useTabStore((state) => state.tabs.find((tab) => tab.id === (state.activeTabId ?? DEFAULT_TAB_ID)));
  const closeTab = useTabStore((state) => state.closeTab);
  const activeShellConfig = activeTabId ? tabPanelRegistry.get(activeTabId) : undefined;
  const updateRightPanelState = useTabStore((state) => state.updateRightPanelState);
  const setActiveTab = useTabStore((state) => state.setActiveTab);
  const dataSource = getServiceDataSource("admin");
  const [openNavigationItems, setOpenNavigationItems] = useState<Record<string, boolean>>({});
  const [rightDrawerOpen, setRightDrawerOpen] = useState(false);
  const rightPanelOpen = activeTab?.rightPanelOpen ?? false;

  const [topbarToolsMenuOpen, setTopbarToolsMenuOpen] = useState(false);
  const [workspaceSwitcherLoading, setWorkspaceSwitcherLoading] = useState(false);
  const [navigationLoading, setNavigationLoading] = useState(false);
  const [mainAgentWindowState, setMainAgentWindowState] = useState<ChatFloatingState>(() => ({
    ...readFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, defaultFloatingState),
    open: false,
  }));
  const workspaceSwitcherControllerRef = useRef<AbortController | null>(null);
  const workspaceSwitcherRequestSequenceRef = useRef(0);
  const navigationDomainsControllerRef = useRef<AbortController | null>(null);
  const navigationTreeControllerRef = useRef<AbortController | null>(null);
  const utilityPanelRef = useRef<HTMLDivElement | null>(null);
  const workspaceTriggerRef = useRef<HTMLButtonElement | null>(null);
  const domainTriggerRef = useRef<HTMLButtonElement | null>(null);
  const userMenuTriggerRef = useRef<HTMLButtonElement | null>(null);
  const topbarToolsMenuRef = useRef<HTMLDivElement | null>(null);
  const topbarToolsMenuContainerRef = useRef<HTMLDivElement | null>(null);
  const [expandedWorkspaceCodes, setExpandedWorkspaceCodes] = useState<string[]>([]);
  const [showBottomPanel, setBottomPanel] = useState(false)

  useEffect(() => {
    if (!authSession) return;

    const controller = new AbortController();
    void fetchAccountProfile(controller.signal)
      .then((profile) => {
        updateSessionUser({ avatarUrl: profile.avatarUrl });
      })
      .catch(() => undefined);

    return () => controller.abort();
  }, [authSession?.user.userCode, updateSessionUser]);

  const visibleNavigation = useMemo(
    () => (navigationMenuTree ? mapNavigationTree(navigationMenuTree) : tronNavigation).filter((section) => {
      if (!section.requiredRoleCodes?.length) {
        return true;
      }

      if (!roleCodes) {
        return true;
      }

      return section.requiredRoleCodes.some((roleCode) => roleCodes.includes(roleCode));
    }),
    [navigationMenuTree, roleCodes],
  );

  const selectedNavigationDomain = navigationDomains.find((domain) => domain.domainCode === selectedDomainCode)
    ?? navigationDomains[0];

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

  const renderedContentClassName = activeShellConfig?.contentClassName ?? contentClassName;
  const renderedTopbarActions = activeShellConfig?.topbarActions ?? topbarActions;

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
    navigationDomainsControllerRef.current?.abort();
    navigationTreeControllerRef.current?.abort();

    if (!authSession) {
      setNavigationDomains([]);
      setNavigationMenuTree(null);
      setNavigationLoading(false);
      return;
    }

    if (dataSource !== "api") {
      setNavigationDomains([{
        builtIn: true,
        description: t("shell:navigation.defaultDescription"),
        domainCode: "ADVANCED",
        isDefault: true,
        name: t("shell:navigation.defaultName"),
        position: 0,
      }]);
      setNavigationMenuTree(null);
      setSelectedDomainCode("ADVANCED");
      setNavigationLoading(false);
      return;
    }

    const controller = new AbortController();
    navigationDomainsControllerRef.current = controller;
    setNavigationLoading(true);

    void fetchNavigationDomains(controller.signal)
      .then((domains) => {
        if (controller.signal.aborted) return;
        const availableDomains = [...domains].sort((left, right) => left.position - right.position);
        setNavigationDomains(availableDomains);
        const storedDomainCode = useNavigationStore.getState().selectedDomainCode;
        const nextDomain = availableDomains.find((domain) => domain.domainCode === storedDomainCode)
          ?? availableDomains.find((domain) => domain.isDefault)
          ?? availableDomains[0];
        setSelectedDomainCode(nextDomain?.domainCode ?? "");
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted || isUnauthorizedApiError(error)) return;
        console.warn("[shell] Navigation domains refresh failed.", error);
        setNavigationDomains([]);
        setNavigationMenuTree(null);
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setNavigationLoading(false);
        }
        if (navigationDomainsControllerRef.current === controller) {
          navigationDomainsControllerRef.current = null;
        }
      });

    return () => controller.abort();
  }, [authSession?.user.userCode, currentLocale, dataSource, setNavigationDomains, setNavigationMenuTree, setSelectedDomainCode, t]);

  useEffect(() => {
    navigationTreeControllerRef.current?.abort();
    if (!authSession || dataSource !== "api" || !selectedDomainCode || !navigationDomains.some((domain) => domain.domainCode === selectedDomainCode)) {
      return;
    }

    const controller = new AbortController();
    navigationTreeControllerRef.current = controller;
    setNavigationLoading(true);

    void fetchNavigationMenuTree(selectedDomainCode, controller.signal)
      .then((tree) => {
        if (!controller.signal.aborted) {
          setNavigationMenuTree(tree);
        }
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted || isUnauthorizedApiError(error)) return;
        console.warn("[shell] Navigation menu tree refresh failed.", error);
        setNavigationMenuTree(null);
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setNavigationLoading(false);
        }
        if (navigationTreeControllerRef.current === controller) {
          navigationTreeControllerRef.current = null;
        }
      });

    return () => controller.abort();
  }, [authSession?.user.userCode, currentLocale, dataSource, navigationDomains, selectedDomainCode, setNavigationMenuTree]);

  useEffect(() => {
    const activeSection = visibleNavigation.find((section) => sectionHasActiveItem(section, activeItem, pathname));
    ensureSectionOpen(activeSection?.id ?? activeSection?.label);

    if (!activeSection) {
      return;
    }

    const activeParentKeys = findActiveNavigationParentKeys(activeSection.items, activeItem, pathname, activeSection.label);
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
  }, [activeItem, ensureSectionOpen, pathname, visibleNavigation]);

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
    window.dispatchEvent(new Event("tron:close-sidebar"));
    setRightDrawerOpen(false);
    setTopbarToolsMenuOpen(false);
  }, [pathname]);

  useEffect(() => {
    setRightDrawerOpen(Boolean(activeTab?.rightPanelOpen));
  }, [activeTab?.id, activeTab?.rightPanelOpen]);

  useEffect(() => {
    if (!rightDrawerOpen) {
      return;
    }

    const previousOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setRightDrawerOpen(false);
      }
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [rightDrawerOpen]);

  useDismissibleLayer({
    open: Boolean(utilityPanel),
    onDismiss: () => {
      if (!utilityPanel) return;
      toggleUtilityPanel(utilityPanel);
    },
    popupRef: utilityPanelRef,
    triggerElement:
      utilityPanel === "domain"
        ? domainTriggerRef.current
        : utilityPanel === "workspace"
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

  function openRightDrawer() {
    closeFloatingPanels();
    setTopbarToolsMenuOpen(false);
    window.dispatchEvent(new Event("tron:close-sidebar"));
    setRightDrawerOpen(true);
    if (activeTab?.id) {
      updateRightPanelState(activeTab.id, { open: true });
    }
  }

  function closeRightDrawer() {
    setRightDrawerOpen(false);
    if (activeTab?.id) {
      updateRightPanelState(activeTab.id, { open: false });
    }
  }

  function toggleRightSidebarPanel() {
    if (window.innerWidth < 1024) {
      if (rightDrawerOpen) {
        closeRightDrawer();
      } else {
        openRightDrawer();
      }
      return;
    }

    if (activeTab?.id) {
      updateRightPanelState(activeTab.id, { open: !rightPanelOpen });
    } else {
      setActiveTab(DEFAULT_TAB_ID)
      updateRightPanelState(DEFAULT_TAB_ID, { open: !rightPanelOpen });
    }
  }

  function toggleBottomPanel() {
    setBottomPanel((open) => !open);
  }

  useShellShortcuts({
    closeCurrentTab: () => {
      if (!activeTabId) return false;
      const redirect = closeTab(activeTabId);
      if (redirect === null) return false;
      navigate(redirect);
    },
    newSession: () => {
      if (pathname !== "/chats" && !pathname.startsWith("/chats/")) return false;
      window.dispatchEvent(new Event(CREATE_CHAT_SESSION_SHORTCUT_EVENT));
    },
    toggleBottomPanel: () => toggleBottomPanel(),
    toggleLeftSidebar: (event) => {
      // SidebarProvider owns its built-in Ctrl/Cmd+B shortcut. Its listener
      // re-registers whenever the sidebar state changes and may then run
      // after ours, so leave that combination to it instead of toggling twice.
      if (event.key.toLowerCase() === "b" && (event.metaKey || event.ctrlKey)) {
        return false;
      }
      window.dispatchEvent(new Event("tron:toggle-sidebar"));
    },
    toggleRightSidebar: () => {
      toggleRightSidebarPanel();
    },
  });

  function handleSidebarNavigationClick() {
    closeFloatingPanels();
  }

  function handleWorkspaceSwitcherClick() {
    setAgentOpen(false);
    if (utilityPanel !== "workspace") {
      void refreshWorkspaceSwitcher();
    }
    toggleUtilityPanel("workspace");
  }

  function handleNavigationDomainSwitcherClick() {
    setAgentOpen(false);
    setTopbarToolsMenuOpen(false);
    toggleUtilityPanel("domain");
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
      "flex min-w-0 flex-1 cursor-pointer items-center rounded-lg px-2.5 py-1.5 text-left transition-colors",
      selected
        ? "bg-[var(--color-menu-selected-surface)]"
        : "hover:bg-[var(--color-menu-hover-surface)]",
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
              className="flex size-6 shrink-0 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] hover:bg-[var(--color-menu-hover-surface)] hover:text-[var(--color-on-surface)]"
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

  function renderNavigationDomainSwitcherContent() {
    return (
      <div className="space-y-1" role="listbox" aria-label={t("shell:navigation.selectorTitle")}>
        {navigationLoading ? <p className="px-2 py-1 text-[12px] text-[var(--color-on-surface-variant)]">{t("shell:navigation.refreshing")}</p> : null}
        {navigationDomains.length ? navigationDomains.map((domain) => {
          const selected = domain.domainCode === selectedDomainCode;
          return (
            <button
              aria-selected={selected}
              className={cn(
                "flex w-full min-w-0 items-start rounded-lg px-2.5 py-2 text-left transition-colors",
                selected
                  ? "bg-[var(--color-menu-selected-surface)]"
                  : "hover:bg-[var(--color-menu-hover-surface)]",
              )}
              key={domain.domainCode}
              role="option"
              type="button"
              onClick={() => {
                setSelectedDomainCode(domain.domainCode);
                toggleUtilityPanel("domain");
              }}
            >
              <span className="min-w-0">
                <span className="block truncate text-sm font-medium">{domain.name}</span>
                {domain.description ? <span className="mt-0.5 block truncate text-[12px] text-[var(--color-on-surface-variant)]">{domain.description}</span> : null}
              </span>
            </button>
          );
        }) : (
          <p className="px-2 py-1 text-[13px] text-[var(--color-on-surface-variant)]">{t("shell:navigation.noDomains")}</p>
        )}
      </div>
    );
  }

  function renderUserPanelContent() {
    return (
      <>
        <div className="space-y-3 p-3">
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
        {userMenuContent ? (
          <div className="">
            {userMenuContent}
          </div>
        ) : null}
      </>
    );
  }

  function renderSidebarUserSection(compact: boolean, mobile: boolean) {
    const displayName = authSession?.user.displayName || t("shell:utilityPanel.user");
    const email = authSession?.user.email ?? "";
    const initial = displayName.trim().charAt(0).toUpperCase() || "U";

    return (
      <div className="relative shrink-0">
        <button
          ref={userMenuTriggerRef}
          className={cn(
            "flex h-14 min-w-0 w-full cursor-pointer items-center rounded-lg text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus-ring)]",
            compact ? "justify-center px-1" : "gap-3 px-2.5 hover:bg-[var(--color-hover-surface)]",
          )}
          type="button"
          aria-label={t("shell:aria.openUserMenu")}
          aria-expanded={utilityPanel === "user"}
          aria-haspopup="dialog"
          onClick={() => handleUtilityPanelClick("user")}
        >
          <span className="flex size-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-primary)] text-sm font-semibold text-[var(--color-on-primary)]" aria-hidden="true">
            {avatarSrc ? <img alt="" className="size-full object-cover" src={avatarSrc} /> : initial}
          </span>
          {compact ? null : (
            <>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium text-[var(--color-on-surface)]">{displayName}</span>
                {email ? <span className="block truncate text-[12px] text-[var(--color-on-surface-variant)]">{email}</span> : null}
              </span>
              <ChevronsUpDown className="shrink-0 text-[var(--color-on-surface-variant)]" size={16} aria-hidden="true" />
            </>
          )}
        </button>
        {utilityPanel === "user" ? (
          <div
            ref={utilityPanelRef}
            className={cn(
              "absolute z-[70] w-[min(20rem,calc(100vw-1rem))] overflow-y-auto rounded-lg border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
              mobile ? "bottom-[calc(100%+8px)] left-0 max-h-[min(70vh,36rem)]" : "bottom-0 left-[calc(100%+8px)] max-h-[min(80vh,36rem)]",
            )}
            role="dialog"
            aria-label={t("shell:utilityPanel.user")}
          >
            <div className="flex min-w-0 items-center gap-3 px-3 py-3">
              <span className="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-primary)] text-sm font-semibold text-[var(--color-on-primary)]" aria-hidden="true">
                {avatarSrc ? <img alt="" className="size-full object-cover" src={avatarSrc} /> : initial}
              </span>
              <span className="min-w-0">
                <span className="block truncate text-sm font-medium text-[var(--color-on-surface)]">{displayName}</span>
                {email ? <span className="block truncate text-[12px] text-[var(--color-on-surface-variant)]">{email}</span> : null}
              </span>
            </div>
            <div className="border-t border-[var(--color-card-border)]">{renderUserPanelContent()}</div>
          </div>
        ) : null}
      </div>
    );
  }

  function renderNavigationDomainSelector(compact: boolean, mobile: boolean) {
    const domainName = selectedNavigationDomain?.name ?? t("shell:navigation.defaultName");
    return (
      <div className="relative">
        <button
          ref={domainTriggerRef}
          className={cn(
            "flex h-10 w-full min-w-0 items-center rounded-lg text-left text-sm font-medium text-[var(--color-on-surface)] transition-colors hover:bg-[var(--color-hover-surface)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus-ring)]",
            compact ? "justify-center px-1.5" : "justify-between gap-2 px-2.5 text-[15px]",
          )}
          type="button"
          id="tron-domain-trigger"
          aria-busy={navigationLoading}
          aria-expanded={utilityPanel === "domain"}
          aria-haspopup="dialog"
          aria-label={utilityPanel === "domain" ? t("shell:aria.closeDomainSelector") : t("shell:aria.openDomainSelector")}
          onClick={handleNavigationDomainSwitcherClick}
          title={domainName}
        >
          {compact ? (
            <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-[var(--color-surface-container)] text-sm font-semibold" aria-hidden="true">
              {domainName.slice(0, 1)}
            </span>
          ) : (
            <span className="min-w-0 truncate">{domainName}</span>
          )}
          <ChevronDown className={cn("shrink-0 text-[var(--color-on-surface-variant)]", compact ? "hidden" : "")} size={15} aria-hidden="true" />
        </button>
        {utilityPanel === "domain" ? (
          <div
            ref={utilityPanelRef}
            id="tron-domain-selector-panel"
            className={cn(
              "absolute z-[70] w-[min(260px,calc(100vw-2rem))] rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2.5 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
              compact && !mobile ? "left-[calc(100%+8px)] top-0" : "left-0 top-[calc(100%+8px)]",
            )}
            role="dialog"
            aria-label={t("shell:navigation.selectorTitle")}
          >
            {renderNavigationDomainSwitcherContent()}
          </div>
        ) : null}
      </div>
    );
  }

  if (shellFrameMounted) {
    return <>{children}</>;
  }

  return (
    <TronShellFrameContext.Provider value={true}>
      <WorkspaceSpacesContext.Provider value={workspaceSpacesValue}>
        <SidebarProvider
          className="isolate min-h-screen bg-[var(--color-background)] text-[var(--color-on-surface)]"
          open={!sidebarCollapsed}
          onOpenChange={(open) => setSidebarCollapsed(!open)}
        >
          <header className="fixed inset-x-0 top-0 z-[60] h-14 bg-[var(--color-header-backdrop)] px-4 shadow-[0_10px_28px_rgba(15,23,42,0.045)] backdrop-blur sm:px-6">
            <div className="flex h-full items-center justify-between gap-4">
              <div className="flex min-w-0 flex-1 items-center gap-3">
                <SidebarTrigger aria-label={t("shell:aria.openNavigation")} className="-ml-3 size-9 cursor-pointer md:hidden" />
                <SidebarTrigger
                  aria-label={sidebarCollapsed ? t("shell:aria.openNavigation") : t("shell:aria.closeNavigation")}
                  className="-ml-3 hidden size-9 cursor-pointer md:flex"
                />
                <button
                  ref={workspaceTriggerRef}
                  className="ml-1 inline-flex h-9 max-w-[min(60vw,28rem)] shrink-0 cursor-pointer items-center gap-[12px] rounded-[4px] border border-[#d9d9d9] bg-white px-3 text-sm font-medium text-[var(--color-on-surface)]"
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
              <div className="flex shrink-0 items-center gap-[12px]">
                {renderedTopbarActions}
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
                      <button className="flex h-9 w-full items-center gap-[12px] rounded-lg px-2 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={openSystemSettings}>
                        <Settings2 size={16} aria-hidden="true" />
                        {t("shell:utilityPanel.settings")}
                      </button>
                      <button className="flex h-9 w-full items-center gap-[12px] rounded-lg px-2 text-left hover:bg-[var(--color-hover-surface)]" type="button" role="menuitem" onClick={handleAgentLauncherClick}>
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
                    aria-expanded={rightPanelOpen}
                    aria-label={rightPanelOpen ? t("shell:aria.closeRightSidebar") : t("shell:aria.openRightSidebar")}
                    data-testid="tron-mobile-right-sidebar-toggle"
                    onClick={toggleRightSidebarPanel}
                  >
                    {rightPanelOpen ? <PanelRightClose size={17} aria-hidden="true" /> : <PanelRightOpen size={17} aria-hidden="true" />}
                  </Button>
                  </>
              </div>
            </div>
            {utilityPanel === "workspace" ? (
              <div
                ref={utilityPanelRef}
                id="tron-workspace-selector-panel"
                className="absolute left-4 top-[calc(100%+8px)] w-[min(260px,calc(100vw-2rem))] rounded-lg border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] px-3 py-2.5 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)] sm:left-6"
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

          {rightDrawerOpen ? (
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
                <RightPanelContainer />
              </aside>
            </>
          ) : null}

          <TronSidebar
            activeItem={activeItem}
            activeHref={pathname}
            navigation={visibleNavigation}
            openNavigationItems={openNavigationItems}
            openSections={openSections}
            renderDomainSelector={renderNavigationDomainSelector}
            renderFooter={renderSidebarUserSection}
            setOpenNavigationItems={setOpenNavigationItems}
            translate={(key, options) => t(key, options)}
            onNavigate={handleSidebarNavigationClick}
            onToggleSection={toggleSection}
          />

          <div className="relative z-0 flex h-screen min-w-0 flex-1 flex-col pt-14">
            <Content
              contentClassName={renderedContentClassName}
              showBottomPanel={showBottomPanel}

              changeBottomPanel={setBottomPanel}
            >
              {children}
            </Content>
          </div>

        </SidebarProvider>
      </WorkspaceSpacesContext.Provider>
    </TronShellFrameContext.Provider>
  );
}

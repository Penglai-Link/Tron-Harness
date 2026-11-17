import { TabBar } from "./content/TabBar";
import { CREATE_TOPIC_SHORTCUT_EVENT, ADD_DIRECTORY_SHORTCUT_EVENT } from "@/shared/lib/keyboardShortcuts";
import {
  Settings,
  Check,
  Bot,
  ChevronDown,
  PanelBottomClose,
  PanelBottomOpen,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
} from "lucide-react";
import { createPortal } from "react-dom";
import { ShinyText } from "@/shared/motion";
import { setApiWorkspaceCode } from "@/shared/api/httpClient";
import { defaultMainAgentWindowState, MAIN_AGENT_WINDOW_STORAGE_KEY } from "@/domain/agent/model/mainAgentWindowState";
import { LocaleMenu } from "./LocaleMenu";
import { TimeZoneMenu } from "./TimeZoneMenu";
import { ThemeMenu } from "./ThemeMenu";
import { CheckinButton } from "@/domain/auth/components/CheckinButton";
import {
  useCallback,
  createContext,
  cloneElement,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  lazy,
  Suspense,
  type CSSProperties,
  type ReactElement,
  type ReactNode,
} from "react";

import {
  readFloatingState,
  writeFloatingState,
  type ChatSession,
  type ChatFloatingState,
} from "@tron/chat-core";

import { useTranslation } from "react-i18next";
import {
  useLocation,
  useNavigate,
} from "react-router";

import { fetchWorkspaceSwitcher } from "@/domain/admin/api";
import { fetchNavigationDomains, fetchNavigationMenuTree } from "@/domain/navigation/api";
import { useAuthSession } from "@/domain/auth";
import { NotificationLauncher } from "@/domain/notification/components/NotificationLauncher";
import { RemoteAccessDialog } from "@/domain/settings/components/RemoteAccessDialog";
import { MainAgentChatWindow } from "@/domain/agent/components/MainAgentChatWindow";
import { captureAssistantPageContext, AssistantPageContext } from "./assistantPageContext";
import { isUnauthorizedApiError } from "@/shared/api/httpClient";
import { getServiceDataSource } from "@/shared/api/serviceConfig";
import { tronNavigation } from "@/shared/constants/shellNavigation";
import { getEmptyWorkspaceSwitcher, getWorkspaceSwitcherMock } from "@/shared/constants/workspaceSpaces";
import { WorkspaceSpacesContext } from "@/shared/hooks/useWorkspaceSpaces";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useLocale } from "@/i18n";
import { useAppShellStore } from "@/store/appShellStore";
import { useNavigationStore } from "@/store/navigationStore";
import {
  buildWorkspaceTree,
  type WorkspaceTreeNode,
} from "@/shared/lib/workspaceTree";
import { useCreateChatTab } from "./chat/hooks/useCreateChatTab";
import { OPEN_CHAT_CONTEXT_EVENT, OPEN_CHATS_PAGE_VIEW_EVENT } from "@/shared/lib/shellEvents";
import { cn } from "@/shared/lib/utils";
import { mapNavigationTree } from "@/shared/lib/navigationMenuMapper";
import { useDismissibleLayer } from "@/shared/hooks/useDismissibleLayer";
import { useManagedAvatar } from "@/shared/hooks/useManagedAvatar";
import { useShellShortcuts } from "@/shared/hooks/useShellShortcuts";
import { useIsMobile } from "@/hooks/use-mobile";
import { DEFAULT_TAB_ID, panelTabId, useTabStore } from "@/store/tabStore";
import { useAuthStore } from "@/store/authStore";
import type { TronNavItem, TronNavSection } from "@/shared/types/ShellNavigation";
import type {
  WorkspaceSpacesContextValue,
  WorkspaceSwitcherData,
} from "@/shared/types/WorkspaceSpace";
import DraggleLine from "@/components/draggle-line";
import {
  SidebarProvider,
  useSidebar,
} from "@/components/ui/sidebar";
import { Content } from "./content";
import RightPanelContainer from "./content/RightPanelContainer";
import { TronSidebar } from "./TronSidebar";
import { WorkspaceLayout } from "./WorkspaceLayout";
import { ChatNavigationProvider } from "./chat";
import { useTabPanelRegistry } from "./content/TabPanelRegistry";
import { ShortcutHint } from "@/shared/shortcuts/ShortcutHints";
import type { ShellShortcutAction } from "@/shared/lib/keyboardShortcuts";

const ChatContextInspector = lazy(() => import("@/domain/chat/components/ChatContextInspector").then((module) => ({ default: module.ChatContextInspector })));

function ToolbarControl({ label, children, shortcutAction }: { label: string; children: ReactElement<{ children?: ReactNode }>; shortcutAction?: ShellShortcutAction }) {
  return (
    <Tooltip>
      <TooltipTrigger render={cloneElement(children, undefined, children.props.children,
        shortcutAction ? <ShortcutHint action={shortcutAction} compact /> : null,
      )} className="tron-shell-tool" />
      <TooltipContent side="bottom" sideOffset={8}>{label}</TooltipContent>
    </Tooltip>
  );
}

function MobileNavigationEntry() {
  const { isMobile, openMobile, toggleSidebar } = useSidebar();
  const { t } = useTranslation();
  if (!isMobile) return null;

  return (
    <Button
      className="tron-shell-tool"
      variant="ghost"
      size="icon"
      aria-label={t(openMobile ? "shell:aria.closeNavigation" : "shell:aria.openNavigation")}
      aria-controls="tron-sidebar"
      aria-expanded={openMobile}
      onClick={toggleSidebar}
    >
      {openMobile ? <PanelLeftClose size={17} aria-hidden="true" /> : <PanelLeftOpen size={17} aria-hidden="true" />}
      <ShortcutHint action="toggleLeftSidebar" compact />
    </Button>
  );
}

interface TronShellProps {
  activeItem?: string;
  children: ReactNode;
  contentClassName?: string;
  searchPlaceholder?: string;
  topbarActions?: ReactNode;
  userMenuContent?: ReactNode;
}

const TronShellFrameContext = createContext(false);

type WorkspaceSwitcherTreeNode = WorkspaceTreeNode<WorkspaceSwitcherData["secondarySpaces"][number]>;

const PERSON_AI_WEBSITE_URL = "https://personai.chat/person-ai-index/user/home/usage";

export { useOpenRightSidebar } from "@/shared/hooks/useOpenRightSidebar";

function navigationHrefMatches(itemHref: string, activeHref?: string) {
  return Boolean(itemHref && activeHref && (activeHref === itemHref || activeHref.startsWith(`${itemHref}/`)));
}

function personAiPlanKey(level: number) {
  switch (level) {
    case 0:
      return "free";
    case 1:
      return "vip";
    case 2:
      return "pro";
    case 3:
      return "max";
    default:
      return "level";
  }
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

function formatRemainingPoints(value: number, locale: string) {
  return new Intl.NumberFormat(locale, {
    compactDisplay: "short",
    maximumFractionDigits: 2,
    notation: "compact",
  }).format(value);
}

export function TronShell({
  activeItem = "Apps",
  children,
  contentClassName,
  searchPlaceholder,
  topbarActions,
  userMenuContent,
}: TronShellProps) {
  const { t } = useTranslation(["common", "shell"]);
  const { currentLocale } = useLocale();
  const { session: authSession } = useAuthSession();
  const personAiPlan = authSession?.user.vipLevel === null || authSession?.user.vipLevel === undefined
    ? null
    : t(`shell:userMenu.plans.${personAiPlanKey(authSession.user.vipLevel)}`, {
      level: authSession.user.vipLevel,
    });
  const remainingPoints = authSession?.user.remainingPoints === null || authSession?.user.remainingPoints === undefined
    ? null
    : formatRemainingPoints(authSession.user.remainingPoints, currentLocale);
  const ensureSessionFresh = useAuthStore((state) => state.ensureSessionFresh);
  const { src: avatarSrc } = useManagedAvatar(authSession?.user.avatarUrl);
  const handleAvatarLoadError = useCallback(() => {
    void ensureSessionFresh();
  }, [ensureSessionFresh]);
  const isMobileViewport = useIsMobile();
  const isRightDrawerViewport = useIsMobile(1024);
  const { pathname, search } = useLocation();
  const navigate = useNavigate();
  const navigateToPath = useCallback((path: string) => navigate(path), [navigate]);
  const shellFrameMounted = useContext(TronShellFrameContext);
  const agentOpen = useAppShellStore((state) => state.agentOpen);
  const closeFloatingPanels = useAppShellStore((state) => state.closeFloatingPanels);
  const ensureSectionOpen = useAppShellStore((state) => state.ensureSectionOpen);
  const openSections = useAppShellStore((state) => state.openSections);
  const selectedPrimarySpaceId = useAppShellStore((state) => state.selectedPrimarySpaceId);
  const selectedSecondarySpaceId = useAppShellStore((state) => state.selectedSecondarySpaceId);
  const [navigationRevision, setNavigationRevision] = useState(0);
  useEffect(() => {
    const refresh = () => setNavigationRevision((value) => value + 1);
    window.addEventListener("tron:navigation-updated", refresh);
    return () => window.removeEventListener("tron:navigation-updated", refresh);
  }, []);
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
  const cachedWorkspaceSwitcher = useAppShellStore((state) => state.workspaceSwitcherSnapshot);
  const [workspaceSwitcherValidated, setWorkspaceSwitcherValidated] = useState(false);
  const tabPanelRegistry = useTabPanelRegistry();
  const activeTabId = useTabStore((state) => state.activeTabId ?? DEFAULT_TAB_ID);
  const activePanelId = useTabStore((state) => panelTabId(state.activeTabId, state.workspaceCode));
  const activeTab = useTabStore((state) => state.tabs.find((tab) => tab.id === (state.activeTabId ?? DEFAULT_TAB_ID)));
  const closeTab = useTabStore((state) => state.closeTab);
  const activeShellConfig = activeTabId ? tabPanelRegistry.get(activePanelId) : undefined;
  const updateRightPanelState = useTabStore((state) => state.updateRightPanelState);
  const setActiveTab = useTabStore((state) => state.setActiveTab);
  const dataSource = getServiceDataSource("admin");
  // Persisted choices are display preferences, not proof that a workspace still exists.
  const workspaceSwitcher = useMemo(() => (
    dataSource === "api" && !workspaceSwitcherValidated
      ? getEmptyWorkspaceSwitcher()
      : cachedWorkspaceSwitcher
  ), [cachedWorkspaceSwitcher, dataSource, workspaceSwitcherValidated]);
  const [openNavigationItems, setOpenNavigationItems] = useState<Record<string, boolean>>({});
  const [rightDrawerOpen, setRightDrawerOpen] = useState(false);
  const rightPanelOpen = activeTab?.rightPanelOpen ?? false;

  // 右侧边栏已常驻，任意路由下的悬浮窗"内部打开"都应落在当前激活标签的右侧面板中
  useEffect(() => {
    const handleOpenPageView = (event: Event) => {
      const url = (event as CustomEvent<string>).detail;
      if (typeof url !== "string" || !url.trim()) return;

      const normalizedUrl = url.trim();
      const ownerKey = tabPanelRegistry.getRightPanelOwnerKey(activePanelId);
      const scopedPrefix = ownerKey ? `${encodeURIComponent(ownerKey)}:` : "";
      const key = `${scopedPrefix}markdown-page:${normalizedUrl}`;

      tabPanelRegistry.openRightPanelTab(activePanelId, {
        key,
        stableId: key,
        ownerKey,
        label: normalizedUrl,
        url: normalizedUrl,
        children: null,
      });
      updateRightPanelState(activeTabId, { open: true });
    };

    window.addEventListener(OPEN_CHATS_PAGE_VIEW_EVENT, handleOpenPageView);
    return () => window.removeEventListener(OPEN_CHATS_PAGE_VIEW_EVENT, handleOpenPageView);
  }, [activeTabId, activePanelId, tabPanelRegistry, updateRightPanelState]);

  useEffect(() => {
    const handleOpenContext = (event: Event) => {
      const session = (event as CustomEvent<ChatSession | null>).detail ?? null;
      const ownerKey = tabPanelRegistry.getRightPanelOwnerKey(activePanelId);
      const configuredContext = tabPanelRegistry.get(activePanelId)?.rightPanelTabs?.some((tab) => tab.key === "context");
      if (session?.sessionCode && ownerKey === session.sessionCode && configuredContext) {
        tabPanelRegistry.setRightPanelActiveKey(activePanelId, "context");
      } else {
        const prefix = ownerKey ? `${encodeURIComponent(ownerKey)}:` : "";
        const key = `${prefix}chat-context:${session?.sessionCode ?? "draft"}`;
        const tab = {
          key, stableId: key, ownerKey, closable: true,
          label: t("pages:chat.shortTermMemory.shortTermMemoryTab"),
          children: <Suspense fallback={null}><ChatContextInspector session={session} /></Suspense>,
        };
        if (tabPanelRegistry.getRightPanelTabs(activePanelId).some((item) => item.key === key)) {
          tabPanelRegistry.updateRightPanelTab(activePanelId, key, tab);
        } else {
          tabPanelRegistry.openRightPanelTab(activePanelId, tab);
        }
        tabPanelRegistry.setRightPanelActiveKey(activePanelId, key);
      }
      updateRightPanelState(activeTabId, { open: true });
    };
    window.addEventListener(OPEN_CHAT_CONTEXT_EVENT, handleOpenContext);
    return () => window.removeEventListener(OPEN_CHAT_CONTEXT_EVENT, handleOpenContext);
  }, [activeTabId, activePanelId, tabPanelRegistry, t, updateRightPanelState]);

  const [workspaceSwitcherLoading, setWorkspaceSwitcherLoading] = useState(false);
  const [navigationLoading, setNavigationLoading] = useState(false);
  const [mainAgentWindowState, setMainAgentWindowState] = useState<ChatFloatingState>(() => ({
    ...readFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, defaultMainAgentWindowState),
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
  const userSectionRef = useRef<HTMLDivElement | null>(null);
  const [expandedWorkspaceCodes, setExpandedWorkspaceCodes] = useState<string[]>([]);
  const [showBottomPanel, setBottomPanel] = useState(false)
  const terminalSessionCode = new URLSearchParams(search).get("sessionCode") ?? "";
  useEffect(() => {
    const open = (event: Event) => {
      if ((event as CustomEvent<{sessionCode:string}>).detail?.sessionCode === terminalSessionCode) setBottomPanel(true);
    };
    window.addEventListener("tron:open-agent-terminal", open);
    return () => window.removeEventListener("tron:open-agent-terminal", open);
  }, [terminalSessionCode]);
  const [userPanelStyle, setUserPanelStyle] = useState<CSSProperties | undefined>();

  const updateUserPanelPosition = useCallback(() => {
    if (utilityPanel !== "user" || !userSectionRef.current || !utilityPanelRef.current) {
      return;
    }

    const triggerRect = userSectionRef.current.getBoundingClientRect();
    const panelElement = utilityPanelRef.current;
    const panelRect = panelElement.getBoundingClientRect();
    const viewport = window.visualViewport;
    const viewportWidth = viewport?.width ?? window.innerWidth;
    const viewportHeight = viewport?.height ?? window.innerHeight;
    const margin = 8;
    const gap = 8;
    const viewportLeft = viewport?.offsetLeft ?? 0;
    const viewportTop = viewport?.offsetTop ?? 0;
    const maxWidth = Math.max(1, viewportWidth - margin * 2);
    const panelWidth = Math.min(panelRect.width, maxWidth);
    const preferredLeft = isMobileViewport ? triggerRect.left : triggerRect.right + gap;
    const left = Math.max(viewportLeft + margin, Math.min(preferredLeft, viewportLeft + viewportWidth - panelWidth - margin));

    // Anchor to the viewport bottom, outside the sidebar's filtered containing block.
    const nextStyle: CSSProperties = {
      left,
      bottom: Math.max(0, window.innerHeight - viewportTop - viewportHeight) + margin,
      maxWidth,
      visibility: "visible",
    };

    setUserPanelStyle((current) => (
      current
      && current.left === nextStyle.left
      && current.bottom === nextStyle.bottom
      && current.maxWidth === nextStyle.maxWidth
      && current.visibility === nextStyle.visibility
        ? current
        : nextStyle
    ));
  }, [isMobileViewport, utilityPanel]);

  useLayoutEffect(() => {
    if (utilityPanel !== "user") {
      setUserPanelStyle(undefined);
      return;
    }

    let frame = window.requestAnimationFrame(updateUserPanelPosition);
    const scheduleUpdate = () => {
      window.cancelAnimationFrame(frame);
      frame = window.requestAnimationFrame(updateUserPanelPosition);
    };
    const visualViewport = window.visualViewport;
    const resizeObserver = new ResizeObserver(scheduleUpdate);

    window.addEventListener("resize", scheduleUpdate);
    window.addEventListener("scroll", scheduleUpdate, true);
    visualViewport?.addEventListener("resize", scheduleUpdate);
    visualViewport?.addEventListener("scroll", scheduleUpdate);
    if (userSectionRef.current) resizeObserver.observe(userSectionRef.current);
    if (utilityPanelRef.current) resizeObserver.observe(utilityPanelRef.current);

    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("resize", scheduleUpdate);
      window.removeEventListener("scroll", scheduleUpdate, true);
      visualViewport?.removeEventListener("resize", scheduleUpdate);
      visualViewport?.removeEventListener("scroll", scheduleUpdate);
      resizeObserver.disconnect();
    };
  }, [updateUserPanelPosition, utilityPanel]);

  const visibleNavigation = useMemo(
    () => navigationMenuTree ? mapNavigationTree(navigationMenuTree) : tronNavigation,
    [navigationMenuTree],
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
  const selectedWorkspaceName = selectedWorkspaceNode?.name
    ?? selectedPrimarySpace?.name ?? t("shell:utilityPanel.workspace");
  const activeWorkspaceCode = effectiveSecondarySpaceId || effectivePrimarySpaceId;
  const pageSnapshotSourceRef = useRef({ pathname, search, workspaceCode: activeWorkspaceCode || null,
    workspaceName: selectedWorkspaceNode?.name ?? selectedPrimarySpace?.name ?? null, t });
  pageSnapshotSourceRef.current = { pathname, search, workspaceCode: activeWorkspaceCode || null,
    workspaceName: selectedWorkspaceNode?.name ?? selectedPrimarySpace?.name ?? null, t };
  const getMainAgentPageContext = useCallback(() => {
    const tabs = useTabStore.getState();
    return captureAssistantPageContext({
      ...pageSnapshotSourceRef.current,
      tab: tabs.tabs.find((tab) => tab.id === tabs.activeTabId),
    });
  }, []);
  useLayoutEffect(() => {
    setApiWorkspaceCode(activeWorkspaceCode || null);
    if (activeWorkspaceCode) {
      const path = useTabStore.getState().switchWorkspace(activeWorkspaceCode);
      if (path) navigate(path, { replace: true });
    }
    return () => setApiWorkspaceCode(null);
  }, [activeWorkspaceCode, navigate]);

  useEffect(() => {
    if (pathname !== "/chats") return;
    const pendingPath = useTabStore.getState().pendingWorkspacePath;
    if (pendingPath && pendingPath !== `${pathname}${search}`) return;

    const params = new URLSearchParams(search);
    const explicitWorkspaceCode = params.get("workspaceCode");
    const requestedWorkspaceCode = explicitWorkspaceCode
      ?? useTabStore.getState().getWorkspaceForPath(`${pathname}${search}`);
    if (!requestedWorkspaceCode || (dataSource === "api" && !workspaceSwitcherValidated)) return;

    const secondarySpace = workspaceSwitcher.secondarySpaces.find(
      (space) => space.workspaceCode === requestedWorkspaceCode,
    );
    const primarySpace = workspaceSwitcher.primarySpaces.find(
      (space) => space.workspaceCode === requestedWorkspaceCode,
    );
    const targetPrimarySpaceId = secondarySpace?.primaryWorkspaceCode ?? primarySpace?.workspaceCode;
    const targetPath = `${pathname}${search}`;
    let switched = false;
    if (targetPrimarySpaceId) {
      const targetSecondarySpaceId = secondarySpace?.depth && secondarySpace.depth > 1
        ? secondarySpace.workspaceCode
        : "";
      if (targetPrimarySpaceId !== selectedPrimarySpaceId || targetSecondarySpaceId !== selectedSecondarySpaceId) {
        useTabStore.getState().switchWorkspace(requestedWorkspaceCode, targetPath);
        setApiWorkspaceCode(requestedWorkspaceCode);
        selectPrimarySpaceInStore(targetPrimarySpaceId, targetSecondarySpaceId);
        switched = true;
      }
    }

    // Chat links retain their owner so reload/back/forward can restore its space.
    // Workspace-picker navigation already moves to that space's saved route.
    if (switched) navigate(targetPath, { replace: true });
    else if (explicitWorkspaceCode && !targetPrimarySpaceId) navigate("/workspace", { replace: true });
  }, [
    dataSource,
    navigate,
    pathname,
    search,
    selectPrimarySpaceInStore,
    selectedPrimarySpaceId,
    selectedSecondarySpaceId,
    workspaceSwitcher.primarySpaces,
    workspaceSwitcher.secondarySpaces,
    workspaceSwitcherValidated,
  ]);

  const { create: createChatTab } = useCreateChatTab(activeWorkspaceCode);
  const workspaceTree = useMemo(
    () => buildWorkspaceTree(workspaceSwitcher.secondarySpaces),
    [workspaceSwitcher.secondarySpaces],
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
      setWorkspaceSwitcherValidated(true);
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
  }, [authSession?.user.userCode, currentLocale, dataSource, navigationRevision, setNavigationDomains, setNavigationMenuTree, setSelectedDomainCode, t]);

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
  }, [authSession?.user.userCode, currentLocale, dataSource, navigationDomains, navigationRevision, selectedDomainCode, setNavigationMenuTree]);

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
    writeFloatingState(MAIN_AGENT_WINDOW_STORAGE_KEY, mainAgentWindowState);
  }, [mainAgentWindowState]);

  useEffect(() => {
    window.dispatchEvent(new Event("tron:close-sidebar"));
    setRightDrawerOpen(false);
  }, [pathname]);

  useEffect(() => {
    setRightDrawerOpen(isRightDrawerViewport && Boolean(activeTab?.rightPanelOpen));
  }, [activeTab?.id, activeTab?.rightPanelOpen, isRightDrawerViewport]);

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
    nestedLayerSelectors: ["[data-slot='select-positioner']", "[data-tron-theme-menu]", "[data-tron-locale-menu]", "[data-tron-timezone-menu]"],
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

  function openRightDrawer() {
    closeFloatingPanels();
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

  function activateTabAtIndex(index: number) {
    const tabState = useTabStore.getState();
    const tab = tabState.tabs.filter((item) => !item.hide)[index];
    if (!tab) return false;
    tabState.setActiveTab(tab.id);
    navigate(tab.path);
  }

  function activateAdjacentTab(direction: -1 | 1) {
    const tabState = useTabStore.getState();
    const visibleTabs = tabState.tabs.filter((tab) => !tab.hide);
    const activeIndex = visibleTabs.findIndex((tab) => tab.id === tabState.activeTabId);
    if (activeIndex < 0 || visibleTabs.length < 2) return false;

    const nextIndex = (activeIndex + direction + visibleTabs.length) % visibleTabs.length;
    const nextTab = visibleTabs[nextIndex];
    tabState.setActiveTab(nextTab.id);
    navigate(nextTab.path);
    return;
  }

  function closeVisibleTabs(keepActive: boolean) {
    const tabState = useTabStore.getState();
    const activeTabId = tabState.activeTabId;
    const tabIds = tabState.tabs
      .filter((tab) => !tab.hide && tab.closable)
      .filter((tab) => !keepActive || tab.id !== activeTabId)
      .map((tab) => tab.id);
    if (!tabIds.length) return false;

    tabState.removeTabs(tabIds);
    const updatedTabState = useTabStore.getState();
    const nextActiveTab = updatedTabState.tabs.find(
      (tab) => tab.id === updatedTabState.activeTabId,
    );
    if (nextActiveTab && nextActiveTab.id !== DEFAULT_TAB_ID) {
      navigate(nextActiveTab.path);
    }
    return;
  }

  function canStartSidebarCreation() {
    return Boolean(activeWorkspaceCode) && !Array.from(
      document.querySelectorAll<HTMLElement>('[role="dialog"]'),
    ).some((dialog) => dialog.offsetWidth > 0 || dialog.offsetHeight > 0);
  }

  useShellShortcuts({
    closeAllTabs: () => closeVisibleTabs(false),
    closeCurrentTab: () => {
      if (!activeTabId) return false;
      const redirect = closeTab(activeTabId);
      if (redirect !== null) navigate(redirect);
    },
    closeOtherTabs: () => closeVisibleTabs(true),
    nextTab: () => activateAdjacentTab(1),
    activateTab1: () => activateTabAtIndex(0),
    activateTab2: () => activateTabAtIndex(1),
    activateTab3: () => activateTabAtIndex(2),
    activateTab4: () => activateTabAtIndex(3),
    activateTab5: () => activateTabAtIndex(4),
    activateTab6: () => activateTabAtIndex(5),
    activateTab7: () => activateTabAtIndex(6),
    activateTab8: () => activateTabAtIndex(7),
    activateTab9: () => activateTabAtIndex(8),
    newTopic: () => {
      if (!canStartSidebarCreation()) return false;
      window.dispatchEvent(new Event(CREATE_TOPIC_SHORTCUT_EVENT));
    },
    newDirectory: () => {
      if (!canStartSidebarCreation()) return false;
      window.dispatchEvent(new Event(ADD_DIRECTORY_SHORTCUT_EVENT));
    },
    newSession: () => {
      if (!canStartSidebarCreation()) return false;
      void createChatTab();
    },
    openNotifications: () => { void navigate("/notifications"); },
    previousTab: () => activateAdjacentTab(-1),
    toggleAgentWindow: () => handleAgentLauncherClick(),
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
  }, openSystemSettings);

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
    toggleUtilityPanel("domain");
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
    closeFloatingPanels();
    navigate("/settings/general", { state: { from: `${pathname}${search}` } });
  }

  function selectWorkspaceNode(space: WorkspaceSwitcherData["secondarySpaces"][number]) {
    const nextSecondarySpaceId = space.depth > 1 ? space.workspaceCode : "";
    const path = useTabStore.getState().switchWorkspace(space.workspaceCode);
    setApiWorkspaceCode(space.workspaceCode);
    selectPrimarySpaceInStore(space.primaryWorkspaceCode, nextSecondarySpaceId);
    setBottomPanel(false);
    if (path) navigate(path, { replace: true });
    toggleUtilityPanel("workspace");
    workspaceTriggerRef.current?.focus();
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
      "flex min-h-10 min-w-0 flex-1 cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 text-left transition-colors focus-visible:outline-2 focus-visible:outline-[var(--color-focus-ring)]",
      selected
        ? "bg-[var(--color-menu-selected-surface)] text-[var(--color-accent)]"
        : "hover:bg-[var(--color-menu-hover-surface)]",
    );
    const content = (
      <>
        <span className="min-w-0 flex-1 truncate text-sm font-medium">{space.name}</span>
        {selected ? <Check size={16} className="shrink-0" aria-hidden="true" /> : null}
      </>
    );

    return (
      <div key={space.workspaceCode} className="space-y-0.5">
        <div className="flex items-center gap-1" style={{ paddingInlineStart: `${nestingLevel}rem` }}>
          {hasChildren ? (
            <button
              className="flex size-6 shrink-0 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] hover:bg-[var(--color-menu-hover-surface)] hover:text-[var(--color-on-surface)]"
              type="button"
              aria-label={t(expanded ? "shell:aria.collapseNavigationItem" : "shell:aria.expandNavigationItem", { item: workspacePathLabel })}
              aria-expanded={expanded}
              onClick={() => toggleWorkspaceExpanded(space.workspaceCode)}
            >
              <ChevronDown className={cn("transition-transform", !expanded && "-rotate-90")} size={14} aria-hidden="true" />
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

  function renderScopeSwitcherHeader(title: string, settingsLabel: string, settingsPath: string) {
    return (
      <div className="mb-2 flex min-w-0 items-center justify-between gap-2 pl-1">
        <h2 className="min-w-0 text-sm font-semibold">{title}</h2>
        <Tooltip>
          <TooltipTrigger render={
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label={settingsLabel}
              onClick={() => {
                closeFloatingPanels();
                navigate(settingsPath, { state: { from: `${pathname}${search}` } });
              }}
            >
              <Settings size={16} aria-hidden="true" />
            </Button>
          } />
          <TooltipContent side="top" sideOffset={8}>{settingsLabel}</TooltipContent>
        </Tooltip>
      </div>
    );
  }

  function renderWorkspaceSwitcherContent() {
    return (
      <>
        {renderScopeSwitcherHeader(t("shell:workspace.switchTitle"), t("shell:workspace.manage"), "/admin/workspace")}
        {workspaceSwitcherLoading ? <p role="status" className="mb-2 px-1 text-xs text-[var(--color-on-surface-variant)]"><ShinyText>{t("shell:workspace.refreshing")}</ShinyText></p> : null}
        {workspaceTree.length ? (
          <div className="max-h-[min(50vh,24rem)] space-y-1 overflow-y-auto" role="group" aria-label={t("shell:workspace.selectorTitle")}>
            {workspaceTree.map((root) => renderWorkspaceTreeNode(root))}
          </div>
        ) : (
          <p role="status" className="px-2 py-3 text-sm text-[var(--color-on-surface-variant)]">{t("shell:workspace.noPrimarySpaces")}</p>
        )}
      </>
    );
  }

  function renderNavigationDomainSwitcherContent() {
    return (
      <>
        {renderScopeSwitcherHeader(t("shell:navigation.switchTitle"), t("shell:navigation.manage"), "/admin/navigation/domains")}
        <div className="max-h-[min(50vh,24rem)] space-y-1 overflow-y-auto" role="group" aria-label={t("shell:navigation.selectorTitle")}>
          {navigationLoading ? <p role="status" className="px-2 py-1 text-xs text-[var(--color-on-surface-variant)]"><ShinyText>{t("shell:navigation.refreshing")}</ShinyText></p> : null}
          {navigationDomains.length ? navigationDomains.map((domain) => {
            const selected = domain.domainCode === selectedDomainCode;
            return (
              <button
                aria-pressed={selected}
                className={cn(
                  "flex min-h-10 w-full min-w-0 items-center gap-2.5 rounded-lg px-2.5 py-2 text-left transition-colors focus-visible:outline-2 focus-visible:outline-[var(--color-focus-ring)]",
                  selected ? "bg-[var(--color-menu-selected-surface)] text-[var(--color-accent)]" : "hover:bg-[var(--color-menu-hover-surface)]",
                )}
                key={domain.domainCode}
                type="button"
                onClick={() => {
                  setSelectedDomainCode(domain.domainCode);
                  toggleUtilityPanel("domain");
                  domainTriggerRef.current?.focus();
                }}
              >
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-medium">{domain.name}</span>
                  {domain.description ? <span className="mt-0.5 block truncate text-xs text-[var(--color-on-surface-variant)]">{domain.description}</span> : null}
                </span>
                {selected ? <Check size={16} className="shrink-0" aria-hidden="true" /> : null}
              </button>
            );
          }) : <p role="status" className="px-2 py-3 text-sm text-[var(--color-on-surface-variant)]">{t("shell:navigation.noDomains")}</p>}
        </div>
      </>
    );
  }

  function renderUserPanelContent() {
    return (
      <>
        <div className="space-y-1 p-2">
          <LocaleMenu />
          <TimeZoneMenu />
          <ThemeMenu />
        </div>
      </>
    );
  }

  function renderSidebarUserSection(compact: boolean, mobile: boolean) {
    const displayName = authSession?.user.displayName || t("shell:utilityPanel.user");
    const email = authSession?.user.email ?? "";
    const initial = displayName.trim().charAt(0).toUpperCase() || "U";

    return (
      <div ref={userSectionRef} className={cn("relative flex w-full min-w-0 shrink-0", compact ? "flex-col items-center gap-1" : "items-center gap-1")}>
        <button
          ref={userMenuTriggerRef}
          className={cn(
            "flex min-h-14 min-w-0 cursor-pointer items-center rounded-lg py-1 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus-ring)]",
            compact
              ? "w-full justify-center px-1"
              : "flex-1 gap-2 px-2 hover:bg-[var(--color-hover-surface)]",
          )}
          type="button"
          aria-label={t("shell:aria.openUserMenu")}
          aria-expanded={utilityPanel === "user"}
          aria-haspopup="dialog"
          onClick={() => handleUtilityPanelClick("user")}
        >
          <span className={cn("flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-primary)] text-base font-semibold text-[var(--color-on-primary)]", compact ? "size-10" : "size-12")} aria-hidden="true">
            {avatarSrc ? <img alt="" className="size-full object-cover" onError={handleAvatarLoadError} src={avatarSrc} /> : initial}
          </span>
          {compact ? null : (
            <>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium text-[var(--color-on-surface)]">{displayName}</span>
                {email ? <span className="block truncate text-xs text-[var(--color-on-surface-variant)]">{email}</span> : null}
                {!email && (personAiPlan || remainingPoints !== null) ? <span className="mt-1 flex min-w-0 items-center gap-2 text-xs text-[var(--color-on-surface-variant)]">
                  {personAiPlan && <span className="shrink-0 rounded bg-[var(--color-hover-surface)] px-1.5 py-0.5 font-medium leading-none">{personAiPlan}</span>}
                  {remainingPoints !== null && <span className="truncate tabular-nums" title={t("shell:userMenu.remainingPoints")}>{remainingPoints}</span>}
                </span> : null}
              </span>
            </>
          )}
        </button>
        <RemoteAccessDialog compact={compact} />
        {utilityPanel === "user" ? createPortal(
          <div
            ref={utilityPanelRef}
            className={cn(
              "fixed z-[70] w-[min(20rem,calc(100vw-1rem))] overflow-visible rounded-lg border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
            )}
            style={userPanelStyle ?? { visibility: "hidden" }}
            role="dialog" data-theme-anchor="dialog" data-tron-sidebar-surface
            aria-label={t("shell:utilityPanel.user")}
          >
            <div className="flex min-w-0 items-center gap-2 px-3 py-2">
              <a
                aria-label="打开 PersonAI 官网"
                className="flex min-w-0 flex-1 items-center gap-3 rounded-md py-1 transition-colors hover:bg-[var(--color-hover-surface)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus-ring)]"
                href={PERSON_AI_WEBSITE_URL}
                rel="noreferrer"
                target="_blank"
              >
                <span className="flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-primary)] text-sm font-semibold text-[var(--color-on-primary)]" aria-hidden="true">
                  {avatarSrc ? <img alt="" className="size-full object-cover" onError={handleAvatarLoadError} src={avatarSrc} /> : initial}
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-sm font-medium text-[var(--color-on-surface)]">{displayName}</span>
                  {email ? <span className="block truncate text-xs text-[var(--color-on-surface-variant)]">{email}</span> : null}
                </span>
              </a>
              <button
                className="inline-flex size-8 shrink-0 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-focus-ring)]"
                type="button"
                aria-label={t("shell:aria.openSystemSettings")}
                title={t("shell:aria.openSystemSettings")}
                onClick={openSystemSettings}
              >
                <Settings size={16} aria-hidden="true" />
              </button>
              {userMenuContent ? <div className="shrink-0">{userMenuContent}</div> : null}
            </div>
            {authSession ? (
              <dl className="grid grid-cols-[1fr_1fr_auto] items-center gap-x-3 border-t border-[var(--color-card-border)] px-3 py-2 text-xs">
                <div className="min-w-0">
                  <dt className="text-[var(--color-on-surface-variant)]">{t("shell:userMenu.plan")}</dt>
                  <dd className="mt-0.5 truncate font-medium text-[var(--color-on-surface)]">{personAiPlan ?? "—"}</dd>
                </div>
                <div className="min-w-0">
                  <dt className="text-[var(--color-on-surface-variant)]">{t("shell:userMenu.remainingPoints")}</dt>
                  <dd className="mt-0.5 truncate font-medium tabular-nums text-[var(--color-on-surface)]">{remainingPoints ?? "—"}</dd>
                </div>
                <div className="flex justify-end"><dt className="sr-only">{t("shell:userMenu.checkin")}</dt><dd><CheckinButton /></dd></div>
              </dl>
            ) : null}
            <div className="border-t border-[var(--color-card-border)]">{renderUserPanelContent()}</div>
          </div>,
          document.body,
        ) : null}
      </div>
    );
  }

  function renderNavigationDomainSelector(compact: boolean, mobile: boolean) {
    const domainName = selectedNavigationDomain?.name ?? t("shell:navigation.defaultName");
    return (
      <div className="tron-scope-selector-row">
        <button
          ref={domainTriggerRef}
          className="tron-scope-selector tron-domain-selector"
          type="button"
          id="tron-domain-trigger"
          aria-controls={utilityPanel === "domain" ? "tron-domain-selector-panel" : undefined}
          aria-busy={navigationLoading}
          aria-expanded={utilityPanel === "domain"}
          aria-haspopup="dialog"
          aria-label={utilityPanel === "domain" ? t("shell:aria.closeDomainSelector") : t("shell:aria.openDomainSelector")}
          onClick={handleNavigationDomainSwitcherClick}
          title={domainName}
        >
          <span className="tron-scope-selector-copy"><span className="tron-scope-selector-label">{t("shell:navigation.scopeLabel")}</span>{!compact ? <span className="tron-scope-selector-name">{domainName}</span> : null}</span>
        </button>
        {utilityPanel === "domain" ? (
          <div
            ref={utilityPanelRef}
            id="tron-domain-selector-panel"
            className={cn(
              "tron-scope-popover absolute z-[70] max-h-[calc(100dvh-10rem)] w-[min(16rem,calc(100vw-2rem))] overflow-y-auto rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]",
              compact && !mobile ? "bottom-0 left-[calc(100%+8px)]" : "bottom-[calc(100%+8px)] left-0",
            )}
            role="dialog" data-theme-anchor="dialog" data-tron-sidebar-surface
            aria-label={t("shell:navigation.selectorTitle")}
            onKeyDown={(event) => {
              if (event.key !== "Escape") return;
              event.stopPropagation();
              toggleUtilityPanel("domain");
              domainTriggerRef.current?.focus();
            }}
          >
            {renderNavigationDomainSwitcherContent()}
          </div>
        ) : null}
      </div>
    );
  }

  function renderWorkspaceSelector(compact: boolean) {
    return (
      <div className="tron-scope-selector-row min-w-0">
        <button
          ref={workspaceTriggerRef}
          className="tron-scope-selector tron-workspace-selector"
          type="button"
          id="tron-workspace-trigger"
          aria-controls={utilityPanel === "workspace" ? "tron-workspace-selector-panel" : undefined}
          aria-busy={workspaceSwitcherLoading}
          aria-expanded={utilityPanel === "workspace"}
          aria-haspopup="dialog"
          aria-label={utilityPanel === "workspace" ? t("shell:aria.closeWorkspaceSelector") : t("shell:aria.openWorkspaceSelector")}
          onClick={handleWorkspaceSwitcherClick}
          title={selectedWorkspacePathLabel}
        >
          <span className="tron-scope-selector-copy"><span className="tron-scope-selector-label">{t("shell:workspace.shortScopeLabel")}</span>{!compact ? <span className="tron-scope-selector-name">{selectedWorkspaceName}</span> : null}</span>
        </button>
        {utilityPanel === "workspace" ? (
          <div
            ref={utilityPanelRef}
            id="tron-workspace-selector-panel"
            className={cn("tron-scope-popover absolute z-[80] max-h-[calc(100dvh-10rem)] w-[min(16rem,calc(100vw-2rem))] overflow-y-auto rounded-xl border border-[var(--color-card-border)] bg-[var(--color-elevated-surface)] p-3 text-sm shadow-[0_12px_30px_rgba(0,0,0,0.08)]", compact ? "bottom-0 left-[calc(100%+8px)]" : "bottom-[calc(100%+8px)] left-0")}
            role="dialog" data-theme-anchor="dialog"
            aria-label={t("shell:workspace.selectorTitle")}
            onKeyDown={(event) => {
              if (event.key !== "Escape") return;
              event.stopPropagation();
              toggleUtilityPanel("workspace");
              workspaceTriggerRef.current?.focus();
            }}
          >
            {renderWorkspaceSwitcherContent()}
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
      <AssistantPageContext.Provider value={getMainAgentPageContext}>
      <WorkspaceSpacesContext.Provider value={workspaceSpacesValue}>
        <SidebarProvider
          className="tron-skin-canvas tron-shell-frame"
          data-theme-part="app-sidebar-frame"
          data-theme-anchor="app-shell"
          data-tron-entry-ready=""
          open={!sidebarCollapsed}
          onOpenChange={(open) => setSidebarCollapsed(!open)}
        >
          <ChatNavigationProvider
            activeSessionCode={new URLSearchParams(search).get("sessionCode") ?? ""}
            navigateToPath={navigateToPath}
          >
            <WorkspaceLayout
              navigation={
                <TronSidebar
                  activeItem={activeTabId === DEFAULT_TAB_ID ? "" : activeItem}
                  activeHref={activeTabId === DEFAULT_TAB_ID ? undefined : pathname}
                  navigation={visibleNavigation}
                  openNavigationItems={openNavigationItems}
                  openSections={openSections}
                  renderWorkspaceSelector={renderWorkspaceSelector}
                  renderDomainSelector={renderNavigationDomainSelector}
                  renderFooter={renderSidebarUserSection}
                  setOpenNavigationItems={setOpenNavigationItems}
                  translate={(key, options) => t(key, options)}
                  onNavigate={handleSidebarNavigationClick}
                  onToggleSection={toggleSection}
                />
              }
              toolbar={
                <header className="tron-shell-toolbar" data-theme-part="app-toolbar">
                  <div className="flex h-full items-center justify-between gap-2">
                    <div className="flex min-w-0 flex-1 items-center gap-2">
                      <MobileNavigationEntry />
                      <TabBar className="min-w-0 flex-1" />
                    </div>
                    <div className="flex shrink-0 items-center gap-0.5">
                      {renderedTopbarActions}
                      <NotificationLauncher className="tron-shell-tool" />
                      <ToolbarControl label={t("shell:assistant.title")} shortcutAction="toggleAgentWindow">
                        <Button className="tron-shell-tool" size="icon" type="button" variant="ghost" aria-label={t("shell:assistant.title")} aria-expanded={agentOpen} aria-pressed={agentOpen} onClick={handleAgentLauncherClick}>
                          <Bot size={17} aria-hidden="true" />
                        </Button>
                      </ToolbarControl>
                      <ToolbarControl label={t(showBottomPanel ? "shell:aria.closeTerminal" : "shell:aria.openTerminal")} shortcutAction="toggleBottomPanel">
                        <Button
                          className="hidden min-[900px]:inline-flex"
                          size="icon"
                          type="button"
                          variant="ghost"
                          aria-label={t(showBottomPanel ? "shell:aria.closeTerminal" : "shell:aria.openTerminal")}
                          aria-expanded={showBottomPanel}
                          onClick={toggleBottomPanel}
                        >
                          {showBottomPanel ? <PanelBottomClose size={17} /> : <PanelBottomOpen size={17} />}
                        </Button>
                      </ToolbarControl>
                      <>
                        <ToolbarControl label={rightDrawerOpen ? t("shell:aria.closeRightSidebar") : t("shell:aria.openRightSidebar")} shortcutAction="toggleRightSidebar">
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
                        </ToolbarControl>
                        <ToolbarControl label={rightPanelOpen ? t("shell:aria.closeRightSidebar") : t("shell:aria.openRightSidebar")} shortcutAction="toggleRightSidebar">
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
                        </ToolbarControl>
                      </>
                    </div>
                  </div>
                </header>
              }
            >
              <Content
                sessionCode={terminalSessionCode}
                contentClassName={renderedContentClassName}
                showBottomPanel={showBottomPanel}
                changeBottomPanel={setBottomPanel}
              >
                {children}
              </Content>
            </WorkspaceLayout>
          </ChatNavigationProvider>

          {agentOpen ? (
            <MainAgentChatWindow
              currentUserDisplayName={authSession?.user.displayName || authSession?.user.username || ""}
              getPageContext={getMainAgentPageContext}
              workspaceCode={activeWorkspaceCode}
              windowState={{ ...mainAgentWindowState, open: true }}
              onWindowStateChange={handleMainAgentWindowStateChange}
            />
          ) : null}

          {rightDrawerOpen && isRightDrawerViewport ? (
            <>
              <button
                className="fixed inset-x-0 bottom-0 top-[var(--tron-shell-toolbar-height)] z-40 bg-[var(--color-backdrop-scrim)] lg:hidden"
                type="button"
                aria-label={t("shell:aria.closeRightSidebar")}
                data-testid="tron-mobile-right-sidebar-backdrop"
                onClick={closeRightDrawer}
              />
              <aside
                className="fixed bottom-2 right-2 top-[var(--tron-shell-toolbar-height)] z-50 rounded-xl flex w-[min(22rem,calc(100vw-1rem))] flex-col overflow-y-auto bg-[var(--color-elevated-surface)] px-5 py-5 text-[var(--color-on-surface)] shadow-[-12px_0_34px_rgba(15,23,42,0.12),0_18px_45px_rgba(15,23,42,0.12)] lg:hidden"
                data-theme-part="app-right-sidebar"
                data-theme-anchor="panel"
                data-testid="tron-mobile-right-sidebar"
                id="tron-mobile-right-sidebar"
              >
                <RightPanelContainer />
              </aside>
            </>
          ) : null}

        </SidebarProvider>
      </WorkspaceSpacesContext.Provider>
      </AssistantPageContext.Provider>
    </TronShellFrameContext.Provider>
  );
}

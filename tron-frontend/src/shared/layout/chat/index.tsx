import { ChevronDown, Plus } from "lucide-react";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { Create } from "./components/Create";
import SessionGroup from "./components/SessionGroup";
import RecentSessionGroup from "./components/RecentSessionGroup";
import { ProjectCreateDialog } from "./components/ProjectCreateDialog";
import { useChatSidebarData } from "./hooks/useChatSidebarData";
import { usePinnedProjectCodes } from "./hooks/usePinnedProjectCodes";
import { usePinnedSessionCodes } from "./hooks/usePinnedSessionCodes";
import type { ProjectRecord } from "@/domain/project";
import { CHAT_PANEL_WIDGET_KEYS } from "@/shared/constants/chatPanelMenu";

import { Button } from "@/components/ui/button";
import ActionButton from "./components/ActionButton";
import Item from "./components/Item";

const sidebarListPageSize = 5;

interface ChatWidgetMenu {
  menuCode: string;
  label: string;
  pageCode: string | null;
}

const ChatNavigationContext = createContext<((menu: ChatWidgetMenu) => ReactNode) | null>(null);

export function ChatNavigationWidget({ menu }: { menu: ChatWidgetMenu }) {
  const renderWidget = useContext(ChatNavigationContext);
  return renderWidget?.(menu) ?? null;
}

export function ChatNavigationProvider({
  activeSessionCode,
  navigateToPath,
  children,
}: {
  activeSessionCode: string;
  navigateToPath: (path: string) => void;
  children: ReactNode;
}) {
  const { t } = useTranslation("shell");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspaceCode =
    (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const { projects, loading, recentSessions, sessionsByProject } =
    useChatSidebarData(workspaceCode);
  const [runningSessionCode, setRunningSessionCode] = useState("");
  const [projectListExpanded, setProjectListExpanded] = useState(true);
  const [visibleProjectCount, setVisibleProjectCount] =
    useState(sidebarListPageSize);
  const [selectedProjectCode, setSelectedProjectCode] = useState<string>();
  const [projectCreateOpen, setProjectCreateOpen] = useState(false);
  const [editingProject, setEditingProject] = useState<ProjectRecord>();
  const { pinnedProjectCodes, togglePinned } = usePinnedProjectCodes();
  const { pinnedSessionCodes, togglePinnedSession, removePinnedSession } = usePinnedSessionCodes();
  const orderedProjects = useMemo(() => {
    const pinned = new Set(pinnedProjectCodes);
    return [...projects].sort(
      (left, right) =>
        Number(pinned.has(right.projectCode)) -
        Number(pinned.has(left.projectCode))
    );
  }, [pinnedProjectCodes, projects]);
  useEffect(() => {
    if (selectedProjectCode && !projects.some((project) => project.projectCode === selectedProjectCode)) {
      setSelectedProjectCode(undefined);
    }
  }, [projects, selectedProjectCode]);
  useEffect(() => {
    const handleRunStateChange = (event: Event) => {
      const detail = (event as CustomEvent<{ sessionCode?: string }>).detail;
      setRunningSessionCode(detail?.sessionCode ?? "");
    };
    window.addEventListener(
      "tron:chat-session-run-state",
      handleRunStateChange
    );
    return () =>
      window.removeEventListener(
        "tron:chat-session-run-state",
        handleRunStateChange
      );
  }, []);
  const selectedProject = orderedProjects.find(
    (project) => project.projectCode === selectedProjectCode
  );
  const createTargetProject = selectedProject;

  function renderMenuItem(menu: ChatWidgetMenu) {
    if (menu.pageCode === CHAT_PANEL_WIDGET_KEYS.newSession) {
      return (
        <Item
          key={menu.menuCode}
          left={
            <Create
              label={menu.label}
              project={createTargetProject}
              workspaceCode={workspaceCode}
            />
          }
        />
      );
    }
    if (menu.pageCode === CHAT_PANEL_WIDGET_KEYS.projects) {
      return (
        <div key={menu.menuCode} className="flex flex-col gap-1">
          <Item
            onClick={() => setProjectListExpanded((value) => !value)}
            className="group/projects flex items-center gap-1 rounded px-2 hover:bg-sidebar-accent"
            left={<span className="opacity-50">{menu.label}</span>}
            right={
              <div className="flex items-center gap-2">
                <ActionButton
                  className="opacity-0 transition-opacity group-hover/projects:opacity-100"
                  onClick={() => setProjectCreateOpen(true)}
                  aria-label={t("common:actions.create", "Create")}
                ><Plus size={16} aria-hidden="true" /></ActionButton>
                <ActionButton
                  className={
                    projectListExpanded
                      ? "transition-transform"
                      : "-rotate-90 transition-transform"
                  }
                  aria-label={t(projectListExpanded ? "common:actions.collapse" : "common:actions.expand", projectListExpanded ? "Collapse" : "Expand")}
                ><ChevronDown size={16} aria-hidden="true" /></ActionButton>
              </div>
            }
          />
          {projectListExpanded && orderedProjects.length > 0 ? (
            <div className="space-y-0.5" aria-busy={loading}>
              {orderedProjects
                .slice(0, visibleProjectCount)
                .map((project) => (
                  <SessionGroup
                    key={project.projectCode}
                    project={project}
                    runningSessionCode={runningSessionCode}
                    activeSessionCode={activeSessionCode}
                    onNavigate={navigateToPath}
                    selected={project.projectCode === selectedProjectCode}
                    sessions={
                      sessionsByProject.get(project.projectCode) ?? []
                    }
                    pinned={pinnedProjectCodes.includes(project.projectCode)}
                    pinnedSessionCodes={pinnedSessionCodes}
                    onToggleSessionPin={togglePinnedSession}
                    onRemoveSessionPin={removePinnedSession}
                    onEdit={() => setEditingProject(project)}
                    onPinnedChange={() => togglePinned(project.projectCode)}
                    onRefresh={() =>
                      window.dispatchEvent(new Event("tron:project-created"))
                    }
                    onSelect={setSelectedProjectCode}
                  />
                ))}
              {orderedProjects.length > visibleProjectCount ? (
                <Button
                  className="ml-2 w-fit"
                  size="sm"
                  type="button"
                  variant="ghost"
                  onClick={() =>
                    setVisibleProjectCount(
                      (count) => count + sidebarListPageSize
                    )
                  }
                >
                  {t("shell:chatSidebar.showMore")}
                </Button>
              ) : null}
            </div>
          ) : null}
        </div>
      );
    }
    if (menu.pageCode === CHAT_PANEL_WIDGET_KEYS.recents) {
      return recentSessions.length > 0 ? (
        <RecentSessionGroup
          label={menu.label}
          key={menu.menuCode}
          runningSessionCode={runningSessionCode}
          activeSessionCode={activeSessionCode}
          onNavigate={navigateToPath}
          sessions={recentSessions}
          pinnedSessionCodes={pinnedSessionCodes}
          onToggleSessionPin={togglePinnedSession}
          onRemoveSessionPin={removePinnedSession}
        />
      ) : null;
    }
    return null;
  }

  return (
    <ChatNavigationContext.Provider value={renderMenuItem}>
      {children}
      <ProjectCreateDialog
        open={projectCreateOpen || Boolean(editingProject)}
        project={editingProject}
        workspaceCode={workspaceCode}
        onOpenChange={(open) => {
          if (!open) {
            setProjectCreateOpen(false);
            setEditingProject(undefined);
          }
        }}
        onSaved={(project) => {
          setSelectedProjectCode(project.projectCode);
          window.dispatchEvent(new Event("tron:project-created"));
        }}
      />
    </ChatNavigationContext.Provider>
  );
}

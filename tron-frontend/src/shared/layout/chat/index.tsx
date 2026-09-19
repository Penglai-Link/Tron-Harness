import { memo, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, Plus } from "lucide-react";
import { useNavigate } from "react-router";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { Create } from "./components/Create";
import SessionGroup from "./components/SessionGroup";
import RecentSessionGroup from "./components/RecentSessionGroup";
import { ProjectCreateDialog } from "./components/ProjectCreateDialog";
import { useChatSidebarData } from "./hooks/useChatSidebarData";
import { usePinnedProjectCodes } from "./hooks/usePinnedProjectCodes";
import { useChatPanelMenuTree } from "./hooks/useChatPanelMenuTree";
import type { ProjectRecord } from "@/domain/project";
import { navigationPageRegistry } from "@/shared/constants/navigationPageRegistry";
import { CHAT_PANEL_WIDGET_KEYS } from "@/shared/constants/chatPanelMenu";
import type { NavigationRuntimeMenu } from "@/shared/types/Navigation";
import { Button } from "@/components/ui/button";
import IconButton from "./components/IconButton";
import Item from "./components/Item";

const sidebarListPageSize = 5;

function LinkItem({ menu }: { menu: NavigationRuntimeMenu }) {
  const navigate = useNavigate();
  const href =
    menu.menuType === "ExternalLink"
      ? menu.url ?? "#"
      : navigationPageRegistry[menu.pageCode ?? ""] ?? "#";

  if (menu.menuType === "ExternalLink") {
    return (
      <Item
        className="flex items-center gap-1"
        left={
          <a
            className="flex h-full w-full items-center text-sm"
            href={href}
            rel="noopener noreferrer"
            target="_blank"
          >
            {menu.label}
          </a>
        }
      />
    );
  }

  return (
    <Item
      className="flex items-center gap-1"
      left={<span className="text-sm">{menu.label}</span>}
      onClick={() => {
        if (href && href !== "#") navigate(href);
      }}
    />
  );
}

export const ChatSidebarPanel = memo(function ChatSidebarPanel({
  activeSessionCode,
  compact,
  navigateToPath,
}: {
  activeSessionCode: string;
  compact: boolean;
  navigateToPath: (path: string) => void;
}) {
  const { t } = useTranslation("shell");
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspaceCode =
    (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const { projects, loading, recentSessions, sessionsByProject } =
    useChatSidebarData(workspaceCode);
  const { children: menuChildren } = useChatPanelMenuTree();
  const [runningSessionCode, setRunningSessionCode] = useState("");
  const [projectListExpanded, setProjectListExpanded] = useState(true);
  const [visibleProjectCount, setVisibleProjectCount] =
    useState(sidebarListPageSize);
  const [selectedProjectCode, setSelectedProjectCode] = useState<string>();
  const [projectCreateOpen, setProjectCreateOpen] = useState(false);
  const [editingProject, setEditingProject] = useState<ProjectRecord>();
  const { pinnedProjectCodes, togglePinned } = usePinnedProjectCodes();
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
  if (compact) return null;
  const selectedProject = orderedProjects.find(
    (project) => project.projectCode === selectedProjectCode
  );
  const createTargetProject = selectedProject;

  function renderMenuItem(menu: NavigationRuntimeMenu) {
    if (menu.menuType === "Widget") {
      if (menu.pageCode === CHAT_PANEL_WIDGET_KEYS.newSession) {
        return (
          <Item
            key={menu.menuCode}
            left={
              <Create
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
                  <IconButton
                    className="opacity-0 transition-opacity group-hover/projects:opacity-100"
                    onClick={() => setProjectCreateOpen(true)}
                    icon={Plus}
                  />
                  <IconButton
                    className={
                      projectListExpanded
                        ? "transition-transform"
                        : "-rotate-90 transition-transform"
                    }
                    icon={ChevronDown}
                  />
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
            key={menu.menuCode}
            runningSessionCode={runningSessionCode}
            activeSessionCode={activeSessionCode}
            onNavigate={navigateToPath}
            sessions={recentSessions}
          />
        ) : null;
      }
      return null;
    }

    if (menu.menuType === "Internal" || menu.menuType === "ExternalLink") {
      return <LinkItem key={menu.menuCode} menu={menu} />;
    }

    return null;
  }

  return (
    <section className="border-b border-[var(--color-card-border)] p-2 flex flex-col gap-1">
      {menuChildren.map((menu) => renderMenuItem(menu))}
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
    </section>
  );
});

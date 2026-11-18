import { CREATE_TOPIC_SHORTCUT_EVENT, ADD_DIRECTORY_SHORTCUT_EVENT } from "@/shared/lib/keyboardShortcuts";
import { useCreateChatTab } from "./hooks/useCreateChatTab";
import { useDirectoryPicker } from "./hooks/useDirectoryPicker";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import SessionGroup from "./components/SessionGroup";
import RecentSessionGroup from "./components/RecentSessionGroup";
import { DirectorySection } from "./components/DirectorySection";
import { ProjectCreateDialog } from "./components/ProjectCreateDialog";
import { useChatSidebarData } from "./hooks/useChatSidebarData";
import { usePinnedProjectCodes } from "./hooks/usePinnedProjectCodes";
import { usePinnedSessionCodes } from "./hooks/usePinnedSessionCodes";
import type { ProjectRecord } from "@/domain/project";
import { CHAT_PANEL_WIDGET_KEYS } from "@/shared/constants/chatPanelMenu";

import { SectionHeader } from "./components/SectionHeader";
import { ShowMoreButton } from "./components/ShowMoreButton";

const sidebarListPageSize = 5;

interface ChatWidgetMenu {
  menuCode: string;
  label: string;
  pageCode: string | null;
}

interface ChatCreationActions {
  available: boolean;
  addingDirectory: boolean;
  createQuickChat: () => void;
  createTopic: () => void;
  addDirectory: () => void;
}
const ChatCreationContext = createContext<ChatCreationActions | null>(null);
export function useChatCreationActions() { return useContext(ChatCreationContext); }

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
  const { create: createQuickChat } = useCreateChatTab(workspaceCode);
  const directoryPicker = useDirectoryPicker(workspaceCode);
  const { projects, loading, recentSessions, sessionsByProject, sessionsByDirectory } =
    useChatSidebarData(workspaceCode);
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
    const createTopic = () => { if (workspaceCode) setProjectCreateOpen(true); };
    const addDirectory = () => { void directoryPicker.choose(); };
    window.addEventListener(CREATE_TOPIC_SHORTCUT_EVENT, createTopic);
    window.addEventListener(ADD_DIRECTORY_SHORTCUT_EVENT, addDirectory);
    return () => {
      window.removeEventListener(CREATE_TOPIC_SHORTCUT_EVENT, createTopic);
      window.removeEventListener(ADD_DIRECTORY_SHORTCUT_EVENT, addDirectory);
    };
  }, [workspaceCode, directoryPicker.choose]);
  function renderMenuItem(menu: ChatWidgetMenu) {
    if (menu.pageCode === CHAT_PANEL_WIDGET_KEYS.projects) {
      return (
        <div key={menu.menuCode} className="flex flex-col gap-1">
          <SectionHeader
            label={menu.label}
            open={projectListExpanded}
            onToggle={() => setProjectListExpanded((value) => !value)}
            onCreate={() => setProjectCreateOpen(true)}
            createDisabled={!workspaceCode}
            createLabel={t("pages:settingsShortcuts.shortcuts.newTopic.label")}
            shortcutAction="newTopic"
          />
          {projectListExpanded && orderedProjects.length > 0 ? (
            <div className="space-y-0.5" aria-busy={loading}>
              {orderedProjects
                .slice(0, visibleProjectCount)
                .map((project) => (
                  <SessionGroup
                    key={project.projectCode}
                    project={project}
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
                <ShowMoreButton
                  className="ml-2"
                  onClick={() =>
                    setVisibleProjectCount(
                      (count) => count + sidebarListPageSize
                    )
                  }
                />
              ) : null}
            </div>
          ) : null}
        </div>
      );
    }
    if (menu.pageCode === CHAT_PANEL_WIDGET_KEYS.directories) {
      return <DirectorySection key={menu.menuCode} label={menu.label} workspaceCode={workspaceCode}
        onAdd={() => void directoryPicker.choose()} adding={directoryPicker.saving}
        sessions={sessionsByDirectory} activeSessionCode={activeSessionCode}
        onNavigate={navigateToPath} pinnedSessionCodes={pinnedSessionCodes}
        onToggleSessionPin={togglePinnedSession} onRemoveSessionPin={removePinnedSession} />;
    }
    if (menu.pageCode === CHAT_PANEL_WIDGET_KEYS.recents) {
      return (
        <RecentSessionGroup
          label={menu.label}
          onCreate={createQuickChat}
          canCreate={Boolean(workspaceCode)}
          key={menu.menuCode}

          activeSessionCode={activeSessionCode}
          onNavigate={navigateToPath}
          sessions={recentSessions}
          pinnedSessionCodes={pinnedSessionCodes}
          onToggleSessionPin={togglePinnedSession}
          onRemoveSessionPin={removePinnedSession}
        />
      );
    }
    return null;
  }

  return (
    <ChatCreationContext.Provider value={{
      available: Boolean(workspaceCode),
      addingDirectory: directoryPicker.saving,
      createQuickChat,
      createTopic: () => setProjectCreateOpen(true),
      addDirectory: () => { void directoryPicker.choose(); },
    }}>
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
          setProjectListExpanded(true);
          window.dispatchEvent(new Event("tron:project-created"));
          if (!editingProject) navigateToPath(`/topics/${encodeURIComponent(project.projectCode)}`);
        }}
      />
    </ChatNavigationContext.Provider>
    </ChatCreationContext.Provider>
  );
}

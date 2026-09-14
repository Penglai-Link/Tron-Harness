import { useEffect, useMemo, useState } from "react";
import { ChevronDown, Plus } from "lucide-react";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { Create } from "./components/Create";
import SessionGroup from "./components/SessionGroup";
import { ProjectCreateDialog } from "./components/ProjectCreateDialog";
import { useChatSidebarData } from "./hooks/useChatSidebarData";
import { usePinnedProjectCodes } from "./hooks/usePinnedProjectCodes";
import type { ProjectRecord } from "@/domain/project";
import IconButton from "./components/IconButton";
import Item from "./components/Item";

export function ChatSidebarPanel({ compact }: { compact: boolean }) {
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspaceCode =
    (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const { projects, loading, sessionsByProject } =
    useChatSidebarData(workspaceCode);
  const [runningSessionCode, setRunningSessionCode] = useState("");
  const [projectListExpanded, setProjectListExpanded] = useState(true);
  const [selectedProjectCode, setSelectedProjectCode] = useState<string>();
  const [projectCreateOpen, setProjectCreateOpen] = useState(false);
  const [editingProject, setEditingProject] = useState<ProjectRecord>();
  const { pinnedProjectCodes, togglePinned } = usePinnedProjectCodes();
  const orderedProjects = useMemo(() => {
    const pinned = new Set(pinnedProjectCodes);
    return [...projects].sort(
      (left, right) =>
        Number(pinned.has(right.projectCode)) -
        Number(pinned.has(left.projectCode)),
    );
  }, [pinnedProjectCodes, projects]);
  useEffect(() => {
    if (
      !projects.some((project) => project.projectCode === selectedProjectCode)
    )
      setSelectedProjectCode(projects[0]?.projectCode);
  }, [projects, selectedProjectCode]);
  useEffect(() => {
    const handleRunStateChange = (event: Event) => {
      const detail = (event as CustomEvent<{ sessionCode?: string }>).detail;
      setRunningSessionCode(detail?.sessionCode ?? "");
    };
    window.addEventListener(
      "tron:chat-session-run-state",
      handleRunStateChange,
    );
    return () =>
      window.removeEventListener(
        "tron:chat-session-run-state",
        handleRunStateChange,
      );
  }, []);
  if (compact) return null;
  const selectedProject = orderedProjects.find(
    (project) => project.projectCode === selectedProjectCode,
  );
  return (
    <section className="border-b border-[var(--color-card-border)] p-2 flex flex-col gap-1">
      <Item
        left={
          <Create
            project={selectedProject}
            sessions={
              selectedProject
                ? sessionsByProject.get(selectedProject.projectCode) ?? []
                : []
            }
            workspaceCode={workspaceCode}
          />
        }
      />

      <Item
        onClick={() => setProjectListExpanded((value) => !value)}
        className="group/projects flex items-center gap-1 rounded px-2 hover:bg-sidebar-accent"
        left={<span className="opacity-50">项目</span>}
        right={
          <div className="flex items-center gap-2">
            <IconButton
              className="opacity-0 pointer-events-none transition-opacity group-hover/projects:opacity-100"
              onClick={() => setProjectCreateOpen(true)}
              icon={Plus}
            />
            <IconButton
              className={
                projectListExpanded
                  ? "transition-transform"
                  : "-rotate-90 transition-transform"
              }
              onClick={() => setProjectCreateOpen(true)}
              icon={ChevronDown}
            />
          </div>
        }
      />
      {projectListExpanded && orderedProjects.length > 0 ? (
        <div className="max-h-64 space-y-0.5 overflow-y-auto">
          {loading ? (
            <p className="px-2 py-1 text-xs text-sidebar-foreground/60">
              加载中
            </p>
          ) : (
            orderedProjects.map((project) => (
              <SessionGroup
                key={project.projectCode}
                project={project}
                runningSessionCode={runningSessionCode}
                selected={project.projectCode === selectedProjectCode}
                sessions={sessionsByProject.get(project.projectCode) ?? []}
                pinned={pinnedProjectCodes.includes(project.projectCode)}
                onEdit={() => setEditingProject(project)}
                onPinnedChange={() => togglePinned(project.projectCode)}
                onRefresh={() =>
                  window.dispatchEvent(new Event("tron:project-created"))
                }
                onSelect={setSelectedProjectCode}
              />
            ))
          )}
        </div>
      ) : null}
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
}

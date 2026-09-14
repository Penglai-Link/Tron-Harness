import { useEffect, useMemo, useState } from "react";
import { FolderPlus, MoreHorizontal } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useWorkspaceSpaces } from "@/shared/hooks/useWorkspaceSpaces";
import { NewChatButton } from "./components/NewChatButton";
import { ProjectSessionGroup } from "./components/ProjectSessionGroup";
import { ProjectCreateDialog } from "./components/ProjectCreateDialog";
import { useChatSidebarData } from "./hooks/useChatSidebarData";
import { usePinnedProjectCodes } from "./hooks/usePinnedProjectCodes";
import type { ProjectRecord } from "@/domain/project";

export function ChatSidebarPanel({ compact }: { compact: boolean }) {
  const { selectedPrimarySpace, selectedSecondarySpace } = useWorkspaceSpaces();
  const workspaceCode =
    (selectedSecondarySpace ?? selectedPrimarySpace)?.workspaceCode ?? "";
  const { projects, loading, sessionsByProject } =
    useChatSidebarData(workspaceCode);
  const [runningSessionCode, setRunningSessionCode] = useState("");
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
    <section>
      <div className="px-2 hover:bg-sidebar-accent rounded">
        <NewChatButton
          project={selectedProject}
          sessions={
            selectedProject
              ? sessionsByProject.get(selectedProject.projectCode) ?? []
              : []
          }
          workspaceCode={workspaceCode}
        />
      </div>
      <div className="flex items-center gap-1 justify-between pl-2">
        <span className="text-[14px] opacity-45">项目</span>
        <Button
          type="button"
          variant="link"
          onClick={() => setProjectCreateOpen(true)}
        >
          <FolderPlus size={12} />
        </Button>
      </div>
      {orderedProjects.length > 0 ? (
        <div className="max-h-64 space-y-0.5 overflow-y-auto">
          {loading ? (
            <p className="px-2 py-1 text-xs text-sidebar-foreground/60">
              加载中
            </p>
          ) : (
            orderedProjects.map((project) => (
              <ProjectSessionGroup
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

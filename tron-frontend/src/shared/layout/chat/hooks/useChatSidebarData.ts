import { useEffect, useMemo, useState } from "react";
import { useChatSessionGroupList } from "@/domain/chat/hooks";
import { useProjectList } from "@/domain/project";

export function useChatSidebarData(workspaceCode: string) {
  const projects = useProjectList(workspaceCode);
  const [refreshKey, setRefreshKey] = useState(0);
  const sessionGroups = useChatSessionGroupList(workspaceCode, refreshKey);
  useEffect(() => {
    const refreshSessions = () => setRefreshKey((value) => value + 1);
    const refreshProjectAndSessions = () => {
      projects.refresh();
      refreshSessions();
    };
    window.addEventListener("tron:chat-session-created", refreshSessions);
    window.addEventListener("tron:chat-session-archived", refreshSessions);
    window.addEventListener("tron:chat-session-updated", refreshSessions);
    window.addEventListener("tron:project-created", refreshProjectAndSessions);
    return () => {
      window.removeEventListener("tron:chat-session-created", refreshSessions);
      window.removeEventListener("tron:chat-session-archived", refreshSessions);
      window.removeEventListener("tron:chat-session-updated", refreshSessions);
      window.removeEventListener("tron:project-created", refreshProjectAndSessions);
    };
  }, [projects.refresh]);
  const sessionsByProject = useMemo(() => {
    const entries = sessionGroups
      .flatMap((group) => group.sessions)
      .filter(
        (session) => session.sourceType?.toUpperCase() === "MANUAL"
      );
    return new Map(
      projects.projects.map((project) => [
        project.projectCode,
        entries.filter(
          (session) => session.projectCode === project.projectCode
        ),
      ])
    );
  }, [projects.projects, sessionGroups]);
  return { ...projects, sessionsByProject };
}

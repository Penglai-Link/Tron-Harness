import { useEffect, useMemo, useState } from "react";
import type { ChatSession } from "@/domain/chat/model";
import { useChatSessionGroupList } from "@/domain/chat/hooks";
import { useProjectList } from "@/domain/project";

export function useChatSidebarData(workspaceCode: string) {
  const projects = useProjectList(workspaceCode);
  const [refreshKey, setRefreshKey] = useState(0);
  const [createdSessions, setCreatedSessions] = useState<ChatSession[]>([]);
  const sessionGroups = useChatSessionGroupList(workspaceCode, refreshKey);
  useEffect(() => {
    const refreshSessions = () => setRefreshKey((value) => value + 1);
    const handleSessionCreated = (event: Event) => {
      const session = (event as CustomEvent<ChatSession | undefined>).detail;
      if (!session) {
        refreshSessions();
        return;
      }
      setCreatedSessions((current) => [
        { ...session, sourceType: session.sourceType ?? "Manual" },
        ...current.filter((item) => item.sessionCode !== session.sessionCode),
      ]);
    };
    const handleSessionArchived = (event: Event) => {
      const sessionCode = (event as CustomEvent<{ sessionCode?: string }>).detail?.sessionCode;
      if (sessionCode) {
        setCreatedSessions((current) => current.filter((session) => session.sessionCode !== sessionCode));
      }
      refreshSessions();
    };
    const refreshProjectAndSessions = () => {
      projects.refresh();
      refreshSessions();
    };
    window.addEventListener("tron:chat-session-created", handleSessionCreated);
    window.addEventListener("tron:chat-session-archived", handleSessionArchived);
    window.addEventListener("tron:chat-session-updated", refreshSessions);
    window.addEventListener("tron:project-created", refreshProjectAndSessions);
    return () => {
      window.removeEventListener("tron:chat-session-created", handleSessionCreated);
      window.removeEventListener("tron:chat-session-archived", handleSessionArchived);
      window.removeEventListener("tron:chat-session-updated", refreshSessions);
      window.removeEventListener("tron:project-created", refreshProjectAndSessions);
    };
  }, [projects.refresh]);
  const sessions = useMemo(() => {
    const listedSessions = sessionGroups.flatMap((group) => group.sessions);
    const listedCodes = new Set(listedSessions.map((session) => session.sessionCode));
    return [
      ...createdSessions.filter((session) => (
        session.workspaceCode === workspaceCode && !listedCodes.has(session.sessionCode)
      )),
      ...listedSessions,
    ];
  }, [createdSessions, sessionGroups, workspaceCode]);
  useEffect(() => {
    const listedCodes = new Set(
      sessionGroups.flatMap((group) => group.sessions.map((session) => session.sessionCode)),
    );
    if (createdSessions.some((session) => listedCodes.has(session.sessionCode))) {
      setCreatedSessions((current) => current.filter((session) => !listedCodes.has(session.sessionCode)));
    }
  }, [createdSessions, sessionGroups]);
  const sessionsByProject = useMemo(() => {
    const entries = sessions
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
  }, [projects.projects, sessions]);
  const recentSessions = useMemo(() => {
    const entries = sessions
      .filter(
        (session) =>
          session.sourceType?.toUpperCase() === "MANUAL" &&
          !session.projectCode,
      );
    return [...entries].sort(
      (left, right) =>
        new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime(),
    );
  }, [sessions]);
  return { ...projects, recentSessions, sessionsByProject };
}

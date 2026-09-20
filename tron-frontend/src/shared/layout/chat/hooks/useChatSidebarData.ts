import { useEffect, useMemo, useState } from "react";
import type { ChatSession } from "@/domain/chat/model";
import { useChatSessionGroupList } from "@/domain/chat/hooks";
import { useProjectList } from "@/domain/project";

export function useChatSidebarData(workspaceCode: string) {
  const projects = useProjectList(workspaceCode);
  const [refreshKey, setRefreshKey] = useState(0);
  const [createdSessions, setCreatedSessions] = useState<ChatSession[]>([]);
  const [sessionOverrides, setSessionOverrides] = useState<Record<string, ChatSession>>({});
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
        setSessionOverrides((current) => {
          if (!(sessionCode in current)) return current;
          const next = { ...current };
          delete next[sessionCode];
          return next;
        });
      }
      refreshSessions();
    };
    const handleSessionUpdated = (event: Event) => {
      const session = (event as CustomEvent<{ session?: ChatSession }>).detail?.session;
      if (session) {
        setSessionOverrides((current) => ({ ...current, [session.sessionCode]: session }));
        return;
      }
      refreshSessions();
    };
    const refreshProjectAndSessions = () => {
      projects.refresh();
      refreshSessions();
    };
    window.addEventListener("tron:chat-session-created", handleSessionCreated);
    window.addEventListener("tron:chat-session-archived", handleSessionArchived);
    window.addEventListener("tron:chat-session-updated", handleSessionUpdated);
    window.addEventListener("tron:project-created", refreshProjectAndSessions);
    return () => {
      window.removeEventListener("tron:chat-session-created", handleSessionCreated);
      window.removeEventListener("tron:chat-session-archived", handleSessionArchived);
      window.removeEventListener("tron:chat-session-updated", handleSessionUpdated);
      window.removeEventListener("tron:project-created", refreshProjectAndSessions);
    };
  }, [projects.refresh]);
  const sessions = useMemo(() => {
    const listedSessions = sessionGroups.flatMap((group) => group.sessions).map((session) => (
      sessionOverrides[session.sessionCode]
        ? { ...session, ...sessionOverrides[session.sessionCode] }
        : session
    ));
    const listedCodes = new Set(listedSessions.map((session) => session.sessionCode));
    return [
      ...createdSessions.filter((session) => (
        session.workspaceCode === workspaceCode && !listedCodes.has(session.sessionCode)
      )),
      ...listedSessions,
    ];
  }, [createdSessions, sessionGroups, sessionOverrides, workspaceCode]);
  useEffect(() => {
    const listedSessions = sessionGroups.flatMap((group) => group.sessions);
    if (!listedSessions.length) return;
    const listedByCode = new Map(listedSessions.map((session) => [session.sessionCode, session]));
    setSessionOverrides((current) => {
      let changed = false;
      const next = { ...current };
      Object.entries(current).forEach(([sessionCode, override]) => {
        if (listedByCode.get(sessionCode)?.agentCode === override.agentCode) {
          delete next[sessionCode];
          changed = true;
        }
      });
      return changed ? next : current;
    });
  }, [sessionGroups]);
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

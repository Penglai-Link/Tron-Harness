import {
  archiveMockChatSession,
  createMockChatSession,
  deleteMockChatSession,
  getChatSessionGroups,
  updateMockChatSession,
} from "@/domain/chat/mock/data";
import { useEffect, useMemo } from "react";
import { chatApiEndpoints } from "@/domain/chat/api";
import type { ChatSession, ChatSessionGroup, CreateChatSessionInput, UpdateChatSessionInput } from "@/domain/chat/model";
import { requestApi } from "@/shared/api/httpClient";
import { getServiceDataSource, useServiceQuery } from "@/shared/api/serviceConfig";

const recentlyCreatedSessions = new Map<string, ChatSession>();

export function rememberCreatedChatSession(session: ChatSession) {
  recentlyCreatedSessions.set(session.sessionCode, session);
}

function mergeRecentlyCreatedSessions(
  groups: ChatSessionGroup[],
  workspaceCode: string,
) {
  const pendingSessions = [...recentlyCreatedSessions.values()].filter(
    (session) => session.workspaceCode === workspaceCode,
  );
  if (!pendingSessions.length) return groups;

  const listedCodes = new Set(groups.flatMap((group) => group.sessions.map((session) => session.sessionCode)));
  const sessionsToMerge = pendingSessions.filter((session) => !listedCodes.has(session.sessionCode));
  if (!sessionsToMerge.length) return groups;

  const nextGroups = groups.map((group) => ({
    ...group,
    sessions: [...group.sessions],
  }));
  const manualGroup = nextGroups.find(
    (group) => !group.taskCode && group.sourceType?.toUpperCase() === "MANUAL",
  );
  if (manualGroup) {
    manualGroup.sessions = [...sessionsToMerge, ...manualGroup.sessions];
    return nextGroups;
  }

  return [
    {
      taskCode: null,
      taskTitle: "Chat",
      sourceType: "Manual",
      sourceCode: null,
      sessions: sessionsToMerge,
    },
    ...nextGroups,
  ];
}

export function useChatSessionGroupList(workspaceCode: string, refreshKey = 0) {
  const groups = useServiceQuery<ChatSessionGroup[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(workspaceCode),
    endpoint: chatApiEndpoints.listChatSessionGroups,
    mockData: () => getChatSessionGroups(),
    deps: [workspaceCode],
    params: { workspaceCode },
    refreshKey,
  });

  const mergedGroups = useMemo(
    () => mergeRecentlyCreatedSessions(groups, workspaceCode),
    [groups, workspaceCode],
  );

  useEffect(() => {
    const listedCodes = new Set(groups.flatMap((group) => group.sessions.map((session) => session.sessionCode)));
    listedCodes.forEach((sessionCode) => recentlyCreatedSessions.delete(sessionCode));
  }, [groups]);

  return mergedGroups;
}

export async function createChatSession(input: CreateChatSessionInput): Promise<ChatSession> {
  let session: ChatSession;
  if (getServiceDataSource("chat") === "mock") {
    session = createMockChatSession(input);
  } else {
    session = await requestApi<ChatSession>(chatApiEndpoints.createChatSession, {
      body: input,
    });
  }

  rememberCreatedChatSession(session);
  return session;
}

export async function archiveChatSession(sessionCode: string): Promise<ChatSession> {
  if (getServiceDataSource("chat") === "mock") {
    const archived = archiveMockChatSession(sessionCode);
    recentlyCreatedSessions.delete(sessionCode);
    return archived;
  }

  const archived = await requestApi<ChatSession>(chatApiEndpoints.archiveChatSession, {
    pathParams: { sessionCode },
  });
  recentlyCreatedSessions.delete(sessionCode);
  return archived;
}

export async function deleteArchivedChatSession(sessionCode: string): Promise<void> {
  if (getServiceDataSource("chat") === "mock") {
    deleteMockChatSession(sessionCode);
    return;
  }

  await requestApi(chatApiEndpoints.deleteChatSession, {
    pathParams: { sessionCode },
  });
}

export function updateChatSession(sessionCode: string, input: UpdateChatSessionInput): Promise<ChatSession> {
  if (getServiceDataSource("chat") === "mock") {
    return Promise.resolve(updateMockChatSession(sessionCode, input));
  }

  return requestApi<ChatSession>(chatApiEndpoints.updateChatSession, {
    body: input,
    pathParams: { sessionCode },
  });
}

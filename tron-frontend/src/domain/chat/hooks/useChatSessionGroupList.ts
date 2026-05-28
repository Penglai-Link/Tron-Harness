import {
  archiveMockChatSession,
  createMockChatSession,
  deleteMockChatSession,
  getChatSessionGroups,
  updateMockChatSession,
} from "@/domain/chat/mock/data";
import { chatApiEndpoints } from "@/domain/chat/api";
import type { ChatSession, ChatSessionGroup, CreateChatSessionInput, UpdateChatSessionInput } from "@/domain/chat/model";
import { requestApi } from "@/shared/api/httpClient";
import { getServiceDataSource, useServiceQuery } from "@/shared/api/serviceConfig";

export function useChatSessionGroupList(workspaceCode: string) {
  return useServiceQuery<ChatSessionGroup[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(workspaceCode),
    endpoint: chatApiEndpoints.listChatSessionGroups,
    mockData: () => getChatSessionGroups(),
    deps: [workspaceCode],
    params: { workspaceCode },
  });
}

export function createChatSession(input: CreateChatSessionInput): Promise<ChatSession> {
  if (getServiceDataSource("chat") === "mock") {
    return Promise.resolve(createMockChatSession(input));
  }

  return requestApi<ChatSession>(chatApiEndpoints.createChatSession, {
    body: input,
  });
}

export function archiveChatSession(sessionCode: string): Promise<ChatSession> {
  if (getServiceDataSource("chat") === "mock") {
    return Promise.resolve(archiveMockChatSession(sessionCode));
  }

  return requestApi<ChatSession>(chatApiEndpoints.archiveChatSession, {
    pathParams: { sessionCode },
  });
}

export function deleteChatSession(sessionCode: string): Promise<ChatSession> {
  if (getServiceDataSource("chat") === "mock") {
    return Promise.resolve(deleteMockChatSession(sessionCode));
  }

  return requestApi<ChatSession>(chatApiEndpoints.deleteChatSession, {
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

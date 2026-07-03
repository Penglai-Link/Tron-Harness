import { chatApiEndpoints } from "@/domain/chat/api";
import {
  createMockChatCompaction,
  getMockChatLatestCompaction,
  getMockChatSessionGoal,
  getMockChatSessionLatestGoal,
  getMockChatSessionTodos,
  setMockChatSessionGoal,
  transitionMockChatSessionGoal,
  updateMockChatSessionGoal,
} from "@/domain/chat/mock/data";
import type {
  ChatSessionCompaction,
  ChatSessionGoalCommand,
  ChatSessionGoalView,
  ChatSessionTodo,
  CreateChatCompactionInput,
  SetChatSessionGoalInput,
  UpdateChatSessionGoalInput,
} from "@/domain/chat/model";
import { requestApi } from "@/shared/api/httpClient";
import {
  getServiceDataSource,
  useServiceQuery,
  useServiceQueryState,
} from "@/shared/api/serviceConfig";

export function useChatSessionTodosState(sessionCode: string, refreshKey = 0) {
  return useServiceQueryState<ChatSessionTodo[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getChatSessionTodos,
    mockData: () => getMockChatSessionTodos(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export function useChatSessionTodos(sessionCode: string, refreshKey = 0) {
  return useServiceQuery<ChatSessionTodo[]>({
    apiInitialData: () => [],
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getChatSessionTodos,
    mockData: () => getMockChatSessionTodos(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export function useChatLatestCompactionState(sessionCode: string, refreshKey = 0) {
  return useServiceQueryState<ChatSessionCompaction | null>({
    apiInitialData: () => null,
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getLatestChatCompaction,
    mockData: () => getMockChatLatestCompaction(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export function useChatLatestCompaction(sessionCode: string, refreshKey = 0) {
  return useServiceQuery<ChatSessionCompaction | null>({
    apiInitialData: () => null,
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getLatestChatCompaction,
    mockData: () => getMockChatLatestCompaction(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export async function createChatCompaction(
  input: CreateChatCompactionInput,
): Promise<ChatSessionCompaction> {
  if (getServiceDataSource("chat") === "mock") {
    return createMockChatCompaction(input);
  }
  return requestApi<ChatSessionCompaction>(chatApiEndpoints.createChatCompaction, {
    body: input,
  });
}

export function useChatSessionGoalState(sessionCode: string, refreshKey = 0) {
  return useServiceQueryState<ChatSessionGoalView | null>({
    apiInitialData: () => null,
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getChatSessionGoal,
    mockData: () => getMockChatSessionGoal(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export function useChatSessionGoal(sessionCode: string, refreshKey = 0) {
  return useServiceQuery<ChatSessionGoalView | null>({
    apiInitialData: () => null,
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getChatSessionGoal,
    mockData: () => getMockChatSessionGoal(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export function useChatSessionLatestGoalState(sessionCode: string, refreshKey = 0) {
  return useServiceQueryState<ChatSessionGoalView | null>({
    apiInitialData: () => null,
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getChatSessionLatestGoal,
    mockData: () => getMockChatSessionLatestGoal(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export function useChatSessionLatestGoal(sessionCode: string, refreshKey = 0) {
  return useServiceQuery<ChatSessionGoalView | null>({
    apiInitialData: () => null,
    domain: "chat",
    enabled: Boolean(sessionCode),
    endpoint: chatApiEndpoints.getChatSessionLatestGoal,
    mockData: () => getMockChatSessionLatestGoal(sessionCode),
    deps: [sessionCode],
    params: { sessionCode },
    refreshKey,
  });
}

export async function setChatSessionGoal(
  input: SetChatSessionGoalInput,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return setMockChatSessionGoal(input);
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.setChatSessionGoal, {
    body: input,
  });
}

export async function updateChatSessionGoal(
  input: UpdateChatSessionGoalInput,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return updateMockChatSessionGoal(input);
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.updateChatSessionGoal, {
    body: input,
  });
}

export async function pauseChatSessionGoal(
  input: ChatSessionGoalCommand,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return transitionMockChatSessionGoal(input, "PAUSED");
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.pauseChatSessionGoal, {
    body: input,
  });
}

export async function resumeChatSessionGoal(
  input: ChatSessionGoalCommand,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return transitionMockChatSessionGoal(input, "ACTIVE");
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.resumeChatSessionGoal, {
    body: input,
  });
}

export async function cancelChatSessionGoal(
  input: ChatSessionGoalCommand,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return transitionMockChatSessionGoal(input, "CANCELLED");
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.cancelChatSessionGoal, {
    body: input,
  });
}

export async function clearChatSessionGoal(
  input: ChatSessionGoalCommand,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return transitionMockChatSessionGoal(input, "CLEARED");
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.clearChatSessionGoal, {
    body: input,
  });
}

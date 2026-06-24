import { chatApiEndpoints } from "@/domain/chat/api";
import {
  addMockChatSessionGoalCriterion,
  createMockChatCompaction,
  getMockChatLatestCompaction,
  getMockChatSessionGoal,
  getMockChatSessionTodos,
  removeMockChatSessionGoalCriterion,
  setMockChatSessionGoal,
  transitionMockChatSessionGoal,
} from "@/domain/chat/mock/data";
import type {
  AddChatSessionGoalCriterionInput,
  ChatSessionCompaction,
  ChatSessionGoalCommand,
  ChatSessionGoalView,
  ChatSessionTodo,
  CreateChatCompactionInput,
  RemoveChatSessionGoalCriterionInput,
  SetChatSessionGoalInput,
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

export async function addChatSessionGoalCriterion(
  input: AddChatSessionGoalCriterionInput,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return addMockChatSessionGoalCriterion(input);
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.addChatSessionGoalCriterion, {
    body: input,
  });
}

export async function removeChatSessionGoalCriterion(
  input: RemoveChatSessionGoalCriterionInput,
): Promise<ChatSessionGoalView> {
  if (getServiceDataSource("chat") === "mock") {
    return removeMockChatSessionGoalCriterion(input);
  }
  return requestApi<ChatSessionGoalView>(chatApiEndpoints.removeChatSessionGoalCriterion, {
    body: input,
  });
}

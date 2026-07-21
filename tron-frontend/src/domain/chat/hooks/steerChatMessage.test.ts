import { beforeEach, describe, expect, it, vi } from "vitest";

vi.hoisted(() => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      localStorage: {
        getItem: () => null,
        setItem: () => undefined,
      },
      location: { origin: "http://localhost" },
    },
  });
});

import { chatApiEndpoints } from "@/domain/chat/api";
import type {
  ChatSteerResult,
  GetActiveChatRunResult,
  SteerChatMessageInput,
  StopChatRunInput,
  StopChatRunResult,
} from "@/domain/chat/model";
import { ApiError, requestApi } from "@/shared/api/httpClient";
import { useServiceQueryState } from "@/shared/api/serviceConfig";
import {
  getChatActiveRun,
  steerChatMessage,
  stopChatRun,
  streamChatMessage,
  useChatActiveRunState,
} from "./useChatMessageListBySession";

vi.mock("@/shared/api/httpClient", async (importOriginal) => {
  const original = await importOriginal<typeof import("@/shared/api/httpClient")>();
  return {
    ...original,
    requestApi: vi.fn(),
  };
});

vi.mock("@/shared/api/serviceConfig", () => ({
  getServiceDataSource: vi.fn(() => "api"),
  useServiceQuery: vi.fn(),
  useServiceQueryState: vi.fn(),
}));

describe("steerChatMessage", () => {
  beforeEach(() => {
    vi.mocked(requestApi).mockReset();
  });

  it("posts only the steer contract fields and forwards the abort signal", async () => {
    const input: SteerChatMessageInput = {
      attachments: [],
      content: "Use the database result instead",
      expectedRunCode: "RUN-1",
      sessionCode: "SESSION-1",
    };
    const response: ChatSteerResult = {
      message: {
        actor: "User",
        attachments: [],
        channel: "MAIN_AGENT",
        content: input.content,
        createdAt: "2026-07-17T00:00:00Z",
        id: "7483561976019288065",
        messageCode: "MESSAGE-1",
        role: "USER",
        runCode: "RUN-1",
        sessionCode: "SESSION-1",
      },
      runCode: "RUN-1",
      sessionCode: "SESSION-1",
    };
    const signal = new AbortController().signal;
    vi.mocked(requestApi).mockResolvedValue(response);

    await expect(steerChatMessage(input, signal)).resolves.toEqual(response);
    expect(requestApi).toHaveBeenCalledWith(chatApiEndpoints.steerChatMessage, {
      body: input,
      signal,
    });
    expect(Object.keys(input).sort()).toEqual([
      "attachments",
      "content",
      "expectedRunCode",
      "sessionCode",
    ]);
  });

  it("preserves the HTTP status and service code for a rejected stream", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(
      JSON.stringify({
        code: "CHAT_RUN_ALREADY_ACTIVE",
        data: {},
        message: "session already has an active agent run",
      }),
      {
        headers: { "Content-Type": "application/json" },
        status: 409,
      },
    ));

    const promise = streamChatMessage(
      {
        agentCode: "main-agent",
        appCode: null,
        createdAt: "2026-07-17T00:00:00Z",
        id: 1,
        lastRunCode: null,
        lastRunStatus: null,
        sessionCode: "SESSION-1",
        status: "ACTIVE",
        taskCode: null,
        title: "Chat",
        updatedAt: "2026-07-17T00:00:00Z",
        workspaceCode: "WORKSPACE-1",
      },
      {
        agentCode: "main-agent",
        content: "Continue",
        modelCode: "MODEL-1",
        workspaceCode: "WORKSPACE-1",
      },
    );

    await expect(promise).rejects.toMatchObject({
      code: "CHAT_RUN_ALREADY_ACTIVE",
      message: "session already has an active agent run",
      status: 409,
    } satisfies Partial<ApiError>);
    fetchMock.mockRestore();
  });

  it("preserves the service code from a RUN_ERROR stream event", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(
      [
        "event: RUN_ERROR",
        "data: {\"type\":\"RUN_ERROR\",\"code\":\"CHAT_RUN_ALREADY_ACTIVE\",\"message\":\"session already has an active agent run\"}",
        "",
        "",
      ].join("\n"),
      {
        headers: { "Content-Type": "text/event-stream" },
        status: 200,
      },
    ));

    const promise = streamChatMessage(
      {
        agentCode: "main-agent",
        appCode: null,
        createdAt: "2026-07-17T00:00:00Z",
        id: 1,
        lastRunCode: null,
        lastRunStatus: null,
        sessionCode: "SESSION-1",
        status: "ACTIVE",
        taskCode: null,
        title: "Chat",
        updatedAt: "2026-07-17T00:00:00Z",
        workspaceCode: "WORKSPACE-1",
      },
      {
        agentCode: "main-agent",
        content: "Continue",
        modelCode: "MODEL-1",
        workspaceCode: "WORKSPACE-1",
      },
    );

    await expect(promise).rejects.toMatchObject({
      code: "CHAT_RUN_ALREADY_ACTIVE",
      message: "session already has an active agent run",
      status: 200,
    } satisfies Partial<ApiError>);
    fetchMock.mockRestore();
  });
});

describe("active chat run control API", () => {
  beforeEach(() => {
    vi.mocked(requestApi).mockReset();
    vi.mocked(useServiceQueryState).mockReset();
  });

  it("configures the active-run hook with the exact GET session query contract", () => {
    const response: GetActiveChatRunResult = {
      activeRun: null,
      sessionCode: "SESSION-1",
    };
    const retry = vi.fn();
    vi.mocked(useServiceQueryState).mockReturnValue({
      data: response,
      error: null,
      loading: false,
      retry,
    });

    expect(useChatActiveRunState("SESSION-1", 7)).toEqual({
      data: response,
      error: null,
      loading: false,
      retry,
    });
    expect(useServiceQueryState).toHaveBeenCalledWith({
      apiInitialData: expect.any(Function),
      deps: ["SESSION-1"],
      domain: "chat",
      enabled: true,
      endpoint: chatApiEndpoints.getActiveChatRun,
      mockData: expect.any(Function),
      params: { sessionCode: "SESSION-1" },
      refreshKey: 7,
      suppressErrorToast: true,
    });
  });

  it("gets the active run with exactly the session query field and forwards the response", async () => {
    const response: GetActiveChatRunResult = {
      activeRun: {
        runCode: "RUN-1",
        runtimeAdapterKey: "main-agent",
        state: "RUNNING",
        supportsSteer: true,
      },
      sessionCode: "SESSION-1",
    };
    const signal = new AbortController().signal;
    vi.mocked(requestApi).mockResolvedValue(response);

    await expect(getChatActiveRun("SESSION-1", signal)).resolves.toEqual(response);
    expect(requestApi).toHaveBeenCalledWith(chatApiEndpoints.getActiveChatRun, {
      pathParams: { sessionCode: "SESSION-1" },
      signal,
    });
  });

  it("posts only sessionCode and expectedRunCode when stopping and forwards the signal", async () => {
    const input: StopChatRunInput = {
      expectedRunCode: "RUN-1",
      sessionCode: "SESSION-1",
    };
    const response: StopChatRunResult = {
      runCode: "RUN-1",
      sessionCode: "SESSION-1",
    };
    const signal = new AbortController().signal;
    vi.mocked(requestApi).mockResolvedValue(response);

    await expect(stopChatRun(input, signal)).resolves.toEqual(response);
    expect(requestApi).toHaveBeenCalledWith(chatApiEndpoints.stopChatRun, {
      body: input,
      signal,
    });
    expect(Object.keys(input).sort()).toEqual([
      "expectedRunCode",
      "sessionCode",
    ]);
  });
});

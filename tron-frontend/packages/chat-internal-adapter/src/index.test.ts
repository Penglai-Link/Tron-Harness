import { afterEach, describe, expect, it, vi } from "vitest";

import {
  FetchTransportError,
  type ChatSession,
  type SendChatMessageInput,
} from "@tron/chat-core";

import { createInternalChatAdapter, createInternalTerminalAdapter } from "./index";

const originalFetch = globalThis.fetch;

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    data,
    message: "ok",
  };
}

function sessionFixture(): ChatSession {
  return {
    createdAt: "2026-07-16T00:00:00Z",
    id: 1,
    sessionCode: "SESSION-1",
    status: "ACTIVE",
    title: "Internal session",
    updatedAt: "2026-07-16T00:00:00Z",
    workspaceCode: "TRON_CORE",
  };
}

function steerResultFixture() {
  return {
    message: {
      actor: "User",
      channel: "mainAgent",
      content: "Steer the active response",
      createdAt: "2026-07-16T00:00:01Z",
      id: 2,
      messageCode: "MSG-2",
      role: "user",
      runCode: "RUN-1",
      sessionCode: "SESSION-1",
    },
    runCode: "RUN-1",
    sessionCode: "SESSION-1",
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  globalThis.fetch = originalFetch;
});

describe("createInternalChatAdapter", () => {
  it("creates and removes terminal-scoped sessions with the chat session contract", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope(sessionFixture())), {
        headers: { "Content-Type": "application/json" },
        status: 201,
      }))
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope(sessionFixture())), {
        headers: { "Content-Type": "application/json" },
        status: 200,
      }));
    globalThis.fetch = fetchMock as typeof fetch;
    const adapter = createInternalChatAdapter({
      baseUrl: "https://internal.example.com",
    });

    await adapter.createSession({
      sourceType: "TERMINAL",
      workspaceCode: "TRON_CORE",
    });
    await adapter.deleteSession("SESSION-1");

    const [createUrl, createInit] = fetchMock.mock.calls[0] ?? [];
    const [deleteUrl, deleteInit] = fetchMock.mock.calls[1] ?? [];
    expect(createUrl).toBe("https://internal.example.com/api/v1/chats/sessions");
    expect(createInit?.method).toBe("POST");
    expect(JSON.parse(String(createInit?.body))).toMatchObject({
      sourceType: "TERMINAL",
      workspaceCode: "TRON_CORE",
    });
    expect(deleteUrl).toBe("https://internal.example.com/api/v1/chats/sessions/SESSION-1");
    expect(deleteInit?.method).toBe("DELETE");
  });

  it("preserves the active-run conflict code from a failed stream request", async () => {
    globalThis.fetch = vi.fn(async () => new Response(JSON.stringify({
      code: "CHAT_RUN_ALREADY_ACTIVE",
      data: { activeRunCode: "RUN-1" },
      message: "session already has an active agent run",
    }), {
      headers: { "Content-Type": "application/json" },
      status: 409,
    })) as typeof fetch;
    const adapter = createInternalChatAdapter({
      baseUrl: "https://internal.example.com",
    });

    const failure = await adapter.streamMessage(
      sessionFixture(),
      { content: "queue me", workspaceCode: "TRON_CORE" },
    ).catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(FetchTransportError);
    expect(failure).toMatchObject({
      code: "CHAT_RUN_ALREADY_ACTIVE",
      data: { activeRunCode: "RUN-1" },
      status: 409,
    });
  });

  it("reads and stops the authoritative active run with exact control contracts", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope({
        activeRun: {
          runCode: "RUN-1",
          runtimeAdapterKey: "react",
          state: "running",
          supportsSteer: true,
        },
        sessionCode: "SESSION-1",
      })), {
        headers: { "Content-Type": "application/json" },
        status: 200,
      }))
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope({
        runCode: "RUN-1",
        sessionCode: "SESSION-1",
      })), {
        headers: { "Content-Type": "application/json" },
        status: 200,
      }));
    globalThis.fetch = fetchMock as typeof fetch;
    const adapter = createInternalChatAdapter({
      baseUrl: "https://internal.example.com",
    });
    const abortController = new AbortController();

    await expect(adapter.getActiveRun?.(
      "SESSION-1",
      abortController.signal,
    )).resolves.toEqual({
      activeRun: {
        runCode: "RUN-1",
        runtimeAdapterKey: "react",
        state: "RUNNING",
        supportsSteer: true,
      },
      sessionCode: "SESSION-1",
    });
    await expect(adapter.stopRun?.(
      sessionFixture(),
      { expectedRunCode: "RUN-1" },
      abortController.signal,
    )).resolves.toEqual({
      runCode: "RUN-1",
      sessionCode: "SESSION-1",
    });

    const [activeUrl, activeInit] = fetchMock.mock.calls[0] ?? [];
    const [stopUrl, stopInit] = fetchMock.mock.calls[1] ?? [];
    expect(adapter.capabilities.supportsActiveRun).toBe(true);
    expect(adapter.capabilities.supportsStop).toBe(true);
    expect(activeUrl).toBe(
      "https://internal.example.com/api/v1/chats/sessions/get-active-run?sessionCode=SESSION-1",
    );
    expect(activeInit?.method).toBe("GET");
    expect(activeInit?.signal).toBe(abortController.signal);
    expect(stopUrl).toBe(
      "https://internal.example.com/api/v1/chats/sessions/stop-run",
    );
    expect(stopInit?.method).toBe("POST");
    expect(stopInit?.signal).toBe(abortController.signal);
    expect(JSON.parse(String(stopInit?.body))).toEqual({
      expectedRunCode: "RUN-1",
      sessionCode: "SESSION-1",
    });
  });

  it("posts steer input with protected turn identifiers and the abort signal", async () => {
    const fetchMock = vi.fn(async () => new Response(
      JSON.stringify(successEnvelope(steerResultFixture())),
      {
        headers: { "Content-Type": "application/json" },
        status: 200,
      },
    ));
    globalThis.fetch = fetchMock as typeof fetch;
    const adapter = createInternalChatAdapter({
      baseUrl: "https://internal.example.com",
      getAccessToken: () => "internal-token",
    });
    const abortController = new AbortController();
    const input = {
      content: "Steer the active response",
      expectedRunCode: "FORGED-RUN",
      modelCode: "MODEL-1",
      sessionCode: "FORGED-SESSION",
      workspaceCode: "TRON_CORE",
    } as SendChatMessageInput;

    const result = await adapter.steerMessage?.(
      sessionFixture(),
      input,
      { expectedRunCode: "RUN-1" },
      abortController.signal,
    );

    expect(adapter.capabilities.supportsSteer).toBe(true);
    expect(result).toEqual({
      ...steerResultFixture(),
      message: {
        ...steerResultFixture().message,
        channel: "MAIN_AGENT",
        role: "USER",
      },
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];

    expect(url).toBe("https://internal.example.com/api/v1/chats/sessions/steer-message");
    expect(init?.method).toBe("POST");
    expect(init?.signal).toBe(abortController.signal);
    expect(new Headers(init?.headers).get("Authorization")).toBe(
      "Bearer internal-token",
    );
    expect(JSON.parse(String(init?.body))).toEqual({
      attachments: [],
      content: "Steer the active response",
      expectedRunCode: "RUN-1",
      sessionCode: "SESSION-1",
    });
  });

  it("loads user questions and continues the run through the answer stream", async () => {
    const request = {
      answeredAt: null,
      answers: null,
      intro: "Choose a direction",
      questions: [],
      requestCode: "INPUT-1",
      runCode: "RUN-1",
      sessionCode: "SESSION-1",
      status: "Pending",
      requestedAt: 1_718_100_000_000,
    };
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(`data: ${JSON.stringify({
          type: "RUN_FINISHED",
          result: {
            messages: [],
            run: {
              finalOutput: "continued",
              runCode: "RUN-2",
              status: "COMPLETED",
              stepCount: 1,
              workspaceCode: "TRON_CORE",
            },
            session: sessionFixture(),
          },
        })}\n\n`));
        controller.close();
      },
    });
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope([request])), {
        headers: { "Content-Type": "application/json" },
        status: 200,
      }))
      .mockResolvedValueOnce(new Response(stream, {
        headers: { "Content-Type": "text/event-stream" },
        status: 200,
      }));
    globalThis.fetch = fetchMock as typeof fetch;
    const adapter = createInternalChatAdapter({
      baseUrl: "https://internal.example.com",
    });

    await expect(adapter.loadUserInputRequests?.("SESSION-1")).resolves.toEqual([request]);
    await adapter.answerUserInputRequestStream?.(
      sessionFixture(),
      "INPUT-1",
      { answers: [{ answerKind: "Skipped", questionCode: "QUESTION-1" }] },
    );

    const [listUrl] = fetchMock.mock.calls[0] ?? [];
    const [answerUrl, answerInit] = fetchMock.mock.calls[1] ?? [];
    expect(listUrl).toBe(
      "https://internal.example.com/api/v1/chats/sessions/SESSION-1/user-input-requests",
    );
    expect(answerUrl).toBe(
      "https://internal.example.com/api/v1/chats/sessions/SESSION-1/user-input-requests/INPUT-1/answers/stream",
    );
    expect(JSON.parse(String(answerInit?.body))).toEqual({
      answers: [{ answerKind: "Skipped", questionCode: "QUESTION-1" }],
    });
  });
});

describe("createInternalTerminalAdapter", () => {
  it("uses the internal terminal contracts with the authenticated bearer token", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope({
        expiresAt: 1_786_000_000_000,
        socketNamespace: "/terminals",
        socketPath: "/socket.io",
        ticket: "single-use-ticket",
      })), {
        headers: { "Content-Type": "application/json" },
        status: 200,
      }))
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope({
        command: "git status --short",
        modelCode: "MODEL-1",
      })), {
        headers: { "Content-Type": "application/json" },
        status: 200,
      }));
    globalThis.fetch = fetchMock as typeof fetch;
    const adapter = createInternalTerminalAdapter({
      baseUrl: "https://internal.example.com",
      getAccessToken: () => "internal-token",
    });
    const abortController = new AbortController();

    await expect(adapter.createSocketTicket({
      workspaceCode: "TRON_CORE",
    }, abortController.signal)).resolves.toEqual({
      expiresAt: 1_786_000_000_000,
      socketNamespace: "/terminals",
      socketPath: "/socket.io",
      socketUrl: "https://internal.example.com/terminals",
      ticket: "single-use-ticket",
    });
    await expect(adapter.generateCommand({
      prompt: "show the current changes",
      workspaceCode: "TRON_CORE",
    }, abortController.signal)).resolves.toEqual({
      command: "git status --short",
      modelCode: "MODEL-1",
    });

    const [ticketUrl, ticketInit] = fetchMock.mock.calls[0] ?? [];
    const [commandUrl, commandInit] = fetchMock.mock.calls[1] ?? [];
    expect(ticketUrl).toBe("https://internal.example.com/api/v1/terminals/socket-tickets");
    expect(ticketInit?.method).toBe("POST");
    expect(ticketInit?.signal).toBe(abortController.signal);
    expect(new Headers(ticketInit?.headers).get("Authorization")).toBe("Bearer internal-token");
    expect(JSON.parse(String(ticketInit?.body))).toEqual({
      appCode: null,
      workspaceCode: "TRON_CORE",
    });
    expect(commandUrl).toBe("https://internal.example.com/api/v1/terminals/commands/generate");
    expect(commandInit?.method).toBe("POST");
    expect(commandInit?.signal).toBe(abortController.signal);
    expect(new Headers(commandInit?.headers).get("Authorization")).toBe("Bearer internal-token");
    expect(JSON.parse(String(commandInit?.body))).toEqual({
      modelCode: null,
      prompt: "show the current changes",
      workspaceCode: "TRON_CORE",
    });
  });
});

import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  ChatSendResult,
  ChatSession,
  SendChatMessageInput,
} from "@tron/chat-core";

import { createOpenApiChatAdapter } from "./index";

const originalFetch = globalThis.fetch;

function successEnvelope(data: unknown) {
  return {
    code: "SUCCESS",
    data,
    message: "ok",
  };
}

function sessionFixture(overrides: Partial<ChatSession> = {}): ChatSession {
  return {
    createdAt: "2026-06-11T00:00:00Z",
    id: 1,
    sessionCode: "SESSION-1",
    status: "ACTIVE",
    title: "Open session",
    updatedAt: "2026-06-11T00:00:00Z",
    workspaceCode: "TRON_CORE",
    ...overrides,
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

describe("createOpenApiChatAdapter", () => {
  it("signs authoritative active-run lookup and stop control requests", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(successEnvelope({
        activeRun: {
          runCode: "RUN-1",
          runtimeAdapterKey: "react",
          state: "finishing",
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
    const adapter = createOpenApiChatAdapter({
      appKey: "secret-key-123",
      baseUrl: "https://openapi.example.com",
      nonceFactory: () => "controlnonce1",
      openAppCode: "OPENAPP-TEST-0001",
      timestampFactory: () => 1718100003,
      userCode: "USR-0001",
    });
    const abortController = new AbortController();

    await expect(adapter.getActiveRun?.(
      "SESSION-1",
      abortController.signal,
    )).resolves.toEqual({
      activeRun: {
        runCode: "RUN-1",
        runtimeAdapterKey: "react",
        state: "FINISHING",
        supportsSteer: true,
      },
      sessionCode: "SESSION-1",
    });
    await adapter.stopRun?.(
      sessionFixture(),
      { expectedRunCode: "RUN-1" },
      abortController.signal,
    );

    const [activeUrl, activeInit] = fetchMock.mock.calls[0] ?? [];
    const [stopUrl, stopInit] = fetchMock.mock.calls[1] ?? [];
    expect(activeUrl).toBe(
      "https://openapi.example.com/open-api/v1/chats/sessions/get-active-run?sessionCode=SESSION-1",
    );
    expect(new Headers(activeInit?.headers).get("X-Tron-Signature")).toMatch(
      /^[a-f0-9]{64}$/,
    );
    expect(stopUrl).toBe(
      "https://openapi.example.com/open-api/v1/chats/sessions/stop-run",
    );
    expect(stopInit?.method).toBe("POST");
    expect(JSON.parse(String(stopInit?.body))).toEqual({
      expectedRunCode: "RUN-1",
      sessionCode: "SESSION-1",
    });
  });

  it("signs JSON requests with HMAC headers and user-code semantics", async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(successEnvelope(sessionFixture())), {
      headers: { "Content-Type": "application/json" },
      status: 200,
    }));
    globalThis.fetch = fetchMock as typeof fetch;

    const adapter = createOpenApiChatAdapter({
      openAppCode: "OPENAPP-TEST-0001",
      appKey: "secret-key-123",
      baseUrl: "https://openapi.example.com",
      nonceFactory: () => "abc123nonce789",
      timestampFactory: () => 1718100000,
      userCode: "USR-0001",
    });

    await adapter.createSession({
      title: "Open session",
      workspaceCode: "TRON_CORE",
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    const headers = new Headers(init?.headers);

    expect(url).toBe("https://openapi.example.com/open-api/v1/chats/sessions");
    expect(headers.get("Authorization")).toBeNull();
    expect(headers.get("X-Tron-User-Id")).toBeNull();
    expect(headers.get("X-Tron-App-Code")).toBe("OPENAPP-TEST-0001");
    expect(headers.get("X-Tron-User-Code")).toBe("USR-0001");
    expect(headers.get("X-Tron-Signature-Version")).toBe("TRON-HMAC-SHA256-V1");
    expect(headers.get("X-Tron-Timestamp")).toBe("1718100000");
    expect(headers.get("X-Tron-Nonce")).toBe("abc123nonce789");
    expect(headers.get("X-Tron-Signature")).toMatch(/^[a-f0-9]{64}$/);
  });

  it("signs stream requests before opening the SSE channel", async () => {
    const streamPayload = {
      type: "RUN_FINISHED",
      result: {
        messages: [],
        run: {
          finalOutput: "done",
          runCode: "RUN-1",
          status: "COMPLETED",
          stepCount: 1,
          workspaceCode: "TRON_CORE",
        },
        session: sessionFixture(),
      } satisfies ChatSendResult,
    };
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(`data: ${JSON.stringify(streamPayload)}\n\n`));
        controller.close();
      },
    });
    const fetchMock = vi.fn(async () => new Response(stream, {
      headers: { "Content-Type": "text/event-stream" },
      status: 200,
    }));
    globalThis.fetch = fetchMock as typeof fetch;

    const adapter = createOpenApiChatAdapter({
      openAppCode: "OPENAPP-TEST-0001",
      appKey: "secret-key-123",
      baseUrl: "https://openapi.example.com",
      nonceFactory: () => "streamnonce12",
      timestampFactory: () => 1718100001,
      userCode: "USR-0001",
    });

    await adapter.streamMessage(
      sessionFixture(),
      { content: "hello", workspaceCode: "TRON_CORE" },
    );

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    const headers = new Headers(init?.headers);

    expect(url).toBe("https://openapi.example.com/open-api/v1/chats/sessions/SESSION-1/messages/stream");
    expect(headers.get("Authorization")).toBeNull();
    expect(headers.get("X-Tron-User-Id")).toBeNull();
    expect(headers.get("X-Tron-User-Code")).toBe("USR-0001");
    expect(headers.get("X-Tron-Signature")).toMatch(/^[a-f0-9]{64}$/);
  });

  it("signs steer requests with protected turn identifiers and the abort signal", async () => {
    const fetchMock = vi.fn(async () => new Response(
      JSON.stringify(successEnvelope(steerResultFixture())),
      {
        headers: { "Content-Type": "application/json" },
        status: 200,
      },
    ));
    globalThis.fetch = fetchMock as typeof fetch;
    const adapter = createOpenApiChatAdapter({
      openAppCode: "OPENAPP-TEST-0001",
      appKey: "secret-key-123",
      baseUrl: "https://openapi.example.com",
      nonceFactory: () => "steernonce123",
      timestampFactory: () => 1718100002,
      userCode: "USR-0001",
    });
    const abortController = new AbortController();
    const input = {
      content: "Steer the active response",
      expectedRunCode: "FORGED-RUN",
      sessionCode: "FORGED-SESSION",
      systemModelId: "SYSTEM-MODEL-1",
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
    const headers = new Headers(init?.headers);

    expect(url).toBe("https://openapi.example.com/open-api/v1/chats/sessions/steer-message");
    expect(init?.method).toBe("POST");
    expect(init?.signal).toBe(abortController.signal);
    expect(headers.get("X-Tron-User-Code")).toBe("USR-0001");
    expect(headers.get("X-Tron-Signature")).toMatch(/^[a-f0-9]{64}$/);
    expect(JSON.parse(String(init?.body))).toEqual({
      attachments: [],
      content: "Steer the active response",
      expectedRunCode: "RUN-1",
      sessionCode: "SESSION-1",
    });
  });
});

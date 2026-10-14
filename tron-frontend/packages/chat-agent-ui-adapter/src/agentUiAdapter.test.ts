import { afterEach, describe, expect, it, vi } from "vitest";
import { ChatAdapterError } from "@eliteforge/agent-ui-chat-core";
import { createMockChatAdapter, FetchTransportError, type ChatSession, type SendChatMessageInput } from "@tron/chat-core";
import { createAgentUiAdapter } from "./agentUiAdapter";
import { createInternalChatAdapter } from "../../chat-internal-adapter/src/index";
import type { ChatAguiStreamEvent as UiProtocolEvent } from "@eliteforge/agent-ui-chat-core";
import { fromUiInput, fromUiResume, fromUiSession, toUiAttachment, fromUiAttachment, toUiEvent, toUiInput, toUiSession } from "./agentUiMapping";

const session = {
  id: 12, sessionCode: "SESSION", workspaceCode: "WORKSPACE", appCode: "APP",
  projectCode: "PROJECT", sourceCode: "AUTOMATION", taskCode: "TASK", userCode: "USER",
  agentCode: "codex", systemAgentSessionId: "external-session", title: "Existing history",
  status: "ACTIVE", createdAt: "2026-09-27", updatedAt: "2026-09-27",
} satisfies ChatSession & { projectCode: string; sourceCode: string };

describe("Tron / Agent UI boundary", () => {
  it("retains host scope and external session identity after public UI normalization", () => {
    const ui = toUiSession(session);
    expect(ui.sessionId).toBe("SESSION");
    expect(fromUiSession({ ...ui, title: "Renamed" })).toEqual({ ...session, title: "Renamed" });
    expect(() => fromUiSession({ ...ui, metadata: {} })).toThrow("host context");
  });

  it("keeps routing, source page, attachments and external model IDs when a send is queued", () => {
    const input: SendChatMessageInput = {
      content: "Continue", annotations: [{id:"a-1",messageId:"source",selectedText:"original",comment:"Explain"}], requestedSkills: ["review"], workspaceCode: "WORKSPACE", appCode: "APP", taskCode: "TASK", agentCode: "codex",
      modelSelection: { kind: "ProviderRoute", providerCode: "PersonAi", qualityType: "High", routeCode: "ChatAuto" },
      modelCode: "kuaishou/deepseek", systemModelId: "codex/model", reasoningEffort: "XHigh",
      pageContext: { title: "Application", path: "/apps/APP" },
      attachments: [{ id: "FILE", resourceCode: "RESOURCE", kind: "image", metadata: { fileRef: "attachment" }, summary: "Image" }],
    };
    expect(fromUiInput({ ...toUiInput(input) })).toEqual(input);
    expect(fromUiInput({ ...toUiInput(input), requestedSkills: ["different-skill"] }).requestedSkills).toEqual(["different-skill"]);
    expect(fromUiAttachment(toUiAttachment(input.attachments![0]))).toEqual(input.attachments![0]);
  });

  it("maps terminal interruption questions and continuation payloads in both directions", () => {
    const ui = toUiEvent({ type: "RUN_FINISHED", outcome: { type: "interrupt", interrupts: [{
      id: "REQUEST", reason: "input_required", metadata: { requestCode: "REQUEST", questions: [{
        questionCode: "Q1", question: "Choose", options: [{ optionCode: "A1", label: "First" }],
      }] },
    }] } });
    expect(ui.outcome?.type).toBe("interrupt");
    if (ui.outcome?.type !== "interrupt") throw new Error("Expected interrupt");
    expect(ui.outcome.interrupts[0].metadata?.questions).toMatchObject([{
      questionId: "Q1", options: [{ optionId: "A1" }],
    }]);
    expect(fromUiResume([{ interruptId: "REQUEST", status: "resolved", payload: {
      extra: "preserved", answers: [{ questionId: "Q1", optionId: "A1", answerKind: "Option" }],
    } }])).toEqual([{ interruptId: "REQUEST", status: "resolved", payload: {
      extra: "preserved", answers: [{ questionCode: "Q1", optionCode: "A1", answerKind: "Option" }],
    } }]);
  });

  it("maps live and replayed compaction without rewriting user tool payloads", () => {
    const event = { type: "CUSTOM", name: "tron.short_term_memory.compaction.completed",
      value: { resourceCode: "unrelated-product-payload" } };
    expect(toUiEvent(event)).toMatchObject({
      name: "agent.memory.compaction.completed", value: event.value,
    });
  });

  it("forwards real stop and steer contracts, signals and safe queue errors", async () => {
    const host = createMockChatAdapter();
    host.capabilities.supportsStop = true;
    host.capabilities.supportsSteer = true;
    host.stopRun = vi.fn(async () => ({ sessionCode: "SESSION", runCode: "RUN" }));
    host.steerMessage = vi.fn(async () => { throw new FetchTransportError("Finishing", {
      code: "CHAT_STEER_RUN_FINISHING", status: 409,
    }); });
    const adapter = createAgentUiAdapter(host, { workspaceCode: "WORKSPACE" });
    const signal = new AbortController().signal;
    await expect(adapter.stopRun!(toUiSession(session), { expectedRunId: "RUN" }, signal))
      .resolves.toEqual({ sessionId: "SESSION", runId: "RUN" });
    expect(host.stopRun).toHaveBeenCalledWith(session, { expectedRunCode: "RUN" }, signal);
    await expect(adapter.steerMessage!(toUiSession(session), toUiInput({
      workspaceCode: "WORKSPACE", content: "Continue",
    }), { expectedRunId: "RUN" }, signal)).rejects.toMatchObject({
      name: "ChatAdapterError", code: "STEER_RUN_FINISHING", status: 409,
    });
  });

  it("never creates a session without its workspace and preserves transport failures", async () => {
    const host = createMockChatAdapter();
    host.createSession = vi.fn(async () => session);
    await expect(createAgentUiAdapter(host, {}).createSession({ title: "Test" })).rejects.toThrow("Workspace code");
    expect(host.createSession).not.toHaveBeenCalled();
    host.loadMessages = vi.fn(async () => { throw new FetchTransportError("Denied", {
      code: "FORBIDDEN", status: 403, data: { reason: "scope" },
    }); });
    const error = await createAgentUiAdapter(host, {}).loadMessages("SESSION").catch(value => value);
    expect(error).toBeInstanceOf(ChatAdapterError);
    expect(error).toMatchObject({ code: "FORBIDDEN", status: 403, details: { reason: "scope" } });
  });
});

it.each([true,false,{answers:[true],count:2}])("keeps native resume payload %j through the real product adapter", async (payload) => {
  const host=createMockChatAdapter();
  host.answerUserInputRequestStream=vi.fn(async()=>({session,run:{runCode:"NEXT",workspaceCode:"WORKSPACE",status:"COMPLETED" as const,finalOutput:"",stepCount:1},messages:[],userInputRequest:null}));
  const adapter=createAgentUiAdapter(host,{workspaceCode:"WORKSPACE"});
  const resume=[{interruptId:"SDK-NATIVE",status:"resolved" as const,payload}];
  await adapter.answerUserInputRequestStream!(toUiSession(session),"SDK-NATIVE",{answers:[],resume});
  expect(host.answerUserInputRequestStream).toHaveBeenCalledWith(session,"SDK-NATIVE",{answers:[],resume},expect.anything(),undefined);
});


describe("native protocol result boundary", () => {
  afterEach(() => vi.unstubAllGlobals());
  const nativeResult = {
    output: "Input required", content: [{ type: "text", text: "Input required" }],
    structured: { nested: { runCode: "opaque-user-data" } },
    interactions: [{ id: "SDK-INPUT", kind: "Input", prompt: [{ type: "text", text: "Continue?" }], response_schema: { type: "boolean" } }],
    status: "AwaitingInput", usage: { input_tokens: 10, output_tokens: 3, total_tokens: 13 }, steps: 1, tool_calls: 1,
  };
  const nativeTerminal = {
    type: "RUN_FINISHED", runId: "ROOT", threadId: "SESSION", timestamp: 20,
    outcome: { type: "interrupt" as const, interrupts: [{ id: "SDK-INPUT", reason: "input_required", responseSchema: { type: "boolean" } }] },
    result: nativeResult,
    metadata: { framework: { executionId: "EXECUTION", sourceSequence: 16 }, tron: { eventId: "native:EXECUTION:16:0" } },
  };
  const productResult = {
    session, run: { runCode: "ROOT", workspaceCode: "WORKSPACE", status: "AWAITING_USER_INPUT" as const, finalOutput: "Input required", stepCount: 1 },
    messages: [], userInputRequest: null,
  };

  const message = { messageCode: "MESSAGE", sessionCode: "SESSION", role: "ASSISTANT", channel: "FINAL" };
  const invalidConsumedResults = [
    { name: "session status", result: { ...productResult, session: { ...session, status: 1 } } },
    { name: "last run status", result: { ...productResult, session: { ...session, lastRunStatus: {} } } },
    { name: "source type", result: { ...productResult, session: { ...session, sourceType: false } } },
    { name: "agent code", result: { ...productResult, session: { ...session, agentCode: [] } } },
    { name: "run status", result: { ...productResult, run: { ...productResult.run, status: 1 } } },
    { name: "message role", result: { ...productResult, messages: [{ ...message, role: 1 }] } },
    { name: "message channel", result: { ...productResult, messages: [{ ...message, channel: {} }] } },
    { name: "attachments array", result: { ...productResult, messages: [{ ...message, attachments: 1 }] } },
    { name: "attachment entry", result: { ...productResult, messages: [{ ...message, attachments: [null] }] } },
    { name: "request object", result: { ...productResult, userInputRequest: 1 } },
    { name: "questions array", result: { ...productResult, userInputRequest: { questions: null } } },
    { name: "question entry", result: { ...productResult, userInputRequest: { questions: [null] } } },
    { name: "options array", result: { ...productResult, userInputRequest: { questions: [{ options: 1 }] } } },
    { name: "option entry", result: { ...productResult, userInputRequest: { questions: [{ options: [null] }] } } },
    { name: "answers array", result: { ...productResult, userInputRequest: { questions: [], answers: {} } } },
    { name: "answer entry", result: { ...productResult, userInputRequest: { questions: [], answers: [null] } } },
  ];

  it.each(invalidConsumedResults)("preserves opaque $name and rejects the same marked product shape", ({ result }) => {
    expect(toUiEvent({ ...nativeTerminal, result }).result).toBe(result);
    expect(() => toUiEvent({ ...nativeTerminal, productCompletion: true, result })).toThrow("Invalid product completion result");
  });

  it("loads opaque colliding consumer shapes through the real replay adapter", async () => {
    const frames = invalidConsumedResults.flatMap(({ result }) => [
      { ...nativeTerminal, result },
      { ...nativeTerminal, type: "RUN_ERROR", code: "MODEL_PROVIDER_FAILED", result },
    ]);
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ code: "SUCCESS", data: frames.map((data) => ({ data, sseEvent: data.type })), message: "" }), { headers: { "Content-Type": "application/json" } })));
    const adapter = createAgentUiAdapter(createInternalChatAdapter({ workspaceCode: "WORKSPACE" }), { workspaceCode: "WORKSPACE" });
    const replay = await adapter.loadProtocolEvents!("SESSION");
    expect(replay).toHaveLength(frames.length);
    replay.forEach((event, index) => {
      expect(event.result).toEqual(frames[index].result);
      expect(event.type).toBe(frames[index].type);
    });
  });

  it("maps valid product collections and nullable optional strings", () => {
    const result = {
      ...productResult,
      session: { ...session, sourceType: null, lastRunStatus: null, agentCode: null },
      messages: [{ ...message, attachments: [{ resourceCode: "RESOURCE" }] }],
      userInputRequest: { requestCode: "REQUEST", sessionCode: "SESSION", runCode: "ROOT", questions: [{ questionCode: "QUESTION", options: [{ optionCode: "OPTION" }] }], answers: [{ questionCode: "QUESTION", optionCode: "OPTION" }] },
    };
    expect(toUiEvent({ ...nativeTerminal, productCompletion: true, result }).result).toMatchObject({
      session: { sessionId: "SESSION" },
      messages: [{ messageId: "MESSAGE", attachments: [{ resourceId: "RESOURCE" }] }],
      userInputRequest: { requestId: "REQUEST", questions: [{ questionId: "QUESTION", options: [{ optionId: "OPTION" }] }], answers: [{ questionId: "QUESTION", optionId: "OPTION" }] },
    });
  });

  it("loads real native root and child terminal replay without treating opaque results as product DTOs", async () => {
    const child = { ...nativeTerminal, runId: "CHILD", result: { ...nativeResult, status: "Completed", interactions: [] }, outcome: { type: "success" } };
    const warning = { type: "RUN_ERROR", runId: "ROOT", threadId: "SESSION", code: "AGUI_STREAM_OVERFLOW", presentationIncomplete: true };
    const incomplete = { type: "CUSTOM", runId: "ROOT", threadId: "SESSION", name: "tron.presentation.incomplete", value: { code: "AGENT_AGUI_REPLAY_INCOMPLETE" } };
    const frames = [nativeTerminal, child, warning, incomplete];
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ code: "SUCCESS", data: frames.map((data) => ({ data, sseEvent: data.type })), message: "" }), { headers: { "Content-Type": "application/json" } })));
    const adapter = createAgentUiAdapter(createInternalChatAdapter({ workspaceCode: "WORKSPACE" }), { workspaceCode: "WORKSPACE" });
    const replay = await adapter.loadProtocolEvents!("SESSION");
    expect(replay[0].result).toEqual(nativeResult);
    expect(replay[0].outcome).toEqual(nativeTerminal.outcome);
    expect(replay[0].metadata).toEqual(nativeTerminal.metadata);
    expect(replay[1].result).toEqual(child.result);
    expect(replay[1].runId).toBe("CHILD");
    expect(replay[2]).toMatchObject(warning);
    expect(replay[3]).toMatchObject(incomplete);
  });

  it("still maps actual product completion through the real SSE and UI adapters", async () => {
    const frames = [{ type: "RUN_STARTED", runId: "ROOT", threadId: "SESSION" }, { ...nativeTerminal, productCompletion: true, result: productResult }];
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(frames.map((frame) => `event: ${frame.type}\ndata: ${JSON.stringify(frame)}\n\n`).join(""), { headers: { "Content-Type": "text/event-stream" } })));
    const events: UiProtocolEvent[] = [];
    const adapter = createAgentUiAdapter(createInternalChatAdapter({ workspaceCode: "WORKSPACE" }), { workspaceCode: "WORKSPACE" });
    const result = await adapter.streamMessage(toUiSession(session), toUiInput({ workspaceCode: "WORKSPACE", content: "Continue" }), { onAguiEvent: (event) => events.push(event) });
    expect(result.run.runId).toBe("ROOT");
    expect(result.session.sessionId).toBe("SESSION");
    expect(events[1].result).toMatchObject({ run: { runId: "ROOT" }, session: { sessionId: "SESSION" } });
    expect(events[1].productCompletion).toBe(true);
  });

  it.each([null, false, 0, "opaque", ["native"], { run: "arbitrary-native-json" }, { session: "native", run: "opaque", messages: [] }, { session: { sessionCode: "native" }, run: { runCode: "opaque" }, messages: [null] }].map((result) => ({ result })))("retains arbitrary AG-UI result JSON: $result", ({ result }) => {
    expect(toUiEvent({ ...nativeTerminal, result }).result).toBe(result);
  });

  it.each([undefined, nativeResult, { session: "native", run: "opaque", messages: [] }, { ...productResult, messages: undefined }, { ...productResult, run: null }, { ...productResult, session: {} }, { ...productResult, messages: [null] }])("rejects malformed explicit product completion instead of treating it as native: %j", (result) => {
    expect(() => toUiEvent({ ...nativeTerminal, productCompletion: true, result })).toThrow("Invalid product completion result");
  });

  it("reports lost product acknowledgment through the real SSE adapters without deleting the native result", async () => {
    const failed = { ...nativeTerminal, productCompletion: true, presentationIncomplete: true, productCompletionError: { code: "CHAT_STREAM_COMPLETION_UNAVAILABLE", message: "Product acknowledgment unavailable" } };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(`event: RUN_FINISHED\ndata: ${JSON.stringify(failed)}\n\n`, { headers: { "Content-Type": "text/event-stream" } })));
    const events: UiProtocolEvent[] = [];
    const adapter = createAgentUiAdapter(createInternalChatAdapter({ workspaceCode: "WORKSPACE" }), { workspaceCode: "WORKSPACE" });
    await expect(adapter.streamMessage(toUiSession(session), toUiInput({ workspaceCode: "WORKSPACE", content: "Continue" }), { onAguiEvent: (event) => events.push(event) })).rejects.toMatchObject({ code: "CHAT_STREAM_COMPLETION_UNAVAILABLE" });
    expect(events[0].result).toEqual(nativeResult);
    expect(events[0].presentationIncomplete).toBe(true);
    expect(events[0].productCompletionError).toEqual(failed.productCompletionError);
  });

  it("maps a legacy product envelope and preserves native acknowledgment failure details", () => {
    expect(toUiEvent({ ...nativeTerminal, result: productResult }).result).toMatchObject({ run: { runId: "ROOT" } });
    const failed = { ...nativeTerminal, productCompletion: true, presentationIncomplete: true, productCompletionError: { code: "CHAT_STREAM_COMPLETION_UNAVAILABLE" } };
    const mapped = toUiEvent(failed);
    expect(mapped.result).toBe(nativeResult);
    expect(mapped.presentationIncomplete).toBe(true);
    expect(mapped.productCompletionError).toEqual(failed.productCompletionError);
  });
});

it("marks invalid native annotations as rejected before admission", async () => {
  const { uiRequest } = await import('./agentUiMapping');
  const { FetchTransportError } = await import('@tron/chat-core');
  await expect(uiRequest(Promise.reject(new FetchTransportError('Invalid annotations', {code: '101032006', status: 400})))).rejects.toMatchObject({rejectedBeforeAdmission: true, code: '101032006'});
});

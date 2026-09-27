import { describe, expect, it, vi } from "vitest";
import { ChatAdapterError } from "@eliteforge/agent-ui-chat-core";
import { createMockChatAdapter, FetchTransportError, type ChatSession, type SendChatMessageInput } from "@tron/chat-core";
import { createAgentUiAdapter } from "./agentUiAdapter";
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
      content: "Continue", workspaceCode: "WORKSPACE", appCode: "APP", taskCode: "TASK", agentCode: "codex",
      modelSelection: { kind: "ProviderRoute", providerCode: "PersonAi", qualityType: "High", routeCode: "ChatAuto" },
      modelCode: "kuaishou/deepseek", systemModelId: "codex/model", reasoningEffort: "XHigh",
      pageContext: { title: "Application", path: "/apps/APP" },
      attachments: [{ id: "FILE", resourceCode: "RESOURCE", kind: "image", metadata: { fileRef: "attachment" }, summary: "Image" }],
    };
    expect(fromUiInput({ ...toUiInput(input) })).toEqual(input);
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

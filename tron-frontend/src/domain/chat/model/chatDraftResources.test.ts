// @vitest-environment jsdom
import { describe, expect, it, vi } from "vitest";
import type { ChatAdapter, ChatSession } from "@tron/chat-core";
import { ChatDraftResources } from "./chatDraftResources";

const session: ChatSession = { id: 1, sessionCode: "SESSION", workspaceCode: "workspace", agentCode: "reviewer", title: "New chat", status: "ACTIVE", createdAt: "", updatedAt: "" };

function host() {
  return {
    capabilities: { supportsAbort: true, supportsAgui: false, supportsStreaming: true, supportsProtocolReplay: false },
    createSession: vi.fn(async () => session), deleteSession: vi.fn(), loadMessages: vi.fn(), loadAguiEvents: vi.fn(),
    streamMessage: vi.fn().mockResolvedValue({ session, messages: [] }),
    uploadAttachment: vi.fn(async (_session, input) => ({ id: "RESOURCE", resourceCode: "RESOURCE", kind: input.kind, metadata: input.metadata, summary: input.summary })),
  } satisfies ChatAdapter;
}

describe("draft chat resources", () => {
  it("coalesces creation and permits retry after creation failure", async () => {
    const resources = new ChatDraftResources();
    const create = vi.fn().mockRejectedValueOnce(new Error("offline")).mockResolvedValue(session);
    await expect(resources.createSession(create)).rejects.toThrow("offline");
    const first = resources.createSession(create);
    const second = resources.createSession(create);
    expect(first).toBe(second);
    await expect(first).resolves.toEqual(session);
    expect(create).toHaveBeenCalledTimes(2);
  });

  it("stages and previews files locally, uploading only the retained files on first send", async () => {
    const resources = new ChatDraftResources();
    const adapter = host();
    const runtime = resources.adapt(adapter);
    const file = new File(["document"], "requirements.txt", { type: "text/plain" });
    const attachment = resources.stage({ file, kind: "file", metadata: { annotation: true }, summary: "requirements" });
    resources.stage({ file: new File(["removed"], "removed.txt") });
    expect(adapter.createSession).not.toHaveBeenCalled();
    expect(adapter.uploadAttachment).not.toHaveBeenCalled();
    await expect(runtime.downloadAttachment!(session, attachment)).resolves.toBe(file);
    await runtime.streamMessage(session, { content: "review", attachments: [attachment], agentCode: "reviewer", workspaceCode: "workspace" });
    expect(adapter.uploadAttachment).toHaveBeenCalledExactlyOnceWith(session, { file, kind: "file", metadata: { annotation: true }, summary: "requirements" }, undefined);
    expect(adapter.streamMessage.mock.calls[0][1]).toMatchObject({ content: "review", agentCode: "reviewer", attachments: [{ id: "RESOURCE", resourceCode: "RESOURCE" }] });
  });

  it("reuses successful uploads on send retry and retries failed uploads", async () => {
    const resources = new ChatDraftResources();
    const adapter = host();
    const runtime = resources.adapt(adapter);
    const attachment = resources.stage({ file: new File(["hello"], "hello.txt") });
    const input = { content: "hello", workspaceCode: "workspace", attachments: [attachment] };
    adapter.uploadAttachment.mockRejectedValueOnce(new Error("upload failed"));
    await expect(runtime.streamMessage(session, input)).rejects.toThrow("upload failed");
    expect(adapter.streamMessage).not.toHaveBeenCalled();
    adapter.streamMessage.mockRejectedValueOnce(new Error("send failed"));
    await expect(runtime.streamMessage(session, input)).rejects.toThrow("send failed");
    await runtime.streamMessage(session, input);
    expect(adapter.uploadAttachment).toHaveBeenCalledTimes(2);
    expect(adapter.streamMessage).toHaveBeenCalledTimes(2);
  });

  it("forwards independent submissions without merging their content", async () => {
    const resources = new ChatDraftResources();
    const adapter = host();
    const runtime = resources.adapt(adapter);
    await Promise.all([
      runtime.streamMessage(session, { content: "first", workspaceCode: "workspace" }),
      runtime.streamMessage(session, { content: "second", workspaceCode: "workspace" }),
    ]);
    expect(adapter.streamMessage).toHaveBeenCalledTimes(2);
    expect(adapter.streamMessage.mock.calls.map((call) => call[1].content)).toEqual(["first", "second"]);
  });
});

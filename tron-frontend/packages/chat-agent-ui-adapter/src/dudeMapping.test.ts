import { ChatAdapterError } from "@eliteforge/agent-ui-chat-core";
import { FetchTransportError } from "@tron/chat-core";
import { expect, it } from "vitest";
import { fromUiInput, fromUiSession, toUiEvent, toUiInput, toUiMessage, toUiSession, uiRequest } from "./agentUiMapping";
import type { ChatSession, ChatMessage } from "@tron/chat-core";
const session: ChatSession = { id: "9007199254740993", sessionCode: "platform", workspaceCode: null, agentCode: "main-dude", ownerDudeCode: "main-dude", title: "Dude", status: "ACTIVE", createdAt: "2026-01-01", updatedAt: "2026-01-01" };
it("round-trips platform scope and ownership without substituting a workspace", () => { expect(fromUiSession(toUiSession(session))).toEqual(session); expect(fromUiInput(toUiInput({ content: "hello", workspaceCode: null }))).toMatchObject({ workspaceCode: null }); });
it("preserves real sender identity independently of role in history and stream mapping", () => {
 const provenance = { sender: { kind: "Dude", id: "main-dude", displayName: "Main Dude", avatarUrl: "/avatar.png" }, source: "ThreadDelegation" };
 const message: ChatMessage = { id: 1, messageCode: "m", sessionCode: "thread", role: "USER", channel: "MAIN_AGENT", actor: "Main Dude", provenance, content: "Work", createdAt: "2026-01-01" };
 expect(toUiMessage(message)).toMatchObject({ role: "USER", provenance });
 expect(toUiEvent({ type: "CUSTOM", name: "framework.message.provenance", threadId: "thread", value: { messageId: "m", provenance } })).toMatchObject({ name: "framework.message.provenance", threadId: "thread", value: { provenance } });
});

it("only marks guaranteed composer admission rejections as safe to restore", async () => {
 for (const code of ["101032001", "101032002", "101032003", "101032004"]) {
  await expect(uiRequest(Promise.reject(new FetchTransportError("Rejected", { code, status: 403 })))).rejects.toMatchObject({ rejectedBeforeAdmission: true });
 }
 for (const code of ["101032005", "OTHER_FAILURE"]) {
  await expect(uiRequest(Promise.reject(new FetchTransportError("Unknown admission", { code, status: 403 })))).rejects.toMatchObject({ rejectedBeforeAdmission: false });
 }
 await expect(uiRequest(Promise.reject(new Error("Network lost")))).rejects.not.toBeInstanceOf(ChatAdapterError);
});

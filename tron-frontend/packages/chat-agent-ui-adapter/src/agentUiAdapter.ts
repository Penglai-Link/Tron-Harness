import type * as Ui from "@eliteforge/agent-ui-chat-core";
import type { ChatAdapter, ChatClientConfig, ChatStreamHandlers } from "@tron/chat-core";
import { fromUiAnswer, fromUiAttachment, fromUiInput, fromUiResume, fromUiSession, toUiAttachment, toUiEvent, toUiMessage, toUiQuestion, toUiResult, toUiSession, uiRequest } from "./agentUiMapping";

/** Transport/auth/locale stay in the existing product adapter. No API is called by UI components. */
export function createAgentUiAdapter(host: ChatAdapter, config: ChatClientConfig): Ui.ChatAdapter {
  const handlers = (target?: Ui.ChatStreamHandlers): ChatStreamHandlers => ({
    onDelta: target?.onDelta,
    onAguiEvent: (event) => target?.onAguiEvent?.(toUiEvent(event)),
  });
  return {
    capabilities: host.capabilities,
    createSession: async (input) => {
      if (!config.workspaceCode?.trim()) throw new Error("Workspace code is required before starting a session");
      return toUiSession(await uiRequest(host.createSession({
        workspaceCode: config.workspaceCode, appCode: config.appCode, taskCode: config.taskCode,
        agentCode: config.agentCode, title: input.title,
      })));
    },
    deleteSession: async (...args) => toUiSession(await uiRequest(host.deleteSession(...args))),
    loadA2uiState: host.loadA2uiState ? (...args) => uiRequest(host.loadA2uiState!(...args)) : undefined,
    sendA2uiActionStream: host.sendA2uiActionStream ? async (session,input,events,signal) => toUiResult(await uiRequest(host.sendA2uiActionStream!(fromUiSession(session),input,handlers(events),signal))) : undefined,
    loadMessages: async (id) => (await uiRequest(host.loadMessages(id))).map(toUiMessage),
    loadHistory: host.loadHistory ? (...args) => uiRequest(host.loadHistory!(...args)) : undefined,
    loadAguiEvents: async (id) => (await uiRequest(host.loadAguiEvents(id))).map(({ runCode, ...event }) => ({ ...event, runId: runCode })),
    loadProtocolEvents: host.loadProtocolEvents ? async (id) => (await uiRequest(host.loadProtocolEvents!(id))).map(toUiEvent) : undefined,
    loadUserInputRequests: host.loadUserInputRequests ? async (...args) => (await uiRequest(host.loadUserInputRequests!(...args))).map(toUiQuestion) : undefined,
    getActiveRun: host.getActiveRun ? async (...args) => {
      const value = await uiRequest(host.getActiveRun!(...args));
      return { sessionId: value.sessionCode, activeRun: value.activeRun ? {
        runId: value.activeRun.runCode, runtimeAdapterKey: value.activeRun.runtimeAdapterKey,
        state: value.activeRun.state, supportsSteer: value.activeRun.supportsSteer,
      } : null };
    } : undefined,
    streamMessage: async (session, input, events, signal) => toUiResult(await uiRequest(
      host.streamMessage(fromUiSession(session), fromUiInput(input), handlers(events), signal))),
    steerMessage: host.steerMessage ? async (session, input, context, signal) => {
      const value = await uiRequest(host.steerMessage!(fromUiSession(session), fromUiInput(input),
        { expectedRunCode: context.expectedRunId }, signal));
      return { sessionId: value.sessionCode, runId: value.runCode, message: toUiMessage(value.message) };
    } : undefined,
    stopRun: host.stopRun ? async (session, context, signal) => {
      const value = await uiRequest(host.stopRun!(fromUiSession(session), { expectedRunCode: context.expectedRunId }, signal));
      return { sessionId: value.sessionCode, runId: value.runCode };
    } : undefined,
    uploadAttachment: host.uploadAttachment ? async (session, input, signal) =>
      toUiAttachment(await uiRequest(host.uploadAttachment!(fromUiSession(session), input, signal))) : undefined,
    downloadAttachment: host.downloadAttachment ? (session, attachment, signal) =>
      uiRequest(host.downloadAttachment!(fromUiSession(session), fromUiAttachment(attachment), signal)) : undefined,
    answerUserInputRequestStream: host.answerUserInputRequestStream ? async (session, id, input, events, signal) => {
      const answers = input.answers.map(fromUiAnswer);
      // The payload is a product API contract too; mapping only the outer answers loses resume data.
      const resume = input.resume ? (input.answers.length ? fromUiResume(input.resume) : input.resume) : undefined;
      return toUiResult(await uiRequest(host.answerUserInputRequestStream!(fromUiSession(session), id,
        { answers, resume, clientCapabilities:input.clientCapabilities, a2uiMetadata:input.a2uiMetadata }, handlers(events), signal)));
    } : undefined,
  };
}

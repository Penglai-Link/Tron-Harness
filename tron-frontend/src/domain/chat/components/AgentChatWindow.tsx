import { useCallback, useMemo } from "react";
import { ChatRuntimeWindow, type ChatRuntimeWindowProps } from "@eliteforge/agent-ui-chat-react";
import type * as Ui from "@eliteforge/agent-ui-chat-core";
import type { ChatAdapter, ChatClientConfig, ChatMessageAttachment, ChatRuntimeRunState, ChatSession, SendChatMessageInput } from "@tron/chat-core";
import { createAgentUiAdapter } from "@tron/chat-agent-ui-adapter";
import { fromUiAttachment, fromUiSession, toUiAttachment, toUiInput, toUiSession } from "@tron/chat-agent-ui-adapter";
import { useChatFeedback } from "@/domain/chat/hooks/useChatFeedback";

type MappedProps = "adapter" | "config" | "initialSession" | "pendingAttachments" | "onPendingAttachmentsChange" | "onSessionChange" | "onRunStateChange" | "createSendInput";
export interface AgentChatWindowProps extends Omit<ChatRuntimeWindowProps, MappedProps> {
  adapter: ChatAdapter;
  config: ChatClientConfig;
  initialSession?: ChatSession | null;
  pendingAttachments?: ChatMessageAttachment[];
  onPendingAttachmentsChange?: (value: ChatMessageAttachment[]) => void;
  onSessionChange?: (value: ChatSession | null) => void;
  onRunStateChange?: (value: ChatRuntimeRunState) => void;
  createSendInput?: (session: ChatSession, content: string, attachments: ChatMessageAttachment[]) => SendChatMessageInput | null;
}

/** Product context adapter only. Rendering, streaming, queues, stop and replay belong to Agent UI. */
export function AgentChatWindow({ adapter, config, initialSession, pendingAttachments,
  onPendingAttachmentsChange, onSessionChange, onRunStateChange, createSendInput, onError, onInfo, ...props }: AgentChatWindowProps) {
  const feedback = useChatFeedback();
  const uiAdapter = useMemo(() => createAgentUiAdapter(adapter, config),
    [adapter, config.workspaceCode, config.appCode, config.taskCode, config.agentCode]);
  const session = useMemo(() => initialSession ? toUiSession(initialSession) : initialSession, [initialSession]);
  const attachments = useMemo(() => pendingAttachments?.map(toUiAttachment), [pendingAttachments]);
  const handleSession = useCallback((value: Ui.ChatSession | null) =>
    onSessionChange?.(value ? fromUiSession(value) : null), [onSessionChange]);
  const handleAttachments = useCallback((value: Ui.ChatMessageAttachment[]) =>
    onPendingAttachmentsChange?.(value.map(fromUiAttachment)), [onPendingAttachmentsChange]);
  const handleRun = useCallback((value: Ui.ChatRuntimeRunState) => onRunStateChange?.({
    active: value.active, runCode: value.runId, sessionCode: value.sessionId, supportsSteer: value.supportsSteer,
  }), [onRunStateChange]);
  const createInput = useCallback((value: Ui.ChatSession, content: string, files: Ui.ChatMessageAttachment[]) => {
    const hostSession = fromUiSession(value);
    const hostFiles = files.map(fromUiAttachment);
    const input = createSendInput ? createSendInput(hostSession, content, hostFiles) : {
      content, attachments: hostFiles, workspaceCode: hostSession.workspaceCode,
      appCode: hostSession.appCode ?? config.appCode, taskCode: hostSession.taskCode ?? config.taskCode,
      agentCode: config.agentCode, clientCapabilities: { userInputRequests: true },
    };
    return input ? toUiInput(input) : null;
  }, [createSendInput, config.appCode, config.taskCode, config.agentCode]);
  return <ChatRuntimeWindow {...props} adapter={uiAdapter}
    errorDisplay="external" onError={onError ?? feedback.error} onInfo={onInfo ?? feedback.info}
    config={{ ...config, defaultSessionId: config.defaultSessionCode }}
    initialSession={session} pendingAttachments={attachments} createSendInput={createInput}
    onSessionChange={handleSession} onPendingAttachmentsChange={handleAttachments} onRunStateChange={handleRun} />;
}

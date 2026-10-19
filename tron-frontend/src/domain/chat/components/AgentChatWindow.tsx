import { defaultChatAnnotationLabels, zhChatAnnotationLabels } from "@eliteforge/agent-ui-chat-core";
import { useCallback, useImperativeHandle, useMemo, useRef, type Ref } from "react";
import { useTranslation } from "react-i18next";
import { useChatWorkRailLabels, useChatHistoryLabels } from "../hooks/useChatWorkRailLabels";
import { ChatRuntimeWindow, type ChatRuntimeWindowProps, type ChatRuntimeSubmitter } from "@eliteforge/agent-ui-chat-react";
import type * as Ui from "@eliteforge/agent-ui-chat-core";
import type { ChatAdapter, ChatClientConfig, ChatMessageAttachment, ChatRuntimeRunState, ChatSession, SendChatMessageInput } from "@tron/chat-core";
import { createAgentUiAdapter } from "@tron/chat-agent-ui-adapter";
import { fromUiAttachment, fromUiSession, toUiAttachment, toUiInput, toUiSession } from "@tron/chat-agent-ui-adapter";
import { useChatInputSuggestions } from "@/domain/chat/hooks/useChatInputSuggestions";
import { useChatFeedback } from "@/domain/chat/hooks/useChatFeedback";
import { useAguiWorkReplayPreferences } from "@/domain/settings/model/aguiWorkReplayPreferences";

type MappedProps = "adapter" | "attachmentDownloader" | "submitterRef" | "config" | "initialSession" | "pendingAttachments" | "onPendingAttachmentsChange" | "onSessionChange" | "onRunStateChange" | "createSendInput";
export interface AgentChatSubmitter {
  submit(input: Pick<SendChatMessageInput, "content" | "attachments">): Promise<boolean>;
}
export interface AgentChatWindowProps extends Omit<ChatRuntimeWindowProps, MappedProps> {
  adapter: ChatAdapter;
  attachmentDownloader?: (attachment: ChatMessageAttachment) => Promise<Blob>;
  submitterRef?: Ref<AgentChatSubmitter>;
  knowledgeBaseId?: string;
  config: ChatClientConfig;
  initialSession?: ChatSession | null;
  pendingAttachments?: ChatMessageAttachment[];
  onPendingAttachmentsChange?: (value: ChatMessageAttachment[]) => void;
  onSessionChange?: (value: ChatSession | null) => void;
  onRunStateChange?: (value: ChatRuntimeRunState) => void;
  createSendInput?: (session: ChatSession, content: string, attachments: ChatMessageAttachment[]) => SendChatMessageInput | null;
}

/** Product context adapter only. Rendering, streaming, queues, stop and replay belong to Agent UI. */
export function AgentChatWindow({ adapter, attachmentDownloader, submitterRef, config, knowledgeBaseId, initialSession, pendingAttachments,
  onPendingAttachmentsChange, onSessionChange, onRunStateChange, createSendInput, onError, onInfo, ...props }: AgentChatWindowProps) {
  const feedback = useChatFeedback();
  const runtimeSubmitterRef = useRef<ChatRuntimeSubmitter | null>(null);
  useImperativeHandle(submitterRef, () => ({
    submit: (input) => runtimeSubmitterRef.current?.submit({
      content: input.content, attachments: input.attachments?.map(toUiAttachment),
    }) ?? Promise.resolve(false),
  }), []);
  const { preferences: workPreferences } = useAguiWorkReplayPreferences();
  const inputSuggestions = useChatInputSuggestions(config.agentCode, initialSession?.sessionCode, initialSession?.sourceCode?.startsWith("__dude:") ? initialSession.sourceCode.slice(7) : undefined, config.workspaceCode, knowledgeBaseId);
  const { t, i18n } = useTranslation("pages");
  const workRailLabels = useChatWorkRailLabels();
  const historyLabels = useChatHistoryLabels();
  const userInputLabels = useMemo(() => ({
    ariaLabel: t("chat.userInput.ariaLabel"),
    progress: (current: number, total: number) => t("chat.userInput.progress", { current, total }),
    back: t("chat.userInput.back"), next: t("chat.userInput.next"),
    continue: t("chat.userInput.continue"), approve: t("chat.userInput.approve"),
    reject: t("chat.userInput.reject"), continuing: t("chat.userInput.continuing"),
    otherOptionLabel: t("chat.userInput.otherOptionLabel"),
    otherOptionDescription: t("chat.userInput.otherOptionDescription"),
    skipOptionLabel: t("chat.userInput.skipOptionLabel"),
    skipOptionDescription: t("chat.userInput.skipOptionDescription"),
    otherAnswerPlaceholder: t("chat.userInput.otherAnswerPlaceholder"),
    otherAnswerAriaLabel: (index: number) => t("chat.userInput.otherAnswerAriaLabel", { index }),
  }), [t]);
  const uiAdapter = useMemo(() => createAgentUiAdapter(adapter, config),
    [adapter, config.workspaceCode, config.appCode, config.taskCode, config.agentCode]);
  const session = useMemo(() => initialSession ? toUiSession(initialSession) : initialSession, [initialSession]);
  const attachments = useMemo(() => pendingAttachments?.map(toUiAttachment), [pendingAttachments]);
  const downloadAttachment = useMemo(() => attachmentDownloader
    ? (value: Ui.ChatMessageAttachment) => attachmentDownloader(fromUiAttachment(value))
    : undefined, [attachmentDownloader]);
  const handleSession = useCallback((value: Ui.ChatSession | null) =>
    onSessionChange?.(value ? fromUiSession(value) : null), [onSessionChange]);
  const handleAttachments = useCallback((value: Ui.ChatMessageAttachment[]) =>
    onPendingAttachmentsChange?.(value.map(fromUiAttachment)), [onPendingAttachmentsChange]);
  const handleRun = useCallback((value: Ui.ChatRuntimeRunState) => onRunStateChange?.({
    loading: value.loading, active: value.active, runCode: value.runId, sessionCode: value.sessionId, supportsSteer: value.supportsSteer,
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
  return <ChatRuntimeWindow {...props} adapter={uiAdapter} attachmentDownloader={downloadAttachment} submitterRef={runtimeSubmitterRef}
    errorDisplay="external" onError={onError ?? feedback.error} onInfo={onInfo ?? feedback.info}
    config={{ ...config, ...inputSuggestions, defaultSessionId: config.defaultSessionCode,
      showReasoningCards: workPreferences.showReasoningCards,
      runningEnterSubmitMode: workPreferences.runningEnterSubmitMode,
      inputPlaceholder: "",
      queuedInputPlaceholder: "",
      steeringInputPlaceholder: "",
      annotationLabels: i18n.language.startsWith("zh") ? zhChatAnnotationLabels : defaultChatAnnotationLabels,
      workRailLabels: config.workRailLabels ?? workRailLabels,
      historyLabels: config.historyLabels ?? historyLabels,
      userInputLabels: config.userInputLabels ?? userInputLabels,
    }}
    initialSession={session} pendingAttachments={attachments} createSendInput={createInput}
    onSessionChange={handleSession} onPendingAttachmentsChange={handleAttachments} onRunStateChange={handleRun} />;
}

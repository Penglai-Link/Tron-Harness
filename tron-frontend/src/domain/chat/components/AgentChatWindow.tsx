import { defaultChatAnnotationLabels, zhChatAnnotationLabels } from "@eliteforge/agent-ui-chat-core";
import { lazy, Suspense, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState, type Ref } from "react";
import { useTranslation } from "react-i18next";
import { useChatWorkRailLabels, useChatHistoryLabels } from "../hooks/useChatWorkRailLabels";
import { ChatRuntimeWindow, type ChatRuntimeWindowProps, type ChatRuntimeSubmitter } from "@eliteforge/agent-ui-chat-react";
import type * as Ui from "@eliteforge/agent-ui-chat-core";
import type { ChatAdapter, ChatClientConfig, ChatMessageAttachment, ChatRuntimeRunState, ChatSession, SendChatMessageInput } from "@tron/chat-core";
import { createAgentUiAdapter } from "@tron/chat-agent-ui-adapter";
import { fromUiAttachment, fromUiSession, toUiAttachment, toUiInput, toUiSession } from "@tron/chat-agent-ui-adapter";
import { useChatContextRefreshInterval } from "../hooks/useChatContextRefreshInterval";
import { useChatInputSuggestions } from "@/domain/chat/hooks/useChatInputSuggestions";
import { useChatFeedback } from "@/domain/chat/hooks/useChatFeedback";
import { useAguiWorkReplayPreferences } from "@/domain/settings/model/aguiWorkReplayPreferences";
import { latestChatComposerMetrics, type ChatComposerMetrics as ComposerMetrics } from "../model/chatComposerMetrics";
import type { ContextAssemblySnapshot } from "../model/contextAssemblyReport";
import { ChatComposerMetrics } from "./ChatComposerMetrics";
import { OPEN_CHAT_CONTEXT_EVENT } from "@/shared/lib/shellEvents";
import { requestApi } from "@/shared/api/httpClient";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import "./AgentChatWindow.css";

const ChatContextInspector = lazy(() => import("./ChatContextInspector").then(module => ({ default: module.ChatContextInspector })));

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
  onPendingAttachmentsChange, onSessionChange, onRunStateChange, createSendInput, onError, onInfo, footerAddon, ...props }: AgentChatWindowProps) {
  const feedback = useChatFeedback();
  const runtimeSubmitterRef = useRef<ChatRuntimeSubmitter | null>(null);
  const [activeSessionCode, setActiveSessionCode] = useState<string | null>(initialSession?.sessionCode ?? null);
  const [activeSession, setActiveSession] = useState<ChatSession | null>(initialSession ?? null);
  const initialSessionRef = useRef(initialSession);
  initialSessionRef.current = initialSession;
  const [contextOpen, setContextOpen] = useState(false);
  const [composerSnapshot, setComposerSnapshot] = useState<{ sessionCode: string; metrics: ComposerMetrics | null } | null>(null);
  const metricsRequestRef = useRef(0);
  const wasRunActiveRef = useRef(false);
  const [metricsRunSessionCode, setMetricsRunSessionCode] = useState<string | null>(null);
  const metricsRefreshKey = useChatContextRefreshInterval(activeSessionCode ?? "", Boolean(activeSessionCode && metricsRunSessionCode === activeSessionCode));
  useEffect(() => {
    setActiveSessionCode(initialSession?.sessionCode ?? null);
    setActiveSession(initialSession ?? null);
    setContextOpen(false);
  }, [initialSession?.sessionCode]);
  const refreshComposerMetrics = useCallback(async (sessionCode: string) => {
    const request = ++metricsRequestRef.current;
    const [eventsResult, contextResult] = await Promise.allSettled([
      adapter.loadAguiEvents(sessionCode),
      requestApi<{ snapshot: ContextAssemblySnapshot | null }>("GET /api/v1/chats/sessions/get-context-assembly", {
        queryParams: { sessionCode },
        headers: activeSession?.ownerDudeCode ? { "X-Dude-Code": activeSession.ownerDudeCode } : undefined,
      }),
    ]);
    const events = eventsResult.status === "fulfilled" ? eventsResult.value : [];
    const contextSnapshot = contextResult.status === "fulfilled" ? contextResult.value.snapshot : null;
    const metrics = latestChatComposerMetrics(events, contextSnapshot);
    if (request === metricsRequestRef.current) {
      setComposerSnapshot({ sessionCode, metrics });
    }
  }, [adapter, activeSession?.ownerDudeCode]);
  useEffect(() => {
    if (activeSessionCode) void refreshComposerMetrics(activeSessionCode);
  }, [activeSessionCode, metricsRefreshKey, refreshComposerMetrics]);
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
    approvalTitle: t("chat.userInput.approvalTitle"),
    approvalQuestion: (action: string) => t("chat.userInput.approvalQuestion", { action }),
    approvalFallbackQuestion: t("chat.userInput.approvalFallbackQuestion"),
    approveAll: t("chat.userInput.approveAll"),
    rejectAll: t("chat.userInput.rejectAll"),
    approvalTool: t("chat.userInput.approvalTool"),
    approvalExecution: t("chat.userInput.approvalExecution"),
    approvalReason: t("chat.userInput.approvalReason"),
    approvalDirectory: t("chat.userInput.approvalDirectory"),
    approvalParameters: t("chat.userInput.approvalParameters"),
    approvalMissingDetails: t("chat.userInput.approvalMissingDetails"),
    approvalScope: t("chat.userInput.approvalScope"),
    approvalBatch: t("chat.userInput.approvalBatch"),
    approvalBatchHint: t("chat.userInput.approvalBatchHint"),
    approvalWaiting: t("chat.userInput.approvalWaiting"),
    approvalRetry: t("chat.userInput.approvalRetry"),
    approvalChange: t("chat.userInput.approvalChange"),
    approved: t("chat.userInput.approved"),
    rejected: t("chat.userInput.rejected"),
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
  const handleSession = useCallback((value: Ui.ChatSession | null) => {
    setActiveSessionCode(value?.sessionId ?? null);
    const mapped = value ? fromUiSession(value) : null;
    const origin = initialSessionRef.current;
    // Generic completion responses omit host ownership. Retain it only for
    // the same session, including approval resumes and metric inspection.
    const nextSession = mapped && origin?.sessionCode === mapped.sessionCode
      ? { ...mapped, ownerDudeCode: mapped.ownerDudeCode ?? origin.ownerDudeCode, purpose: mapped.purpose ?? origin.purpose }
      : mapped;
    setActiveSession(nextSession);
    onSessionChange?.(nextSession);
  }, [onSessionChange]);
  const openContext = useCallback(() => {
    if (config.floating && activeSession?.ownerDudeCode) {
      setContextOpen(true);
      return;
    }
    window.dispatchEvent(new CustomEvent(OPEN_CHAT_CONTEXT_EVENT, { detail: activeSession }));
  }, [activeSession, config.floating]);
  const handleAttachments = useCallback((value: Ui.ChatMessageAttachment[]) =>
    onPendingAttachmentsChange?.(value.map(fromUiAttachment)), [onPendingAttachmentsChange]);
  const handleRun = useCallback((value: Ui.ChatRuntimeRunState) => {
    setMetricsRunSessionCode(value.active ? value.sessionId ?? null : null);
    if (wasRunActiveRef.current && !value.active && value.sessionId) {
      void refreshComposerMetrics(value.sessionId);
    }
    wasRunActiveRef.current = value.active;
    onRunStateChange?.({
      loading: value.loading, active: value.active, runCode: value.runId, sessionCode: value.sessionId, supportsSteer: value.supportsSteer, operationKind: value.operationKind,
    });
  }, [onRunStateChange, refreshComposerMetrics]);
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
  return <><ChatRuntimeWindow {...props} adapter={uiAdapter} attachmentDownloader={downloadAttachment} submitterRef={runtimeSubmitterRef}
    footerAddon={<>{footerAddon}<ChatComposerMetrics metrics={composerSnapshot?.sessionCode === activeSessionCode ? composerSnapshot.metrics : null} onOpen={openContext} /></>}
    errorDisplay="external" onError={onError ?? feedback.error} onInfo={onInfo ?? feedback.info}
    config={{ ...config, ...inputSuggestions, defaultSessionId: config.defaultSessionCode,
      resizeHint: t("chat.floating.resizeHint"),
      locale: config.locale ?? i18n.language,
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
    onSessionChange={handleSession} onPendingAttachmentsChange={handleAttachments} onRunStateChange={handleRun} />
    <Dialog open={contextOpen} onOpenChange={setContextOpen}>
      <DialogContent className="flex h-[min(80dvh,900px)] flex-col sm:max-w-3xl">
        <DialogHeader><DialogTitle>{t("chat.shortTermMemory.shortTermMemoryTab")}</DialogTitle></DialogHeader>
        <div className="relative min-h-0 flex-1">
          {contextOpen && activeSession?.ownerDudeCode && <Suspense fallback={null}><ChatContextInspector session={activeSession} /></Suspense>}
        </div>
      </DialogContent>
    </Dialog>
  </>;
}

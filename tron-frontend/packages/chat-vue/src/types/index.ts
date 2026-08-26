import type {
  ChatAdapter,
  ChatAttachmentLabels,
  ChatAguiStreamEvent,
  ChatClientConfig,
  ChatDebugOptions,
  ChatFloatingState,
  ChatMessage,
  ChatMessageAttachment,
  ChatQueuedSubmission,
  ChatRunningSubmitMode,
  ChatRuntimeRunState,
  ChatSession,
  ChatSubmitMode,
  ChatUserInputAnswer,
  ChatUserInputLabels,
  ChatUserInputRequest,
  ChatWorkRailLabels,
} from "@tron/chat-core";

export interface TronChatPanelProps {
  attachmentDownloader?: (attachment: ChatMessageAttachment) => Promise<Blob>;
  attachmentLabels?: ChatAttachmentLabels;
  autoFocusComposer?: boolean;
  composerHeader?: unknown;
  createSessionLabel?: string;
  currentUserDisplayName?: string;
  debugOptions?: ChatDebugOptions;
  disabledInput?: boolean;
  draft: string;
  emptyDescription?: string;
  emptyTitle?: string;
  formatAnnotationCount?: (count: number) => string;
  inputPlaceholder?: string;
  liveAguiAnchorMessageCodes?: Record<string, string>;
  liveAguiEvents?: ChatAguiStreamEvent[];
  liveAguiPendingAnchorMessageCode?: string;
  liveAguiPending?: boolean;
  messages: ChatMessage[];
  onCreateSession?: () => void;
  onNewSession?: () => void;
  onDraftChange: (value: string) => void;
  onPasteAttachment?: (file: File) => void | Promise<void>;
  onEditMessage?: (content: string) => void;
  onEditQueuedMessage?: (id: string, content: string) => void;
  onRemoveQueuedMessage?: (id: string) => void;
  onSend: (mode?: ChatSubmitMode) => void;
  onStop?: () => void;
  protocolEvents?: ChatAguiStreamEvent[];
  queueDisabled?: boolean;
  queuedLabel?: string;
  queuedMessages?: ChatQueuedSubmission[];
  runningSubmitModes?: ChatRunningSubmitMode[];
  sendDisabled?: boolean;
  sendLabel?: string;
  sending?: boolean;
  session?: ChatSession | null;
  showAgui?: boolean;
  showHeader?: boolean;
  showStop?: boolean;
  steerDisabled?: boolean;
  stopLabel?: string;
  title?: string;
  userInputLabels?: ChatUserInputLabels;
  workRailLabels?: ChatWorkRailLabels;
  userInputRequests?: ChatUserInputRequest[];
  userInputRequest?: ChatUserInputRequest | null;
  userInputAnswerPending?: boolean;
  onAnswerUserInput?: (answers: ChatUserInputAnswer[]) => void;
}

export type ToastLikeCallback = (message: string, title?: string) => void;

export interface TronChatRuntimeWindowProps {
  adapter?: ChatAdapter;
  attachmentLabels?: ChatAttachmentLabels;
  autoFocusComposer?: boolean;
  config: ChatClientConfig;
  composerHeader?: unknown;
  currentUserDisplayName?: string;
  draft?: string;
  debugOptions?: ChatDebugOptions;
  emptyDescription?: string;
  emptyTitle?: string;
  floatingSubtitle?: string;
  floatingTitle?: string;
  formatAnnotationCount?: (count: number) => string;
  footerAddon?: unknown;
  initialSession?: ChatSession | null;
  onCreateSession?: () => void;
  onDraftChange?: (draft: string) => void;
  onError?: ToastLikeCallback;
  onInfo?: ToastLikeCallback;
  onPendingAttachmentsChange?: (attachments: ChatMessageAttachment[]) => void;
  onRunStateChange?: (state: ChatRuntimeRunState) => void;
  onSessionChange?: (session: ChatSession | null) => void;
  pendingAttachments?: ChatMessageAttachment[];
  createSendInput?: (
    session: ChatSession,
    content: string,
    attachments: ChatMessageAttachment[],
  ) => import("@tron/chat-core").SendChatMessageInput | null;
  showAttachmentControl?: boolean;
  showHeader?: boolean;
  onWindowStateChange?: (state: ChatFloatingState) => void;
  windowState?: ChatFloatingState;
}

export interface TronChatWindowProps extends Omit<
  TronChatPanelProps,
  "showHeader"
> {
  onStateChange?: (state: ChatFloatingState) => void;
  state?: ChatFloatingState;
  storageKey?: string;
  title?: string;
  titleSubtitle?: string;
}

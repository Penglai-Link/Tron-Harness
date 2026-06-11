import {
  ArrowDown,
  Bot,
  BrainCircuit,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  LoaderCircle,
  MessageSquareText,
  Pencil,
  Send,
  Square,
  Terminal,
  UserRound,
} from "lucide-react";
import {
  aguiEventMillis,
  aguiEventType,
  buildAguiUserWorkCards,
  buildTranscriptPresentation,
  describeDebugEvent,
  deriveReasoningPreview,
  formatJsonBlockValue,
  formatTime,
  sessionAgentTagLabel,
  toolSummaryLabel,
  type AguiUserWorkCard,
  type ChatAguiStreamEvent,
  type ChatDebugOptions,
  type ChatMessage,
  type ChatSession,
  type ToolGroupWorkCard,
  type ToolWorkCard,
} from "@tron/chat-core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  type Ref,
  Fragment,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

function cx(...values: Array<string | false | null | undefined>) {
  return values.filter(Boolean).join(" ");
}

function insertNewlineAtCursor(textarea: HTMLTextAreaElement) {
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;
  const nextValue = `${textarea.value.slice(0, start)}\n${textarea.value.slice(end)}`;
  textarea.value = nextValue;
  textarea.selectionStart = start + 1;
  textarea.selectionEnd = start + 1;
  return nextValue;
}

async function copyPlainText(value: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value);
    return;
  }
  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  document.execCommand("copy");
  document.body.removeChild(textarea);
}

function MarkdownBlock({ content }: { content: string }) {
  return (
    <div className="tron-chat__markdown tron-chat__message-body">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>
        {content || "Waiting for assistant output..."}
      </ReactMarkdown>
    </div>
  );
}

function CopyValueButton({
  ariaLabel,
  title,
  value,
}: {
  ariaLabel: string;
  title: string;
  value: string;
}) {
  const [copied, setCopied] = useState(false);
  const timerRef = useRef<number | null>(null);

  useEffect(() => () => {
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
  }, []);

  async function handleCopy() {
    if (!value.trim()) return;
    await copyPlainText(value);
    setCopied(true);
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <button
      className="tron-chat__icon-button"
      type="button"
      aria-label={ariaLabel}
      disabled={!value.trim()}
      title={title}
      onClick={() => void handleCopy()}
    >
      {copied ? <CheckCircle2 size={14} /> : <Copy size={14} />}
    </button>
  );
}

function ChatBubble({
  currentUserDisplayName,
  message,
  onEditMessage,
}: {
  currentUserDisplayName?: string;
  message: ChatMessage;
  onEditMessage?: (content: string) => void;
}) {
  const isUser = message.role === "USER";
  const actorName = isUser ? (currentUserDisplayName || message.actor) : message.actor;
  return (
    <article className={cx("tron-chat__message", isUser && "tron-chat__message--user")}>
      <div className="tron-chat__message-header">
        <div className="tron-chat__message-actor">
          {isUser ? <UserRound size={16} /> : <Bot size={16} />}
          <span>{actorName}</span>
        </div>
        <div className="tron-chat__message-actions">
          <CopyValueButton
            ariaLabel="Copy raw markdown"
            title="Copy raw markdown"
            value={message.content}
          />
          {isUser && onEditMessage ? (
            <button
              className="tron-chat__icon-button"
              type="button"
              aria-label="Edit message"
              onClick={() => onEditMessage(message.content)}
            >
              <Pencil size={14} />
            </button>
          ) : null}
          <span className="tron-chat__message-time">{formatTime(message.createdAt)}</span>
        </div>
      </div>
      {isUser ? (
        <div className="tron-chat__message-body">{message.content}</div>
      ) : (
        <MarkdownBlock content={message.content} />
      )}
    </article>
  );
}

function formatAguiDuration(milliseconds: number) {
  if (milliseconds < 1000) return "<1s";
  const totalSeconds = Math.max(1, Math.round(milliseconds / 1000));
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (totalMinutes < 60) return seconds ? `${totalMinutes}m ${seconds}s` : `${totalMinutes}m`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes ? `${hours}h ${minutes}m` : `${hours}h`;
}

function aguiWorkDuration(events: ChatAguiStreamEvent[]) {
  const times = events
    .map(aguiEventMillis)
    .filter((value): value is number => typeof value === "number");
  if (times.length < 2) return "<1s";
  return formatAguiDuration(Math.max(...times) - Math.min(...times));
}

function terminalAguiEvent(events: ChatAguiStreamEvent[]) {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const normalized = aguiEventType(events[index]).toUpperCase();
    if (normalized.includes("RUN_FINISHED") || normalized.includes("RUN_ERROR")) return events[index];
  }
  return null;
}

function hasToolParameterPayload(value: unknown) {
  if (value == null) return false;
  if (typeof value === "string") return Boolean(value.trim());
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === "object") return true;
  return true;
}

function JsonBlock({ value }: { value: unknown }) {
  const preview = formatJsonBlockValue(value);
  if (!preview) return null;
  return <pre className="tron-chat__result-block">{preview}</pre>;
}

function ToolCard({
  card,
  nested = false,
  stepNumber,
}: {
  card: ToolWorkCard;
  nested?: boolean;
  stepNumber?: number;
}) {
  const complete = card.ended || Boolean(card.result);
  const running = card.started && !complete;
  const hasParameters = hasToolParameterPayload(card.args);
  const parametersText = hasParameters ? formatJsonBlockValue(card.args) : "";
  const resultText = card.result ? formatJsonBlockValue(card.result) : "";
  const toolLabel = toolSummaryLabel(card.toolName, card.args);
  const nestedToolLabel = `${complete ? "Ran" : "Running"} ${toolLabel.replace(/^Run\s+/i, "")}`;
  const [expanded, setExpanded] = useState(() => running);
  const [parametersExpanded, setParametersExpanded] = useState(false);
  const logRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (running) setExpanded(true);
    else if (complete) setExpanded(false);
  }, [complete, running]);

  useEffect(() => {
    if (!expanded || complete || !card.logs.length) return;
    const element = logRef.current;
    if (!element) return;
    const frame = window.requestAnimationFrame(() => {
      element.scrollTop = element.scrollHeight;
    });
    return () => window.cancelAnimationFrame(frame);
  }, [card.logs.length, complete, expanded]);

  return (
    <div className={cx("tron-chat__tool-row", !nested && "tron-chat__tool-row--standalone")}>
      <button
        className={cx("tron-chat__tool-toggle", nested && "tron-chat__tool-toggle--nested")}
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
      >
        <span className="tron-chat__tool-toggle-main">
          <span className={cx("tron-chat__tool-badge", nested ? "tron-chat__tool-badge--nested" : "tron-chat__tool-badge--standalone")}>
            {typeof stepNumber === "number"
              ? stepNumber
              : complete
                ? <CheckCircle2 size={14} />
                : running
                  ? <LoaderCircle size={14} className="animate-spin" />
                  : <Terminal size={14} />}
          </span>
          <span className="tron-chat__tool-toggle-text">
            <span className="tron-chat__tool-title-row">
              <span className={cx("tron-chat__tool-title", nested && "tron-chat__tool-title--nested")}>
                {nested ? nestedToolLabel : toolLabel}
              </span>
              {nested ? null : (
                <span className={cx("tron-chat__tool-status", complete ? "tron-chat__tool-status--done" : "tron-chat__tool-status--running")}>
                  {complete ? "done" : "running"}
                </span>
              )}
            </span>
          </span>
        </span>
        <span className="tron-chat__tool-chevron">
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </button>
      {expanded ? (
        <div className={cx("tron-chat__tool-body", nested ? "tron-chat__tool-body--nested" : "tron-chat__tool-body--standalone")}>
          <div className="tron-chat__tool-sections">
            {hasParameters ? (
              <section>
                <div className="tron-chat__tool-section-header" style={{ marginBottom: parametersExpanded ? 8 : 0 }}>
                  <button
                    className="tron-chat__tool-subtoggle"
                    type="button"
                    aria-expanded={parametersExpanded}
                    onClick={() => setParametersExpanded((value) => !value)}
                  >
                    <span className="tron-chat__tool-subtoggle-label">
                      {parametersExpanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                      <span>Parameters</span>
                    </span>
                  </button>
                  <CopyValueButton
                    ariaLabel="Copy parameters"
                    title="Copy parameters"
                    value={parametersText}
                  />
                </div>
                {parametersExpanded ? <JsonBlock value={card.args} /> : null}
              </section>
            ) : null}
            {complete ? (
              <section>
                <div className="tron-chat__tool-section-header" style={{ marginBottom: 8 }}>
                  <h3 className="tron-chat__tool-section-title">Result</h3>
                  <CopyValueButton
                    ariaLabel="Copy result"
                    title="Copy result"
                    value={resultText}
                  />
                </div>
                {card.result ? (
                  <div className="max-h-64" style={{ maxHeight: 256, overflowY: "auto" }}>
                    <JsonBlock value={card.result} />
                  </div>
                ) : (
                  <div className="tron-chat__result-block">No result payload.</div>
                )}
              </section>
            ) : (
              <section>
                <h3 className="tron-chat__tool-section-title" style={{ marginBottom: 8 }}>Log</h3>
                <div
                  ref={logRef}
                  className="tron-chat__code-block max-h-64"
                  style={{ maxHeight: 256, overflowY: "auto" }}
                >
                  {card.logs.length ? card.logs.map((line, index) => (
                    <p key={`${card.id}-log-${index}`} style={{ margin: 0, whiteSpace: "pre-wrap" }}>{line}</p>
                  )) : <p style={{ margin: 0 }}>Waiting for tool activity...</p>}
                </div>
              </section>
            )}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function ToolGroupCard({ card }: { card: ToolGroupWorkCard }) {
  const toolCards = card.toolCards.filter((toolCard) => toolCard.started);
  const hasRunningTool = toolCards.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result);
  const [expanded, setExpanded] = useState(() => hasRunningTool);

  useEffect(() => {
    setExpanded(hasRunningTool);
  }, [hasRunningTool]);

  if (!toolCards.length) return null;

  return (
    <section className="tron-chat__rail-entry" aria-label="Tool run steps">
      <button className="tron-chat__rail-entry-toggle" type="button" aria-expanded={expanded} onClick={() => setExpanded((value) => !value)}>
        <span className="tron-chat__rail-entry-main">
          <span className="tron-chat__rail-entry-icon">
            <Terminal size={15} className={hasRunningTool ? "animate-pulse" : undefined} />
          </span>
          <span className="tron-chat__rail-entry-summary tron-chat__rail-entry-summary--strong">
            {`Ran ${toolCards.length} ${toolCards.length === 1 ? "command" : "commands"}`}
          </span>
        </span>
        <span className="tron-chat__rail-entry-chevron">
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </button>
      {expanded ? (
        <ol className="tron-chat__rail-entry-list">
          {toolCards.map((toolCard, index) => (
            <li key={toolCard.id}>
              <ToolCard card={toolCard} nested stepNumber={index + 1} />
            </li>
          ))}
        </ol>
      ) : null}
    </section>
  );
}

function ReasoningCard({ card }: { card: Extract<AguiUserWorkCard, { kind: "reasoning" }>["value"] }) {
  const hasRunningTool = card.toolCards.some((toolCard) => toolCard.started && !toolCard.ended && !toolCard.result);
  const shouldStayExpanded = !card.ended || hasRunningTool;
  const summary = deriveReasoningPreview(card.content) || (card.ended ? "Reasoning captured." : "Thinking...");
  const [expanded, setExpanded] = useState(() => shouldStayExpanded);

  useEffect(() => {
    setExpanded(shouldStayExpanded);
  }, [shouldStayExpanded]);

  return (
    <section className="tron-chat__rail-entry" aria-label="Thinking">
      <button className="tron-chat__rail-entry-toggle" type="button" aria-expanded={expanded} onClick={() => setExpanded((value) => !value)}>
        <span className="tron-chat__rail-entry-main">
          <span className="tron-chat__rail-entry-icon">
            <BrainCircuit size={15} className={card.ended ? undefined : "animate-pulse"} />
          </span>
          <span className={cx("tron-chat__rail-entry-summary", !card.ended && "tron-chat__rail-entry-summary--pending")}>
            {summary}
          </span>
        </span>
        <span className="tron-chat__rail-entry-chevron">
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </span>
      </button>
      {expanded ? (
        <div className="tron-chat__rail-entry-body">
          {card.content ? (
            <div className="tron-chat__rail-detail">
              <MarkdownBlock content={card.content} />
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function AguiRunWorkRail({
  debugEvents,
  events,
}: {
  debugEvents: string[];
  events: ChatAguiStreamEvent[];
}) {
  const userCards = useMemo(() => buildAguiUserWorkCards(events), [events]);
  const terminalEvent = useMemo(() => terminalAguiEvent(events), [events]);
  const [expanded, setExpanded] = useState(() => !terminalEvent);

  useEffect(() => {
    setExpanded(!terminalEvent);
  }, [terminalEvent]);

  if (!userCards.length && !debugEvents.length) return null;
  return (
    <section className="tron-chat__rail" aria-label="AG-UI run work">
      {terminalEvent ? (
        <button className="tron-chat__rail-summary" type="button" aria-expanded={expanded} onClick={() => setExpanded((value) => !value)}>
          <span className="tron-chat__rail-summary-label">
            <span>{`Worked for ${aguiWorkDuration(events)}`}</span>
          </span>
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </button>
      ) : null}
      {(!terminalEvent || expanded) ? (
        <div className="tron-chat__rail-stack" style={{ marginTop: terminalEvent ? 8 : 0 }}>
          {userCards.length ? (
            <section className="tron-chat__rail" aria-label="Agent work rail" style={{ maxWidth: "none", paddingLeft: 0 }}>
              <div className="tron-chat__rail-card-list">
                {userCards.map((card) => (
                  card.kind === "reasoning"
                    ? <ReasoningCard card={card.value} key={card.value.id} />
                    : card.kind === "toolGroup"
                      ? <ToolGroupCard card={card.value} key={card.value.id} />
                      : <ToolCard card={card.value} key={card.value.id} />
                ))}
              </div>
            </section>
          ) : null}
          {debugEvents.length ? (
            <section className="tron-chat__rail" aria-label="AG-UI debug event cards" style={{ maxWidth: "none", paddingLeft: 0 }}>
              <div className="tron-chat__debug-list">
                {debugEvents.map((event, index) => (
                  <div className="tron-chat__debug-item" key={`${event}-${index}`}>
                    <div className="tron-chat__debug-summary">{event}</div>
                  </div>
                ))}
              </div>
            </section>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function WaitingRail() {
  return (
    <section className="tron-chat__rail" aria-label="Waiting for model">
      <div className="tron-chat__waiting">
        <span className="tron-chat__waiting-line" />
        <span className="tron-chat__rail-toggle-label">
          <LoaderCircle size={14} className="animate-spin" />
          <span>Waiting for model...</span>
        </span>
        <span className="tron-chat__waiting-line" />
      </div>
    </section>
  );
}

export interface TronChatPanelProps {
  autoFocusComposer?: boolean;
  createSessionLabel?: string;
  currentUserDisplayName?: string;
  composerRef?: Ref<HTMLTextAreaElement>;
  debugOptions?: ChatDebugOptions;
  disabledInput?: boolean;
  draft: string;
  emptyDescription?: string;
  emptyTitle?: string;
  footerAddon?: ReactNode;
  headerContent?: ReactNode;
  inputPlaceholder?: string;
  liveAguiAnchorMessageCodes?: Record<string, string>;
  liveAguiEvents?: ChatAguiStreamEvent[];
  liveAguiPending?: boolean;
  messages: ChatMessage[];
  onCreateSession?: () => void;
  onDraftChange: (value: string) => void;
  onEditMessage?: (content: string) => void;
  onSend: () => void;
  onStop?: () => void;
  protocolEvents?: ChatAguiStreamEvent[];
  sendDisabled?: boolean;
  sendLabel?: string;
  sending?: boolean;
  session?: ChatSession | null;
  showAgui?: boolean;
  showHeader?: boolean;
  sidebarContent?: ReactNode;
  stopLabel?: string;
  title?: string;
}

export function TronChatPanel({
  autoFocusComposer = false,
  createSessionLabel = "Create session",
  currentUserDisplayName,
  composerRef,
  debugOptions,
  disabledInput = false,
  draft,
  emptyDescription = "Create a session to start chatting.",
  emptyTitle = "No session selected",
  footerAddon,
  headerContent,
  inputPlaceholder = "Enter to send, Shift+Enter for newline",
  liveAguiAnchorMessageCodes,
  liveAguiEvents,
  liveAguiPending,
  messages,
  onCreateSession,
  onDraftChange,
  onEditMessage,
  onSend,
  onStop,
  protocolEvents,
  sendDisabled,
  sendLabel = "Send",
  sending = false,
  session,
  showAgui = true,
  showHeader = true,
  sidebarContent,
  stopLabel = "Stop",
  title,
}: TronChatPanelProps) {
  const transcriptRef = useRef<HTMLElement | null>(null);
  const composerElementRef = useRef<HTMLTextAreaElement | null>(null);
  const [showJumpButton, setShowJumpButton] = useState(false);
  const autoStickToBottomRef = useRef(true);
  const presentation = useMemo(() => (
    buildTranscriptPresentation({
      debugOptions,
      liveAguiAnchorMessageCodes,
      liveAguiEvents,
      liveAguiPending,
      messages,
      protocolEvents,
      sessionCode: session?.sessionCode ?? "",
    })
  ), [debugOptions, liveAguiAnchorMessageCodes, liveAguiEvents, liveAguiPending, messages, protocolEvents, session?.sessionCode]);

  function scrollToBottom(behavior: ScrollBehavior = "auto") {
    const element = transcriptRef.current;
    if (!element) return;
    element.scrollTo({ behavior, top: element.scrollHeight });
    autoStickToBottomRef.current = true;
    setShowJumpButton(false);
  }

  function handleScroll() {
    const element = transcriptRef.current;
    if (!element) return;
    const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
    autoStickToBottomRef.current = distanceFromBottom <= 96;
    setShowJumpButton(distanceFromBottom > 160);
  }

  useEffect(() => {
    if (autoStickToBottomRef.current) scrollToBottom("auto");
    else handleScroll();
  }, [messages, presentation]);

  useEffect(() => {
    autoStickToBottomRef.current = true;
    setShowJumpButton(false);
  }, [session?.sessionCode]);

  useLayoutEffect(() => {
    if (!autoFocusComposer || !session?.sessionCode) return;
    composerElementRef.current?.focus();
  }, [autoFocusComposer, session?.sessionCode]);

  function setComposerNode(node: HTMLTextAreaElement | null) {
    composerElementRef.current = node;
    if (!composerRef) return;
    if (typeof composerRef === "function") {
      composerRef(node);
      return;
    }
    composerRef.current = node;
  }

  return (
    <div className="tron-chat">
      {showHeader ? (
        <header className="tron-chat__header">
          <div className="tron-chat__header-content">
            {headerContent ?? (
              <>
                <span className="tron-chat__title">{title ?? session?.title ?? "Chat"}</span>
                {session ? (
                  <span className="tron-chat__subtitle">
                    {session.sessionCode} · {sessionAgentTagLabel(session.agentCode)}
                  </span>
                ) : null}
              </>
            )}
          </div>
        </header>
      ) : null}
      {session ? (
        <>
          <div className={cx("tron-chat__content", Boolean(sidebarContent) && "tron-chat__content--with-sidebar")}>
            <section className="tron-chat__transcript" aria-label="Chat transcript" ref={transcriptRef} onScroll={handleScroll}>
              <div className="tron-chat__transcript-inner">
                {messages.length ? messages.map((message) => (
                  <Fragment key={message.messageCode}>
                    <ChatBubble
                      currentUserDisplayName={currentUserDisplayName}
                      message={message}
                      onEditMessage={onEditMessage}
                    />
                    {showAgui && message.messageCode === presentation.waitingForModelAnchorMessageCode ? <WaitingRail /> : null}
                    {showAgui ? (presentation.workRailsByMessageCode.get(message.messageCode) ?? []).map((run) => (
                      <AguiRunWorkRail
                        debugEvents={(presentation.debugEventsByRunCode.get(run.runCode) ?? []).map(describeDebugEvent)}
                        events={run.events as ChatAguiStreamEvent[]}
                        key={run.runCode}
                      />
                    )) : null}
                  </Fragment>
                )) : null}
              </div>
            </section>
            {sidebarContent ? <aside className="tron-chat__sidebar">{sidebarContent}</aside> : null}
          </div>
          <footer className="tron-chat__footer">
            {showJumpButton ? (
              <button className="tron-chat__button tron-chat__jump" type="button" aria-label="Jump to latest message" onClick={() => scrollToBottom("smooth")}>
                <ArrowDown size={16} />
              </button>
            ) : null}
            <div className="tron-chat__composer">
              <textarea
                autoFocus={autoFocusComposer}
                key={session?.sessionCode ?? "composer"}
                ref={setComposerNode}
                className="tron-chat__textarea"
                disabled={disabledInput}
                placeholder={inputPlaceholder}
                value={draft}
                onChange={(event) => onDraftChange(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key !== "Enter" || event.nativeEvent.isComposing) return;
                  event.preventDefault();
                  if (event.shiftKey) {
                    onDraftChange(insertNewlineAtCursor(event.currentTarget));
                    return;
                  }
                  if (sending && onStop) {
                    onStop();
                    return;
                  }
                  onSend();
                }}
              />
              <div className="tron-chat__composer-row">
                <div className="tron-chat__composer-addon">{footerAddon}</div>
                <button
                  className={cx("tron-chat__button", sending && "tron-chat__button--danger")}
                  type="button"
                  disabled={disabledInput || (!sending && sendDisabled)}
                  onClick={() => {
                    if (sending && onStop) {
                      onStop();
                      return;
                    }
                    onSend();
                  }}
                >
                  {sending ? <Square size={14} /> : <Send size={15} />}
                  <span>{sending ? stopLabel : sendLabel}</span>
                </button>
              </div>
            </div>
          </footer>
        </>
      ) : (
        <div className={cx("tron-chat__content", Boolean(sidebarContent) && "tron-chat__content--with-sidebar")}>
          <section className="tron-chat__transcript">
            <div className="tron-chat__empty">
              <div className="tron-chat__empty-inner">
                <div className="tron-chat__empty-title">{emptyTitle}</div>
                <div className="tron-chat__empty-description">{emptyDescription}</div>
                {onCreateSession ? (
                  <div style={{ marginTop: 16 }}>
                    <button className="tron-chat__button" type="button" onClick={onCreateSession}>
                      {createSessionLabel}
                    </button>
                  </div>
                ) : null}
              </div>
            </div>
          </section>
          {sidebarContent ? <aside className="tron-chat__sidebar">{sidebarContent}</aside> : null}
        </div>
      )}
    </div>
  );
}

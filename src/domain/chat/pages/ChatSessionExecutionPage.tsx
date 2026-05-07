import { useState } from "react";
import {
  Bot,
  CircleCheck,
  ClipboardList,
  Layers3,
  Paperclip,
  Plus,
  Send,
  Terminal,
  Wrench,
} from "lucide-react";

import { Badge } from "@/shared/ui/Badge";
import { Button } from "@/shared/ui/Button";
import {
  InspectorPanel,
  SegmentedTabs,
  SurfaceCard,
  TextArea,
  TronShell,
} from "@/shared";

import { cn } from "@/shared/lib/utils";
import {
  useChatMessages,
  useChatSessionGroups,
  type ChatMessage,
} from "@/domain/chat/api";

function IconButton({ label, children, onClick }: { label: string; children: React.ReactNode; onClick?: () => void }) {
  return (
    <button className="inline-flex size-8 items-center justify-center rounded-lg text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)]" type="button" aria-label={label} onClick={onClick}>
      {children}
    </button>
  );
}

function Notice({ value }: { value: string }) {
  return <p className="rounded-lg bg-[var(--color-surface-container-low)] px-3 py-2 text-[13px] text-[var(--color-on-surface-variant)]">{value}</p>;
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 text-sm">
      <span className="text-[var(--color-on-surface-variant)]">{label}</span>
      <span className="font-mono text-[13px]">{value}</span>
    </div>
  );
}

function ChatSessionPage({ variant }: { variant: "reasoning" | "execution" }) {
  const groups = useChatSessionGroups();
  const messages = useChatMessages(variant);
  const [activeSession, setActiveSession] = useState("primary");
  const [inspectorTab, setInspectorTab] = useState("info");
  const [composer, setComposer] = useState("");
  const [notice, setNotice] = useState("Composer ready.");

  return (
    <TronShell
      activeItem="Chats"
      searchPlaceholder="Search tasks..."
      contentClassName="!p-0"
      rightPanel={
        <InspectorPanel title="Utility Hub">
          <SegmentedTabs value={inspectorTab} onValueChange={setInspectorTab} items={[{ label: "Info", value: "info" }, { label: "Tools", value: "tools" }, { label: "Files", value: "files" }]} />
          <SurfaceCard className="rounded-xl p-4 shadow-none">
            {inspectorTab === "info" ? (
              <div className="space-y-3">
                <KeyValue label="Active Model" value="tron-opus-v2" />
                <KeyValue label="Tokens Used" value="14,285" />
                <KeyValue label="Agents" value="3 active" />
              </div>
            ) : inspectorTab === "tools" ? (
              <div className="space-y-2 text-sm">
                <p>QueryTelemetry</p>
                <p>CodeSearch</p>
                <p>BuildRunner</p>
              </div>
            ) : (
              <div className="space-y-2 text-sm">
                <p>auth/token_validator.go</p>
                <p>deployments/pr-1042.log</p>
              </div>
            )}
          </SurfaceCard>
        </InspectorPanel>
      }
    >
      <div className="grid h-[calc(100vh-65px)] min-h-[720px] grid-cols-[260px_minmax(0,1fr)] bg-white">
        <aside className="border-r border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)]">
          <div className="flex items-center justify-between border-b border-[var(--color-outline-variant)] p-4">
            <h2 className="text-lg font-semibold">Sessions</h2>
            <IconButton label="New chat" onClick={() => setNotice("New chat requested")}>
              <Plus size={16} />
            </IconButton>
          </div>
          <div className="space-y-5 p-3">
            {groups.map((group) => (
              <section key={group.task}>
                <p className="mb-2 flex items-center gap-2 px-2 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
                  <Layers3 size={13} />
                  Task: {group.task}
                </p>
                <div className="space-y-1">
                  {group.sessions.map((session) => (
                    <button className={cn("flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm", activeSession === session.id ? "bg-[var(--color-secondary-container)] font-medium" : "text-[var(--color-on-surface-variant)] hover:bg-[var(--color-hover-surface)]")} type="button" key={session.id} onClick={() => setActiveSession(session.id)}>
                      <ClipboardList size={15} />
                      {session.title}
                    </button>
                  ))}
                </div>
              </section>
            ))}
          </div>
        </aside>

        <section className="flex min-w-0 flex-col">
          <header className="flex items-center justify-between border-b border-[var(--color-outline-variant)] px-5 py-4">
            <div>
              <h1 className="text-lg font-semibold">Primary Investigation</h1>
              <p className="text-sm text-[var(--color-on-surface-variant)]">Task: 8842-Alpha · 3 Agents Active · {variant === "reasoning" ? "Reasoning view" : "Execution view"}</p>
            </div>
            <Badge tone="accent">{activeSession}</Badge>
          </header>
          <div className="flex-1 space-y-4 overflow-y-auto bg-[var(--color-surface-container-low)] p-5">
            {messages.map((message) => (
              <ChatBubble key={message.id} message={message} />
            ))}
            <Notice value={notice} />
          </div>
          <div className="border-t border-[var(--color-outline-variant)] bg-white p-4">
            <div className="rounded-xl border border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] p-3">
              <TextArea className="min-h-20 border-0 bg-transparent p-0 focus:ring-0" placeholder="Shift + Enter to new line" value={composer} onChange={(event) => setComposer(event.target.value)} />
              <div className="mt-3 flex items-center justify-between">
                <div className="flex gap-1">
                  <IconButton label="Attach file" onClick={() => setNotice("Attach file opened")}>
                    <Paperclip size={16} />
                  </IconButton>
                  <IconButton label="Build context" onClick={() => setNotice("Build context requested")}>
                    <Wrench size={16} />
                  </IconButton>
                </div>
                <Button size="sm" onClick={() => setNotice(composer.trim() ? `Queued message: ${composer.trim().slice(0, 40)}` : "Message send requires content")}>
                  <Send size={15} />
                  Send
                </Button>
              </div>
            </div>
          </div>
        </section>
      </div>
    </TronShell>
  );
}

function ChatBubble({ message }: { message: ChatMessage }) {
  const isUser = message.kind === "user";
  return (
    <article className={cn("max-w-3xl rounded-xl border p-4", isUser ? "ml-auto border-[var(--color-primary)] bg-white" : "border-[var(--color-card-border)] bg-white")}>
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          {message.kind === "tool" || message.kind === "code" ? <Terminal size={16} /> : isUser ? <CircleCheck size={16} /> : <Bot size={16} />}
          <span className="text-sm font-semibold">{message.actor}</span>
          {message.meta ? <Badge tone="neutral">{message.meta}</Badge> : null}
        </div>
        <span className="text-[12px] text-[var(--color-on-surface-variant)]">{message.time}</span>
      </div>
      <pre className="whitespace-pre-wrap font-sans text-sm leading-6 text-[var(--color-on-surface)]">{message.body}</pre>
    </article>
  );
}

export function ChatSessionExecutionPage() {
  return <ChatSessionPage variant="execution" />;
}

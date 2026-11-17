import type { ChatSession } from "@tron/chat-core";
import { useChatAguiEvents, useChatMessageListBySession } from "@/domain/chat/hooks";
import { useChatContextRefreshInterval } from "@/domain/chat/hooks/useChatContextRefreshInterval";
import { useTabVisibility } from "@/components/editable-tabs";
import { RightContextSidebar } from "@/domain/chat/pages/ChatManagementPage";

/** Reuse the session's short-term-memory view outside the chat management page (for PersonAI). */
export function ChatContextInspector({ session }: { session: ChatSession | null }) {
  const sessionCode = session?.sessionCode ?? "";
  const assistantCode = session?.ownerAssistantCode ?? undefined;
  const visible = useTabVisibility();
  const refreshKey = useChatContextRefreshInterval(sessionCode, visible, 3_000);
  const events = useChatAguiEvents(sessionCode, refreshKey, assistantCode);
  const messages = useChatMessageListBySession(sessionCode, 0, { suppressErrorToast: true, assistantCode });
  return <div className="absolute inset-0 min-h-0 overflow-y-auto overscroll-y-contain">
    <div className="m-4">
      <RightContextSidebar assistantCode={assistantCode} refreshInterval={visible ? 3_000 : undefined} events={events} messages={messages} session={session ? { ...session, sourceType: session.sourceType ?? undefined } : undefined} />
    </div>
  </div>;
}

import type { ChatSession } from "@tron/chat-core";
import { useChatAguiEvents, useChatMessageListBySession } from "@/domain/chat/hooks";
import { useChatContextRefreshInterval } from "@/domain/chat/hooks/useChatContextRefreshInterval";
import { useTabVisibility } from "@/components/editable-tabs";
import { RightContextSidebar } from "@/domain/chat/pages/ChatManagementPage";

/** Reuse the session's short-term-memory view outside the chat management page (for Dude). */
export function ChatContextInspector({ session }: { session: ChatSession | null }) {
  const sessionCode = session?.sessionCode ?? "";
  const dudeCode = session?.ownerDudeCode ?? undefined;
  const visible = useTabVisibility();
  const refreshKey = useChatContextRefreshInterval(sessionCode, visible, 3_000);
  const events = useChatAguiEvents(sessionCode, refreshKey, dudeCode);
  const messages = useChatMessageListBySession(sessionCode, 0, { suppressErrorToast: true, dudeCode });
  return <div className="absolute inset-0 min-h-0 overflow-y-auto overscroll-y-contain">
    <div className="m-4">
      <RightContextSidebar dudeCode={dudeCode} refreshInterval={visible ? 3_000 : undefined} events={events} messages={messages} session={session ? { ...session, sourceType: session.sourceType ?? undefined } : undefined} />
    </div>
  </div>;
}

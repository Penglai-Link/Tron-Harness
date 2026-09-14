import { Archive, LoaderCircle } from "lucide-react";
import { useState } from "react";
import { Link, useLocation, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { archiveChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";

export function SessionHistoryItem({
  running,
  session,
}: {
  running: boolean;
  session: ChatSession;
}) {
  const location = useLocation();
  const navigate = useNavigate();
  const toast = useToast();
  const { t } = useTranslation("pages");
  const [archiving, setArchiving] = useState(false);
  const active =
    new URLSearchParams(location.search).get("sessionCode") ===
    session.sessionCode;

  async function archive() {
    if (archiving) return;
    setArchiving(true);
    try {
      await archiveChatSession(session.sessionCode);
      window.dispatchEvent(new Event("tron:chat-session-archived"));
      if (active) navigate("/chats");
      toast.success("会话已归档");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "归档会话失败");
    } finally {
      setArchiving(false);
    }
  }

  return (
    <div
      className={cn(
        "group/session flex h-8 min-w-0 items-center rounded hover:bg-sidebar-accent relative pl-5 pr-1",
        active && "bg-sidebar-accent text-sidebar-accent-foreground",
      )}
    >
      <Link
        to={`/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`}
        className="min-w-0 flex-1 truncate text-[14px]"
      >
        {session.title}
      </Link>
      <div className="opacity-0 z-20 absolute flex items-center pl-2 right-0 h-full bg-linear-to-r from-sidebar-accent/0 from-0% to-sidebar-accent to-30% pointer-events-none transition-opacity group-hover/session:opacity-100 group-hover/session:pointer-events-auto group-focus-within/session:opacity-100 group-focus-within/session:pointer-events-auto">
        <Button
          className="shrink-0"
          disabled={archiving}
          type="button"
          variant="link"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            void archive();
          }}
        >
          <Archive size={14} />
        </Button>
      </div>
      {running ? (
        <Button
          type="button"
          variant="link"
          className="absolute z-10 right-0 shrink-0 items-center justify-center group-hover:hide"
        >
          <LoaderCircle size={14} className="animate-spin" />
        </Button>
      ) : null}
    </div>
  );
}

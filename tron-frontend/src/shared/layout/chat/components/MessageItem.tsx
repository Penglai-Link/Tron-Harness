import { Archive, Icon, LoaderCircle } from "lucide-react";
import { useState } from "react";
import { Link, useLocation, useNavigate } from "react-router";
import { useTranslation } from "react-i18next";
import { archiveChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import { cn } from "@/shared/lib/utils";
import { useToast } from "@/components/ui/toast";
import IconButton from "./IconButton";
import Item from "./Item";

const MessageItem = ({
  running,
  session,
}: {
  running: boolean;
  session: ChatSession;
}) => {
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
    <Item
      className={cn(
        "group/session flex h-8 min-w-0 items-center relative pl-6",
        active && "bg-sidebar-accent text-sidebar-accent-foreground",
      )}
      left={
        <Link
          to={`/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`}
          className="min-w-0 flex-1 truncate text-[14px]"
        >
          {session.title}
        </Link>
      }
      right={
        <>
          <div className="flex items-center pl-2 right-0 h-full bg-linear-to-r from-sidebar-accent/0 from-0% to-sidebar-accent to-30%">
            <IconButton
              needHover
              className="shrink-0 hidden group-hover/session:block"
              disabled={archiving}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                void archive();
              }}
              icon={Archive}
            />
            {running ? (
              <IconButton
                needHover
                className="shrink-0 items-center justify-center pointer-events-none"
              >
                <LoaderCircle size={14} className="animate-spin" />
              </IconButton>
            ) : null}
          </div>
        </>
      }
    ></Item>
  );
};

export default MessageItem;

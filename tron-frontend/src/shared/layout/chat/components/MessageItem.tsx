import { Archive, Icon, LoaderCircle } from "lucide-react";
import { memo, useState } from "react";
import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { archiveChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import { cn } from "@/shared/lib/utils";
import { useToast } from "@/components/ui/toast";
import { useTabStore } from "@/store/tabStore";
import IconButton from "./IconButton";
import Item from "./Item";

function sessionAgentTagLabel(agentCode: string | null | undefined) {
  const normalized = agentCode?.trim().toLowerCase();
  if (!normalized || normalized === "main-agent") return "MAIN";
  return normalized.toUpperCase();
}

function sessionAgentTagTone(agentCode: string | null | undefined) {
  const normalized = agentCode?.trim().toLowerCase();
  if (!normalized || normalized === "main-agent") {
    return "border-emerald-200 bg-emerald-50 text-emerald-700";
  }
  if (normalized === "opencode") {
    return "border-amber-200 bg-amber-50 text-amber-700";
  }
  if (normalized === "codex") {
    return "border-sky-200 bg-sky-50 text-sky-700";
  }
  return "border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] text-[var(--color-on-surface-variant)]";
}

const MessageItem = ({
  active,
  onNavigate,
  running,
  session,
}: {
  active: boolean;
  onNavigate: (path: string) => void;
  running: boolean;
  session: ChatSession;
}) => {
  const toast = useToast();
  const { t } = useTranslation("shell");
  const [archiving, setArchiving] = useState(false);
  async function archive() {
    if (archiving) return;
    setArchiving(true);
    try {
      await archiveChatSession(session.sessionCode);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.session.archiveFailed"));
      setArchiving(false);
      return;
    }

    window.dispatchEvent(
      new CustomEvent("tron:chat-session-archived", {
        detail: { sessionCode: session.sessionCode },
      }),
    );
    const tabStore = useTabStore.getState();
    const archivedTab = tabStore.tabs.find(
      (tab) => tab.activeNav === `chat-session:${session.sessionCode}`,
    );
    if (archivedTab) {
      const wasActive = tabStore.activeTabId === archivedTab.id;
      const nextTab = tabStore.tabs.find(
        (tab) => !tab.hide && tab.id !== archivedTab.id,
      );
      tabStore.removeTab(archivedTab.id);
      if (wasActive && nextTab) {
        useTabStore.getState().setActiveTab(nextTab.id);
        onNavigate(nextTab.path);
      }
    }
    toast.success(t("shell:chatSidebar.session.archived"));
    setArchiving(false);
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
          className="flex min-w-0 flex-1 items-center gap-1.5 truncate text-sm"
          onClick={(event) => {
            if (
              event.defaultPrevented ||
              event.button !== 0 ||
              event.metaKey ||
              event.ctrlKey ||
              event.shiftKey ||
              event.altKey
            ) {
              return;
            }

            const path = `/chats?sessionCode=${encodeURIComponent(session.sessionCode)}`;
            useTabStore.getState().addTab({
              title: session.title,
              path,
              closable: true,
              activeNav: `chat-session:${session.sessionCode}`,
            });
          }}
        >
          <span
            className={cn(
              "inline-flex h-4 shrink-0 items-center rounded border px-1 font-mono text-[0.625rem] leading-none",
              sessionAgentTagTone(session.agentCode),
            )}
          >
            {sessionAgentTagLabel(session.agentCode)}
          </span>
          <span className="min-w-0 truncate">{session.title}</span>
        </Link>
      }
      right={
        <>
          <div
            className={cn(
              "opacity-0 group-hover/session:opacity-100 flex items-center pl-2 right-0 h-full bg-linear-to-r from-sidebar-accent/0 from-0% to-sidebar-accent to-30%",
              running ? "opacity-100" : "",
            )}
          >
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

export default memo(MessageItem);

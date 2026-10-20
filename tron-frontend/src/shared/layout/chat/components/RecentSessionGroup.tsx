import { ChevronDown } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import MessageItem from "./MessageItem";
import ActionButton from "./ActionButton";
import Item from "./Item";
import type { ChatSession } from "@/domain/chat/model";
import { Button } from "@/components/ui/button";
import { sortPinnedSessions } from "../hooks/usePinnedSessionCodes";

const recentSessionListPageSize = 5;

export default function RecentSessionGroup({
  label,
  runningSessionCode,
  activeSessionCode,
  onNavigate,
  sessions,
  pinnedSessionCodes,
  onToggleSessionPin,
  onRemoveSessionPin,
}: {
  label?: string;
  runningSessionCode: string;
  activeSessionCode: string;
  onNavigate: (path: string) => void;
  sessions: ChatSession[];
  pinnedSessionCodes: string[];
  onToggleSessionPin: (sessionCode: string) => void;
  onRemoveSessionPin: (sessionCode: string) => void;
}) {
  const { t } = useTranslation("shell");
  const [open, setOpen] = useState(true);
  const [visibleSessionCount, setVisibleSessionCount] = useState(
    recentSessionListPageSize
  );

  return (
    <section>
      <Item
        className="group/recent px-2"
        left={
          <button
            aria-expanded={open}
            className="flex min-w-0 flex-1 items-center gap-1 text-left opacity-50"
            type="button"
            onClick={() => setOpen((value) => !value)}
          >
            <span>{label ?? t("shell:chatSidebar.recent")}</span>
          </button>
        }
        right={
          <ActionButton
            className={
              open ? "transition-transform" : "-rotate-90 transition-transform"
            }
            aria-label={t(open ? "common:actions.collapse" : "common:actions.expand", open ? "Collapse" : "Expand")}
            onClick={(event) => {
              event.stopPropagation();
              setOpen((value) => !value);
            }}
          ><ChevronDown size={16} aria-hidden="true" /></ActionButton>
        }
      />
      {open ? (
        <div className="flex flex-col gap-1">
          {sortPinnedSessions(sessions, pinnedSessionCodes).slice(0, visibleSessionCount).map((session) => (
            <MessageItem
              key={session.sessionCode}
              running={runningSessionCode === session.sessionCode}
              active={activeSessionCode === session.sessionCode}
              onNavigate={onNavigate}
              session={session}
              pinned={pinnedSessionCodes.includes(session.sessionCode)}
              onTogglePin={() => onToggleSessionPin(session.sessionCode)}
              onRemovePin={() => onRemoveSessionPin(session.sessionCode)}
            />
          ))}
          {sessions.length > visibleSessionCount ? (
            <Button
              className="ml-6 w-fit"
              size="sm"
              type="button"
              variant="ghost"
              onClick={() =>
                setVisibleSessionCount(
                  (count) => count + recentSessionListPageSize
                )
              }
            >
              {t("shell:chatSidebar.showMore")}
            </Button>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

import { useState } from "react";
import { useTranslation } from "react-i18next";
import MessageItem from "./MessageItem";
import { SectionHeader } from "./SectionHeader";
import { ShowMoreButton } from "./ShowMoreButton";
import type { ChatSession } from "@/domain/chat/model";
import { sortPinnedSessions } from "../hooks/usePinnedSessionCodes";

const recentSessionListPageSize = 5;

export default function RecentSessionGroup({
  label,
  onCreate,
  canCreate,
  activeSessionCode,
  onNavigate,
  sessions,
  pinnedSessionCodes,
  onToggleSessionPin,
  onRemoveSessionPin,
}: {
  label?: string;
  onCreate: () => void;
  canCreate: boolean;
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
      <SectionHeader
        label={label ?? t("chatSidebar.recent")}
        open={open}
        onToggle={() => setOpen((value) => !value)}
        onCreate={onCreate}
        createDisabled={!canCreate}
        createLabel={t("pages:settingsShortcuts.shortcuts.newSession.label")}
        shortcutAction="newSession"
      />
      {open ? (
        <div className="flex flex-col gap-1">
          {sortPinnedSessions(sessions, pinnedSessionCodes).slice(0, visibleSessionCount).map((session) => (
            <MessageItem
              key={session.sessionCode}
              active={activeSessionCode === session.sessionCode}
              onNavigate={onNavigate}
              session={session}
              pinned={pinnedSessionCodes.includes(session.sessionCode)}
              onTogglePin={() => onToggleSessionPin(session.sessionCode)}
              onRemovePin={() => onRemoveSessionPin(session.sessionCode)}
            />
          ))}
          {sessions.length > visibleSessionCount ? (
            <ShowMoreButton
              className="ml-2"
              onClick={() =>
                setVisibleSessionCount(
                  (count) => count + recentSessionListPageSize
                )
              }
            />
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

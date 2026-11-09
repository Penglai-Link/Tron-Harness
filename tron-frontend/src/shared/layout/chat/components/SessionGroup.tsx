import { ChevronRight, Folder, FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/shared/lib/utils";
import { useState } from "react";
import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";
import { sortPinnedSessions } from "../hooks/usePinnedSessionCodes";

import MessageItem from "./MessageItem";
import ActionsMenu from "./ActionsMenu";
import CreateByProject from "./CreateByProject";
import ActionButton from "./ActionButton";
import Item from "./Item";

const sessionListPageSize = 5;

const SessionGroup = ({
  project,
  runningSessionCode,
  activeSessionCode,
  onNavigate,
  sessions,
  selected,
  pinned,
  pinnedSessionCodes,
  onEdit,
  onPinnedChange,
  onRefresh,
  onToggleSessionPin,
  onRemoveSessionPin,
  onSelect,
}: {
  project: ProjectRecord;
  runningSessionCode: string;
  activeSessionCode: string;
  onNavigate: (path: string) => void;
  sessions: ChatSession[];
  selected: boolean;
  pinned: boolean;
  pinnedSessionCodes: string[];
  onEdit: () => void;
  onPinnedChange: () => void;
  onRefresh: () => void;
  onToggleSessionPin: (sessionCode: string) => void;
  onRemoveSessionPin: (sessionCode: string) => void;
  onSelect: (projectCode: string) => void;
}) => {
  const { t } = useTranslation("shell");
  const [open, setOpen] = useState(true);
  const [visibleSessionCount, setVisibleSessionCount] =
    useState(sessionListPageSize);
  const TopicIcon = open ? FolderOpen : Folder;
  return (
    <section>
      <Item
        className={cn(
          "group/project flex pl-2 w-full min-w-0 items-center gap-1",
        )}
        left={
          <div className="flex min-w-0 flex-1 items-center gap-1">
            <ActionButton aria-label={t(open ? "common:actions.collapse" : "common:actions.expand")} onClick={() => setOpen((value) => !value)}>
              <ChevronRight size={14} className={open ? "rotate-90" : undefined} aria-hidden="true" />
            </ActionButton>
            <ActionButton className="flex min-w-0 flex-1 items-center gap-1.5 text-left justify-start truncate" onClick={() => {
              onSelect(project.projectCode);
              onNavigate(`/topics/${encodeURIComponent(project.projectCode)}`);
            }}>
            <TopicIcon size={16} className="shrink-0" aria-hidden="true" />
            <span className="truncate" title={project.name}>{project.name}</span>
            </ActionButton>
          </div>
        }
        right={
          <div className="flex shrink-0 gap-2 items-center opacity-0 pointer-events-none group-hover/project:opacity-100 group-hover/project:pointer-events-auto has-[button[data-popup-open]]:opacity-100 has-[button[data-popup-open]]:pointer-events-auto">
            <CreateByProject project={project} sessions={sessions} />
            <ActionsMenu
              pinned={pinned}
              project={project}
              onEdit={onEdit}
              onPinnedChange={onPinnedChange}
              onRefresh={onRefresh}
            />
          </div>
        }
      />
      {open ? (
        <div className="flex flex-col gap-0.5 pl-1.5">
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
          {sessions.length === 0 ? (
            <p className="py-1 pl-6 text-xs opacity-45">{t("shell:chatSidebar.noSessions")}</p>
          ) : null}
          {sessions.length > visibleSessionCount ? (
            <button
              className="py-1 pl-6 text-sm! opacity-45 text-left"
              onClick={() =>
                setVisibleSessionCount((count) => count + sessionListPageSize)
              }
            >
              {t("shell:chatSidebar.showMore")}
            </button>
          ) : null}
        </div>
      ) : null}
    </section>
  );
};

export default SessionGroup;

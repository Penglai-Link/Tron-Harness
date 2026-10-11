import { useTranslation } from "react-i18next";
import { cn } from "@/shared/lib/utils";
import { useState } from "react";
import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";

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
  onEdit,
  onPinnedChange,
  onRefresh,
  onSelect,
}: {
  project: ProjectRecord;
  runningSessionCode: string;
  activeSessionCode: string;
  onNavigate: (path: string) => void;
  sessions: ChatSession[];
  selected: boolean;
  pinned: boolean;
  onEdit: () => void;
  onPinnedChange: () => void;
  onRefresh: () => void;
  onSelect: (projectCode: string) => void;
}) => {
  const { t } = useTranslation("shell");
  const [open, setOpen] = useState(true);
  const [visibleSessionCount, setVisibleSessionCount] =
    useState(sessionListPageSize);
  return (
    <section>
      <Item
        className={cn(
          "group/project flex mb-1 pl-2 w-full min-w-0 items-center gap-1",
        )}
        left={
          <ActionButton
            className="flex min-w-0 flex-1 items-center text-left justify-start truncate"
            onClick={() => {
              onSelect(project.projectCode);
              setOpen((value) => !value);
            }}
          >

            <span className="truncate">{project.name}</span>
          </ActionButton>
        }
        right={
          <div className="flex shrink-0 gap-2 items-center opacity-0 pointer-events-none group-hover/project:opacity-100 group-hover/project:pointer-events-auto has-[button[data-popup-open]]:opacity-100 has-[button[data-popup-open]]:pointer-events-auto">
            <CreateByProject project={project} sessions={sessions} />
            <ActionsMenu
              pinned={pinned}
              project={project}
              sessions={sessions}
              onEdit={onEdit}
              onPinnedChange={onPinnedChange}
              onRefresh={onRefresh}
            />
          </div>
        }
      />
      {open ? (
        <div className="flex flex-col gap-1">
          {sessions.slice(0, visibleSessionCount).map((session) => (
            <MessageItem
              key={session.sessionCode}
              running={runningSessionCode === session.sessionCode}
              active={activeSessionCode === session.sessionCode}
              onNavigate={onNavigate}
              session={session}
            />
          ))}
          {sessions.length === 0 ? (
            <p className="py-1 pl-6 text-sm opacity-45">{t("shell:chatSidebar.noSessions")}</p>
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

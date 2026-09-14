import { ChevronDown, ChevronRight, Folder, FolderOpen } from "lucide-react";
import { cn } from "@/shared/lib/utils";
import { useState } from "react";
import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";
import { SessionHistoryItem } from "./SessionHistoryItem";
import { ProjectActionsMenu } from "./ProjectActionsMenu";
import { ProjectSessionCreateButton } from "./ProjectSessionCreateButton";
import IconButton from "./IconButton";

export function ProjectSessionGroup({
  project,
  runningSessionCode,
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
  sessions: ChatSession[];
  selected: boolean;
  pinned: boolean;
  onEdit: () => void;
  onPinnedChange: () => void;
  onRefresh: () => void;
  onSelect: (projectCode: string) => void;
}) {
  const [open, setOpen] = useState(true);
  return (
    <section>
      <div
        className={cn(
          "group/project flex h-8 mb-1 pl-2 w-full min-w-0 items-center gap-1 rounded hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
        )}
      >
        <IconButton
          className="flex min-w-0 flex-1 items-center gap-1 text-left justify-start truncate"
          onClick={() => {
            onSelect(project.projectCode);
            setOpen((value) => !value);
          }}
          icon={open ? FolderOpen : Folder}
        >
          {project.name}
        </IconButton>
        <div className="flex shrink-0 items-center opacity-0 pointer-events-none transition-opacity group-hover/project:opacity-100 group-hover/project:pointer-events-auto group-focus-within/project:opacity-100 group-focus-within/project:pointer-events-auto has-[button[data-popup-open]]:opacity-100 has-[button[data-popup-open]]:pointer-events-auto">
          <ProjectActionsMenu
            pinned={pinned}
            project={project}
            sessions={sessions}
            onEdit={onEdit}
            onPinnedChange={onPinnedChange}
            onRefresh={onRefresh}
          />
          <ProjectSessionCreateButton project={project} sessions={sessions} />
        </div>
      </div>
      {open ? (
        <div className="flex flex-col gap-1">
          {sessions.map((session) => (
            <SessionHistoryItem
              key={session.sessionCode}
              running={runningSessionCode === session.sessionCode}
              session={session}
            />
          ))}
          {sessions.length === 0 ? (
            <p className="py-1 pl-5 text-[14px] opacity-45">暂无会话</p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

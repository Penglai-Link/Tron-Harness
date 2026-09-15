import { ChevronDown } from "lucide-react";
import { useState } from "react";
import MessageItem from "./MessageItem";
import IconButton from "./IconButton";
import Item from "./Item";
import type { ChatSession } from "@/domain/chat/model";

export default function RecentSessionGroup({
  runningSessionCode,
  sessions,
}: {
  runningSessionCode: string;
  sessions: ChatSession[];
}) {
  const [open, setOpen] = useState(true);

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
            <span>最近</span>
          </button>
        }
        right={
          <IconButton
            aria-label={open ? "收起最近会话" : "展开最近会话"}
            className={open ? "transition-transform" : "-rotate-90 transition-transform"}
            icon={ChevronDown}
            onClick={(event) => {
              event.stopPropagation();
              setOpen((value) => !value);
            }}
          />
        }
      />
      {open ? (
        <div className="flex flex-col gap-1">
          {sessions.map((session) => (
            <MessageItem
              key={session.sessionCode}
              running={runningSessionCode === session.sessionCode}
              session={session}
            />
          ))}
        </div>
      ) : null}
    </section>
  );
}

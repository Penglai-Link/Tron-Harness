import { useState } from "react";
import { MoreHorizontal } from "lucide-react";
import { archiveChatSession } from "@/domain/chat/hooks";
import type { ChatSession } from "@/domain/chat/model";
import { removeProject, type ProjectRecord } from "@/domain/project";
import { ConfirmDialog } from "@/components/confirm-dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useToast } from "@/components/ui/toast";
import IconButton from "./IconButton";

const ActionsMenu = ({
  onEdit,
  onPinnedChange,
  onRefresh,
  pinned,
  project,
  sessions,
}: {
  onEdit: () => void;
  onPinnedChange: () => void;
  onRefresh: () => void;
  pinned: boolean;
  project: ProjectRecord;
  sessions: ChatSession[];
}) => {
  const toast = useToast();
  const [archiveChatsOpen, setArchiveChatsOpen] = useState(false);
  const [removalOpen, setRemovalOpen] = useState(false);
  const [pending, setPending] = useState(false);

  async function archiveChats() {
    setPending(true);
    try {
      await Promise.all(
        sessions.map((session) => archiveChatSession(session.sessionCode)),
      );
      onRefresh();
      setArchiveChatsOpen(false);
      toast.success("项目会话已归档");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "归档项目会话失败");
    } finally {
      setPending(false);
    }
  }

  async function remove() {
    setPending(true);
    try {
      await removeProject(project.projectCode);
      onRefresh();
      setRemovalOpen(false);
      toast.success("项目已移除");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "移除项目失败");
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <IconButton
              onClick={(event) => event.stopPropagation()}
              icon={MoreHorizontal}
            />
          }
        />
        <DropdownMenuContent
          align="start"
          onClick={(event) => event.stopPropagation()}
        >
          <DropdownMenuItem onClick={onPinnedChange}>
            {pinned ? "取消置顶" : "置顶"}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onEdit}>编辑</DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            disabled={sessions.length === 0}
            onClick={() => setArchiveChatsOpen(true)}
          >
            归档聊天
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            variant="destructive"
            onClick={() => setRemovalOpen(true)}
          >
            移除项目
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <ConfirmDialog
        cancelText="取消"
        confirmText="归档聊天"
        description={`将归档“${project.name}”下的 ${sessions.length} 条会话。`}
        open={archiveChatsOpen}
        pending={pending}
        title="归档项目聊天？"
        tone="default"
        onCancel={() => setArchiveChatsOpen(false)}
        onConfirm={() => void archiveChats()}
      />
      <ConfirmDialog
        cancelText="取消"
        confirmText="移除项目"
        description={`将移除“${project.name}”及其项目关联。会话记录会保留。`}
        open={removalOpen}
        pending={pending}
        title="移除项目？"
        onCancel={() => setRemovalOpen(false)}
        onConfirm={() => void remove()}
      />
    </>
  );
};

export default ActionsMenu;

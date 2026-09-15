import { useState } from "react";
import { MoreHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation("shell");
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
      toast.success(t("shell:chatSidebar.project.archived"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.project.archiveFailed"));
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
      toast.success(t("shell:chatSidebar.project.removed"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t("shell:chatSidebar.project.removeFailed"));
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
              needHover
              onClick={(event) => event.stopPropagation()}
              icon={MoreHorizontal}
            />
          }
        />
        <DropdownMenuContent
          className="duration-10"
          align="start"
          onClick={(event) => event.stopPropagation()}
        >
          <DropdownMenuItem onClick={onPinnedChange}>
            {pinned
              ? t("shell:chatSidebar.project.unpin")
              : t("shell:chatSidebar.project.pin")}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={onEdit}>{t("shell:chatSidebar.project.edit")}</DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            disabled={sessions.length === 0}
            onClick={() => setArchiveChatsOpen(true)}
          >
            {t("shell:chatSidebar.project.archiveChats")}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            variant="destructive"
            onClick={() => setRemovalOpen(true)}
          >
            {t("shell:chatSidebar.project.removeProject")}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <ConfirmDialog
        cancelText={t("shell:chatSidebar.project.cancel")}
        confirmText={t("shell:chatSidebar.project.archiveChats")}
        description={t("shell:chatSidebar.confirmations.archiveDescription", {
          count: sessions.length,
          name: project.name,
        })}
        open={archiveChatsOpen}
        pending={pending}
        title={t("shell:chatSidebar.confirmations.archiveTitle")}
        tone="default"
        onCancel={() => setArchiveChatsOpen(false)}
        onConfirm={() => void archiveChats()}
      />
      <ConfirmDialog
        cancelText={t("shell:chatSidebar.project.cancel")}
        confirmText={t("shell:chatSidebar.project.removeProject")}
        description={t("shell:chatSidebar.confirmations.removeDescription", {
          name: project.name,
        })}
        open={removalOpen}
        pending={pending}
        title={t("shell:chatSidebar.confirmations.removeTitle")}
        onCancel={() => setRemovalOpen(false)}
        onConfirm={() => void remove()}
      />
    </>
  );
};

export default ActionsMenu;

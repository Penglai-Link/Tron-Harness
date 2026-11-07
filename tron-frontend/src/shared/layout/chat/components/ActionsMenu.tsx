import { MoreHorizontal } from "lucide-react";
import { useState } from "react";

import { useTranslation } from "react-i18next";
import { notifyArchivedChatSessions } from "@/domain/chat/hooks/useChatSessionGroupList";
import { archiveProject, deleteProject, type ProjectRecord } from "@/domain/project";
import { ConfirmDialog } from "@/components/confirm-dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useToast } from "@/components/ui/toast";
import ActionButton from "./ActionButton";

const ActionsMenu = ({
  onEdit,
  onPinnedChange,
  onRefresh,
  pinned,
  project,
}: {
  onEdit: () => void;
  onPinnedChange: () => void;
  onRefresh: () => void;
  pinned: boolean;
  project: ProjectRecord;
}) => {
  const toast = useToast();
  const { t } = useTranslation("shell");
  const [archiveChatsOpen, setArchiveChatsOpen] = useState(false);
  const [removalOpen, setRemovalOpen] = useState(false);
  const [pending, setPending] = useState(false);

  async function mutate(remove: boolean) {
    if (pending) return;
    setPending(true);
    try {
      const result = await (remove ? deleteProject(project.projectCode) : archiveProject(project.projectCode));
      notifyArchivedChatSessions(result.sessionCodes, { projectCodes: [project.projectCode] });
      window.dispatchEvent(new Event("tron:project-created"));
      onRefresh();
      setArchiveChatsOpen(false);
      setRemovalOpen(false);
      toast.success(t(remove ? "chatSidebar.project.removed" : "chatSidebar.project.archived"));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t(remove ? "chatSidebar.project.removeFailed" : "chatSidebar.project.archiveFailed"));
    } finally { setPending(false); }
  }

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <ActionButton
              needHover
              onClick={(event) => event.stopPropagation()}
              aria-label={t("common:actions.more", "More")}
            ><MoreHorizontal size={16} aria-hidden="true" /></ActionButton>
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
            disabled={pending}
            onClick={() => setArchiveChatsOpen(true)}
          >
            {t("shell:chatSidebar.project.archiveChats")}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem
            variant="destructive"
            disabled={pending}
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
          name: project.name,
        })}
        open={archiveChatsOpen}
        pending={pending}
        title={t("shell:chatSidebar.confirmations.archiveTitle")}
        tone="default"
        onCancel={() => setArchiveChatsOpen(false)}
        onConfirm={() => void mutate(false)}
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
        onConfirm={() => void mutate(true)}
      />
    </>
  );
};

export default ActionsMenu;

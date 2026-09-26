import { SquarePen } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ProjectRecord } from "@/domain/project";
import { useCreateChatTab } from "../hooks/useCreateChatTab";
import IconButton from "./IconButton";

export function Create({
  project,
  workspaceCode,
}: {
  project?: ProjectRecord;
  workspaceCode: string;
}) {
  const { t } = useTranslation("shell");
  const { create, creating } = useCreateChatTab(workspaceCode, project?.projectCode);
  return (
    <IconButton
      disabled={!workspaceCode || creating}
      onClick={() => void create()}
      icon={SquarePen}
      className="w-full justify-start"
    >
      {t("shell:chatSidebar.newChat")}
    </IconButton>
  );
}

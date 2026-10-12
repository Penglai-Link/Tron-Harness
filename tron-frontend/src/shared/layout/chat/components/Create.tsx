import { SquarePen } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ProjectRecord } from "@/domain/project";
import { useCreateChatTab } from "../hooks/useCreateChatTab";
import ActionButton from "./ActionButton";

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
    <ActionButton
      disabled={!workspaceCode || creating}
      onClick={() => void create()}
      className="w-full justify-start gap-3 pl-1"
    >
      <SquarePen size={17} className="shrink-0" aria-hidden="true" />
      {t("shell:chatSidebar.newChat")}
    </ActionButton>
  );
}

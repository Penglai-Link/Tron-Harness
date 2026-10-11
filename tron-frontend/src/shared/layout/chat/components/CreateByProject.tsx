import { useTranslation } from "react-i18next";

import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";
import { useCreateChatTab } from "../hooks/useCreateChatTab";
import ActionButton from "./ActionButton";

const CreateByProject = ({ project }: { project: ProjectRecord; sessions: ChatSession[] }) => {
  const { t } = useTranslation("shell");
  const { create, creating } = useCreateChatTab(project.workspaceCode, project.projectCode);
  return (
    <ActionButton
      className="shrink-0 p-0"
      disabled={creating}
      aria-label={t("shell:chatSidebar.newChat")}
      needHover
      onClick={(event) => {
        event.stopPropagation();
        create();
      }}
    />
  );
};

export default CreateByProject;

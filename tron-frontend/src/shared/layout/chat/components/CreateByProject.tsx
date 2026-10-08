import { Plus } from "lucide-react";
import type { ChatSession } from "@/domain/chat/model";
import type { ProjectRecord } from "@/domain/project";
import { useCreateChatTab } from "../hooks/useCreateChatTab";
import IconButton from "./IconButton";

const CreateByProject = ({ project }: { project: ProjectRecord; sessions: ChatSession[] }) => {
  const { create, creating } = useCreateChatTab(project.workspaceCode, project.projectCode);
  return (
    <IconButton
      className="shrink-0 p-0"
      disabled={creating}
      icon={Plus}
      needHover
      onClick={(event) => {
        event.stopPropagation();
        create();
      }}
    />
  );
};

export default CreateByProject;

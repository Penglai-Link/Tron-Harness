import { useSystemDirectoryPicker } from "@/domain/directory/hooks/useSystemDirectoryPicker";
import { useCreateChatTab } from "./useCreateChatTab";

export function useDirectoryPicker(workspaceCode: string) {
  const { createForDirectory } = useCreateChatTab(workspaceCode);
  return useSystemDirectoryPicker(workspaceCode, (directory) => {
    createForDirectory(directory.directoryCode);
  });
}

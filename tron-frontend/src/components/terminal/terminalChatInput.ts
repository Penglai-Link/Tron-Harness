import type { SendChatMessageInput } from "@tron/chat-core";

export function createTerminalChatInput(
  content: string,
  workspaceCode: string,
): SendChatMessageInput {
  return {
    content,
    modelSelection: { kind: "Auto" },
    workspaceCode,
  };
}

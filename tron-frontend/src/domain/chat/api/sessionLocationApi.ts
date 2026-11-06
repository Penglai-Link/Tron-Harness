import { requestApi } from "@/shared/api/httpClient";
import type { ChatSession } from "../model";

export function moveChatSession(sessionCode: string, workspaceCode: string, projectCode: string | null) {
  return requestApi<ChatSession>("PUT /api/v1/chats/sessions/{sessionCode}/placement", {
    pathParams: { sessionCode }, queryParams: { workspaceCode },
    body: { placement: projectCode ? "Topic" : "Quick", projectCode },
  });
}

export function setChatWorkingDirectory(sessionCode: string, workspaceCode: string, directoryCode: string | null) {
  return requestApi<ChatSession>("PUT /api/v1/chats/sessions/{sessionCode}/working-directory", {
    pathParams: { sessionCode }, queryParams: { workspaceCode }, body: { directoryCode },
  });
}

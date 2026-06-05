export const chatApiEndpoints = {
  listChatSessionGroups: "GET /api/v1/chats/session-groups?workspaceCode={workspaceCode}",
  archiveChatSession: "POST /api/v1/chats/sessions/{sessionCode}/archive",
  createChatSession: "POST /api/v1/chats/sessions",
  deleteChatSession: "DELETE /api/v1/chats/sessions/{sessionCode}",
  updateChatSession: "PATCH /api/v1/chats/sessions/{sessionCode}",
  listChatMessages: "GET /api/v1/chats/sessions/{sessionCode}/messages",
  sendChatMessage: "POST /api/v1/chats/sessions/{sessionCode}/messages",
  streamChatMessage: "POST /api/v1/chats/sessions/{sessionCode}/messages/stream",
  listAguiEvents: "GET /api/v1/agents/agui/streams/{streamKey}/events",
  listAguiProtocolEvents: "GET /api/v1/agents/agui/streams/{streamKey}/protocol-events",
} as const;

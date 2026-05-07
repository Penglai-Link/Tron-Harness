export const chatApiEndpoints = {
  listChatSessionGroups: "GET /api/v1/chats/session-groups",
  listChatMessages: "GET /api/v1/chats/{chatId}/messages?variant={variant}",
} as const;

// Shell 级跨组件事件：请求在当前激活标签右侧边栏的内置浏览器（PageView）中打开链接。
// detail 为要打开的 URL 字符串。
export const OPEN_CHATS_PAGE_VIEW_EVENT = "tron:open-chats-page-view";

// Composer metrics request the short-term-memory panel for their own session.
export const OPEN_CHAT_CONTEXT_EVENT = "tron:open-chat-context";

export const CHAT_SESSIONS_REMOVED_EVENT = "tron:chat-sessions-removed";

export function notifyChatSessionsRemoved(sessionCodes: string[]) {
  if (!sessionCodes.length) return;
  window.dispatchEvent(new CustomEvent(CHAT_SESSIONS_REMOVED_EVENT, {
    detail: { sessionCodes },
  }));
}

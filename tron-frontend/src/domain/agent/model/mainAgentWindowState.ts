import { defaultFloatingState, type ChatFloatingState } from "@tron/chat-core";

export const MAIN_AGENT_WINDOW_STORAGE_KEY = "tron.main-agent.window";

// Offsets are measured from the floating window's bottom-right viewport anchor.
export const defaultMainAgentWindowState: ChatFloatingState = {
  ...defaultFloatingState,
  width: 380,
  height: 760,
  x: 0,
  y: 0,
};

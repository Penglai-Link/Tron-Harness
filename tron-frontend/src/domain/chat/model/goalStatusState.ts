import type { ChatSessionGoalStatus } from "./ChatModel";

export function isEditableGoalStatus(status: ChatSessionGoalStatus) {
  return status === "ACTIVE" || status === "PAUSED" || status === "BLOCKED";
}

export function canStartNewGoalFromStatus(status: ChatSessionGoalStatus) {
  return status === "COMPLETED" || status === "CANCELLED" || status === "CLEARED";
}

export function canClearGoalStatus(status: ChatSessionGoalStatus) {
  return status !== "CLEARED";
}

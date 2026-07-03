import { describe, expect, it } from "vitest";

import {
  canClearGoalStatus,
  canStartNewGoalFromStatus,
  isEditableGoalStatus,
} from "./goalStatusState";
import type { ChatSessionGoalStatus } from "./ChatModel";

const statuses: ChatSessionGoalStatus[] = [
  "ACTIVE",
  "PAUSED",
  "BLOCKED",
  "COMPLETED",
  "CANCELLED",
  "CLEARED",
];

describe("goal status state", () => {
  it("allows editing only unfinished goals", () => {
    expect(statuses.filter(isEditableGoalStatus)).toEqual(["ACTIVE", "PAUSED", "BLOCKED"]);
  });

  it("allows starting a new goal only after terminal statuses", () => {
    expect(statuses.filter(canStartNewGoalFromStatus)).toEqual(["COMPLETED", "CANCELLED", "CLEARED"]);
  });

  it("does not show clear again after a goal has already been cleared", () => {
    expect(statuses.filter(canClearGoalStatus)).toEqual([
      "ACTIVE",
      "PAUSED",
      "BLOCKED",
      "COMPLETED",
      "CANCELLED",
    ]);
  });
});

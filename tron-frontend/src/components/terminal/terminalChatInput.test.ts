import { describe, expect, it } from "vitest";

import { createTerminalChatInput } from "./terminalChatInput";

describe("createTerminalChatInput", () => {
  it("uses Person-AI Auto routing for natural-language terminal requests", () => {
    expect(createTerminalChatInput("show current changes", "WORKSPACE-1")).toEqual({
      content: "show current changes",
      modelSelection: { kind: "Auto" },
      workspaceCode: "WORKSPACE-1",
    });
  });
});

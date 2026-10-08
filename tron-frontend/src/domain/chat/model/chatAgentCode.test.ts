import { describe, expect, it } from "vitest";
import { inferChatAgentCodeFromModel } from "./chatAgentCode";

describe("inferChatAgentCodeFromModel", () => {
  it("keeps the selected custom internal agent when choosing a platform model", () => {
    expect(inferChatAgentCodeFromModel({ optionCode: "main:qwen-3", source: "internal-agent" }, "agent-reviewer")).toBe("agent-reviewer");
  });
  it("returns main-agent for main-agent models", () => {
    expect(inferChatAgentCodeFromModel({ optionCode: "main:qwen-3", source: "main-agent" }, "codex")).toBe("main-agent");
  });

  it("returns the system agent code from optionCode for system-agent models", () => {
    expect(inferChatAgentCodeFromModel({ optionCode: "system:codex:model-1", source: "system-agent" }, "main-agent")).toBe("codex");
  });

  it("falls back when the model does not carry a known source", () => {
    expect(inferChatAgentCodeFromModel({ optionCode: "custom:model", source: "unknown" }, "codex")).toBe("codex");
  });
});

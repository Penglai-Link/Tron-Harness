import { describe, expect, it } from "vitest";

import { parseGoalCommand } from "./goalCommand";

describe("parseGoalCommand", () => {
  it("parses inline goal commands", () => {
    expect(parseGoalCommand("/goal do x")).toEqual({
      isGoalCommand: true,
      content: "do x",
    });
  });

  it("parses multiline goal commands and trims the payload", () => {
    expect(parseGoalCommand("/goal\n do x ")).toEqual({
      isGoalCommand: true,
      content: "do x",
    });
  });

  it("does not parse embedded goal text as a command", () => {
    expect(parseGoalCommand("hello /goal do x")).toEqual({
      isGoalCommand: false,
      content: "hello /goal do x",
    });
  });

  it("treats an empty goal command as a command with empty content", () => {
    expect(parseGoalCommand("/goal   ")).toEqual({
      isGoalCommand: true,
      content: "",
    });
  });
});

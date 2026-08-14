import { describe, expect, it } from "vitest";

import { isShellCommand } from "./common";

describe("isShellCommand", () => {
  it.each([
    "npm",
    "npm init",
    "git status",
    "node --version",
    "my-local-cli run",
    "echo 你好",
    "FOO=bar npm test",
    "printf ok | sed s/o/O/",
  ])("routes command-shaped input to the local PTY: %s", (command) => {
    expect(isShellCommand(command)).toBe(true);
  });

  it.each([
    "帮我查看当前目录",
    "如何初始化 npm 项目",
    "please explain this output",
    "how do I create a directory",
  ])("keeps natural language in AI chat: %s", (input) => {
    expect(isShellCommand(input)).toBe(false);
  });
});

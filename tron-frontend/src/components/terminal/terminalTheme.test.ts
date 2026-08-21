import { describe, expect, it } from "vitest";

import { getTerminalAnsiTheme } from "./terminalTheme";

describe("getTerminalAnsiTheme", () => {
  it("uses a dark ANSI white in the light application theme", () => {
    const theme = getTerminalAnsiTheme("light");

    expect(theme.white).toBe("#4b4b4b");
    expect(theme.brightWhite).toBe("#242424");
  });

  it("keeps ANSI white bright in the matrix-dark application theme", () => {
    const theme = getTerminalAnsiTheme("matrix-dark");

    expect(theme.white).toBe("#d6f5dc");
    expect(theme.brightWhite).toBe("#ffffff");
  });
});

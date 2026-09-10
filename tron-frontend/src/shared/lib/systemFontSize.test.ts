import { describe, expect, it } from "vitest";

import {
  applySystemFontSize,
  defaultSystemFontSize,
  isSystemFontSize,
  maximumSystemFontSize,
  minimumSystemFontSize,
  parseSystemFontSize,
  readStoredSystemFontSize,
  systemFontSizeStorageKey,
  systemFontSizeCssVariable,
} from "./systemFontSize";

describe("system font size preferences", () => {
  it("accepts whole pixel values in the supported range", () => {
    expect(isSystemFontSize(minimumSystemFontSize)).toBe(true);
    expect(isSystemFontSize(defaultSystemFontSize)).toBe(true);
    expect(isSystemFontSize(maximumSystemFontSize)).toBe(true);
  });

  it("rejects values outside the supported range or with fractions", () => {
    expect(isSystemFontSize(minimumSystemFontSize - 1)).toBe(false);
    expect(isSystemFontSize(maximumSystemFontSize + 1)).toBe(false);
    expect(isSystemFontSize(15.5)).toBe(false);
    expect(isSystemFontSize("16")).toBe(false);
  });

  it("reads a valid stored value and ignores an invalid one", () => {
    expect(readStoredSystemFontSize({
      getItem: (key) => key === systemFontSizeStorageKey ? "18" : null,
    })).toBe(18);
    expect(readStoredSystemFontSize({ getItem: () => "invalid" })).toBeUndefined();
    expect(parseSystemFontSize(" ")).toBeUndefined();
  });

  it("updates the root font size and markdown font size variable", () => {
    const properties = new Map<string, string>();
    const root = {
      style: {
        fontSize: "",
        setProperty: (name: string, value: string) => properties.set(name, value),
      },
    };

    applySystemFontSize(18, root);

    expect(root.style.fontSize).toBe("18px");
    expect(properties.get(systemFontSizeCssVariable)).toBe("18px");
  });
});

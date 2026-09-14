import { describe, expect, it } from "vitest";

import { containsNullByte, extensionOf, isBinaryExtension } from "./binaryGuard";

describe("binaryGuard", () => {
  it("extensionOf returns the lower-cased trailing extension", () => {
    expect(extensionOf("a.TSX")).toBe("tsx");
    expect(extensionOf("a.tar.gz")).toBe("gz");
    expect(extensionOf("README")).toBe("");
    expect(extensionOf(".gitignore")).toBe("");
  });

  it("isBinaryExtension flags non-editable extensions only", () => {
    expect(isBinaryExtension("photo.png")).toBe(true);
    expect(isBinaryExtension("archive.zip")).toBe(true);
    expect(isBinaryExtension("main.rs")).toBe(false);
    expect(isBinaryExtension("index.ts")).toBe(false);
  });

  it("containsNullByte mirrors the main-process null-byte sniff", () => {
    expect(containsNullByte("abc\u0000def")).toBe(true);
    expect(containsNullByte("plain text")).toBe(false);
  });
});

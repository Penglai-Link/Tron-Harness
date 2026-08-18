import { describe, expect, it } from "vitest";

import { generateConnectionResourceCode } from "./connectionResourceCode";

describe("generateConnectionResourceCode", () => {
  it("creates opaque, distinct codes with a resource-kind prefix", () => {
    const codes = new Set(Array.from({ length: 3 }, () => generateConnectionResourceCode("DATA_SOURCE")));

    expect(codes).toHaveLength(3);
    for (const code of codes) {
      expect(code).toMatch(/^DATA_SOURCE_[A-Z0-9]+$/);
    }
  });
});

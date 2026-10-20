import { describe, expect, it } from "vitest";
import { contextModelDisplayName } from "./contextAssemblyReport";

describe("context report model label", () => {
  it("shows the concrete model instead of the PersonAI runtime route", () => {
    expect(contextModelDisplayName("PERSON_AI:FIXED:High:qianfan_plan/glm-5.3-flash"))
      .toBe("glm-5.3-flash");
  });

  it("preserves other model identifiers and an empty report label", () => {
    expect(contextModelDisplayName("custom/model")).toBe("custom/model");
    expect(contextModelDisplayName(null)).toBe("—");
  });
});

import { describe, expect, it } from "vitest";
import { contextModelDisplayName, type ContextModelDisplayOption } from "./contextAssemblyReport";

const model = (code: string, provider: string): ContextModelDisplayOption => ({
  modelCode: code, displayName: "DeepSeek V4.1 Flash 1017", modelName: "deepseek-v4.1-flash-1017",
  sourceLabel: provider, vendor: provider, accessPlatform: provider,
});

describe("context report model label", () => {
  it("resolves a configured model to its provider and display name", () => {
    expect(contextModelDisplayName("MODEL-first", [model("MODEL-first", "赛迪1")]))
      .toBe("赛迪1 / DeepSeek V4.1 Flash 1017");
  });

  it("keeps same-named models on different connections distinct", () => {
    expect(contextModelDisplayName("MODEL-second", [model("MODEL-first", "赛迪1"), model("MODEL-second", "赛迪2")]))
      .toBe("赛迪2 / DeepSeek V4.1 Flash 1017");
  });

  it("uses PersonAI's connection name instead of its upstream vendor", () => {
    const code = "PERSON_AI:FIXED:High:qianfan_plan/glm-5.3-flash";
    expect(contextModelDisplayName(code, [{ ...model(code, "Person-AI"), vendor: "Qianfan", displayName: "GLM 5.3 Flash" }]))
      .toBe("Person-AI / GLM 5.3 Flash");
  });

  it("provides a readable PersonAI fallback without its runtime route code", () => {
    expect(contextModelDisplayName("PERSON_AI:FIXED:High:qianfan_plan/glm-5.3-flash"))
      .toBe("Person-AI / glm-5.3-flash");
  });

  it("does not expose an unresolved internal model identity", () => {
    expect(contextModelDisplayName("MODEL-deleted", [model("MODEL-another", "赛迪1")])).toBe("");
    expect(contextModelDisplayName("PERSON_AI:ROUTE:ChatAuto")).toBe("");
  });

  it("preserves other model identifiers and an empty report label", () => {
    expect(contextModelDisplayName("custom/model")).toBe("custom/model");
    expect(contextModelDisplayName(null)).toBe("—");
  });
});

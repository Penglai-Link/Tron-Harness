import { describe, expect, it } from "vitest";

import {
  normalizePromptVariables,
  promptVariablesAreReady,
  reconcilePromptVariables,
} from "./promptVariables";

describe("prompt variable metadata", () => {
  it("preserves metadata by name and defaults newly detected variables to STRING", () => {
    expect(reconcilePromptVariables("{{existing}} {{created}}", [{
      description: "Existing value",
      name: "existing",
      type: "NUMBER",
    }])).toEqual([
      { description: "Existing value", name: "existing", type: "NUMBER" },
      { description: "", name: "created", type: "STRING" },
    ]);
  });

  it("requires exact names and descriptions before normalizing in template order", () => {
    const variables = [
      { description: " Second value ", name: "second", type: "BOOLEAN" as const },
      { description: "First value", name: "first", type: "STRING" as const },
    ];

    expect(promptVariablesAreReady("{{first}} {{second}}", variables)).toBe(true);
    expect(normalizePromptVariables("{{first}} {{second}}", variables)).toEqual([
      { description: "First value", name: "first", type: "STRING" },
      { description: "Second value", name: "second", type: "BOOLEAN" },
    ]);
    expect(promptVariablesAreReady("{{first}} {{missing}}", variables)).toBe(false);
  });

  it("counts multilingual descriptions by characters", () => {
    const variable = { description: "\u53c2".repeat(200), name: "input", type: "STRING" as const };

    expect(promptVariablesAreReady("{{input}}", [variable])).toBe(true);
    expect(promptVariablesAreReady("{{input}}", [{
      ...variable,
      description: `${variable.description}\u6570`,
    }])).toBe(false);
  });
});

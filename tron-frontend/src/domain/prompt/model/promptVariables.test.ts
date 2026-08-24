import { describe, expect, it } from "vitest";

import {
  normalizePromptVariables,
  promptVariablesAreReady,
  reconcilePromptVariables,
} from "./promptVariables";

describe("prompt variable metadata", () => {
  it("preserves explicit bindings by name and creates unbound managed variables", () => {
    expect(reconcilePromptVariables("{{existing}} {{created}}", [{
      description: "Existing value",
      managedVariableKey: "existing_key",
      name: "existing",
      runtimeProvider: null,
      sourceKind: "MANAGED",
      type: "NUMBER",
    }])).toEqual([
      {
        description: "Existing value",
        name: "existing",
        type: "NUMBER",
        sourceKind: "MANAGED",
        managedVariableKey: "existing_key",
        runtimeProvider: null,
      },
      {
        description: "",
        name: "created",
        type: "STRING",
        sourceKind: "MANAGED",
        managedVariableKey: null,
        runtimeProvider: null,
      },
    ]);
  });

  it("requires exact names and descriptions before normalizing in template order", () => {
    const variables = [
      {
        description: " Second value ",
        name: "second",
        type: "BOOLEAN" as const,
        sourceKind: "RUNTIME" as const,
        managedVariableKey: null,
        runtimeProvider: "CHAT_USER_MESSAGE" as const,
      },
      {
        description: "First value",
        name: "first",
        type: "STRING" as const,
        sourceKind: "RUNTIME" as const,
        managedVariableKey: null,
        runtimeProvider: "CHAT_USER_MESSAGE" as const,
      },
    ];

    expect(promptVariablesAreReady("{{first}} {{second}}", variables)).toBe(true);
    expect(normalizePromptVariables("{{first}} {{second}}", variables)).toEqual([
      {
        description: "First value",
        name: "first",
        type: "STRING",
        sourceKind: "RUNTIME",
        managedVariableKey: null,
        runtimeProvider: "CHAT_USER_MESSAGE",
      },
      {
        description: "Second value",
        name: "second",
        type: "BOOLEAN",
        sourceKind: "RUNTIME",
        managedVariableKey: null,
        runtimeProvider: "CHAT_USER_MESSAGE",
      },
    ]);
    expect(promptVariablesAreReady("{{first}} {{missing}}", variables)).toBe(false);
  });

  it("counts multilingual descriptions by characters", () => {
    const variable = {
      description: "\u53c2".repeat(200),
      name: "input",
      type: "STRING" as const,
      sourceKind: "RUNTIME" as const,
      managedVariableKey: null,
      runtimeProvider: "CHAT_USER_MESSAGE" as const,
    };

    expect(promptVariablesAreReady("{{input}}", [variable])).toBe(true);
    expect(promptVariablesAreReady("{{input}}", [{
      ...variable,
      description: `${variable.description}\u6570`,
    }])).toBe(false);
  });
});

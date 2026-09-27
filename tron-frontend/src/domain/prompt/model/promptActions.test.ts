import { describe, expect, it } from "vitest";

import { getWorkspacePromptActions } from "./promptActions";

describe("workspace prompt action matrix", () => {
  it.each([
    ["LOCAL", ["EDIT", "DUPLICATE", "SET_ENABLED", "DELETE"]],
    ["INHERITED", ["OVERRIDE", "SET_ENABLED"]],
    ["OVERRIDE", ["EDIT", "SET_ENABLED", "REVERT"]],
    ["PLATFORM", []],
  ] as const)("maps %s records to allowed actions", (sourceKind, expected) => {
    expect(getWorkspacePromptActions(sourceKind)).toEqual(expected);
  });
});

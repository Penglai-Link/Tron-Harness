import { expect, it } from "vitest";
import { reconcileResourceDraft } from "./ResourceAssignmentDraft";
it("preserves independent local additions and removals while pruning revoked resources", () => {
  expect(
    reconcileResourceDraft(
      ["new", "revoked"],
      ["old", "revoked"],
      ["old"],
      ["new", "old"]
    )
  ).toEqual(["new"]);
});
it("accepts server default grants for newly enabled resources", () => {
  expect(
    reconcileResourceDraft(["local"], [], ["enabled"], ["local", "enabled"])
  ).toEqual(["local", "enabled"]);
});
it("removes a remotely revoked assignment unless the user changed that choice", () => {
  expect(
    reconcileResourceDraft(["old", "local"], ["old"], [], ["old", "local"])
  ).toEqual(["local"]);
});

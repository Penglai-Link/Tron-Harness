import { describe, expect, it } from "vitest";
import type { NavigationAdminTreeItem } from "@/shared/types/Navigation";
import { menuDropPlacement } from "./menuDropPlacement";

function node(menuCode: string, parentMenuCode: string | null = null, children: NavigationAdminTreeItem[] = [], group = false): NavigationAdminTreeItem {
  return { menuCode, parentMenuCode, children, menuType: group ? "Group" : "InternalPage", childDisplay: "Inline" } as NavigationAdminTreeItem;
}

const tree = [node("a", null, [node("child", "a")], true), node("b", null, [], true), node("last")];

describe("menu drop placement", () => {
  it("moves a child out to the root", () => {
    expect(menuDropPlacement(tree, "child", "b", 0.1)).toMatchObject({ parent: null, position: 1, edge: "before" });
  });
  it("moves a child into a new empty group", () => {
    expect(menuDropPlacement(tree, "child", "b", 0.5)).toMatchObject({ parent: "b", position: 0, edge: "inside" });
  });
  it("moves a root into a group", () => {
    expect(menuDropPlacement(tree, "last", "a", 0.5)).toMatchObject({ parent: "a", position: 1 });
  });
  it("accounts for removal when reordering siblings", () => {
    expect(menuDropPlacement(tree, "a", "b", 0.9)).toMatchObject({ parent: null, position: 1 });
    expect(menuDropPlacement(tree, "last", "a", 0.1)).toMatchObject({ parent: null, position: 0 });
  });
  it("rejects cycles and unchanged positions", () => {
    expect(menuDropPlacement(tree, "a", "child", 0.5)).toBeNull();
    expect(menuDropPlacement(tree, "a", "a", 0.5)).toBeNull();
    expect(menuDropPlacement(tree, "a", "b", 0.1)).toBeNull();
  });
});

import type { NavigationAdminTreeItem } from "@/shared/types/Navigation";

export function menuDropPlacement(tree: NavigationAdminTreeItem[], sourceCode: string, targetCode: string, fraction: number) {
  const flatten = (items: NavigationAdminTreeItem[]): NavigationAdminTreeItem[] => items.flatMap((item) => [item, ...flatten(item.children)]);
  const nodes = flatten(tree);
  const source = nodes.find((item) => item.menuCode === sourceCode);
  const target = nodes.find((item) => item.menuCode === targetCode);
  if (!source || !target || flatten([source]).some((item) => item.menuCode === targetCode)) return null;
  const edge: "inside" | "before" | "after" = target.menuType === "Group" && fraction >= 0.25 && fraction <= 0.75
    ? "inside" : fraction < 0.5 ? "before" : "after";
  const parent = edge === "inside" ? target.menuCode : target.parentMenuCode;
  const siblings = parent === null ? tree : nodes.find((item) => item.menuCode === parent)?.children;
  if (!siblings) return null;
  const remaining = siblings.filter((item) => item.menuCode !== sourceCode);
  const position = edge === "inside" ? remaining.length : remaining.findIndex((item) => item.menuCode === targetCode) + (edge === "after" ? 1 : 0);
  if (source.parentMenuCode === parent && siblings.findIndex((item) => item.menuCode === sourceCode) === position) return null;
  return { item: source, parent, position, edge };
}

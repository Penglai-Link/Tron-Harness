import type { SecondarySpace } from "@/shared/types/WorkspaceSpace";

export interface WorkspaceTreeNode<T extends WorkspaceTreeSpace = WorkspaceTreeSpace> {
  children: WorkspaceTreeNode<T>[];
  space: T;
}

type WorkspaceTreeSpace = Pick<
  SecondarySpace,
  "depth" | "parentWorkspaceCode" | "workspaceCode"
>;

function hasExplicitParentLinks<T extends WorkspaceTreeSpace>(spaces: T[]) {
  return spaces.every((space) => "parentWorkspaceCode" in space);
}

function buildTreeFromParentLinks<T extends WorkspaceTreeSpace>(spaces: T[]) {
  const roots: WorkspaceTreeNode<T>[] = [];
  const nodes = new Map<string, WorkspaceTreeNode<T>>();

  spaces.forEach((space) => {
    nodes.set(space.workspaceCode, {
      children: [],
      space,
    });
  });

  spaces.forEach((space) => {
    const node = nodes.get(space.workspaceCode);
    if (!node) {
      return;
    }

    const parentCode = space.parentWorkspaceCode ?? null;
    const parentNode = parentCode ? nodes.get(parentCode) : undefined;
    if (parentNode) {
      parentNode.children.push(node);
      return;
    }

    roots.push(node);
  });

  return roots;
}

function buildTreeFromDepthOrder<T extends WorkspaceTreeSpace>(spaces: T[]) {
  const roots: WorkspaceTreeNode<T>[] = [];
  const stack: WorkspaceTreeNode<T>[] = [];

  spaces.forEach((space) => {
    const node: WorkspaceTreeNode<T> = {
      children: [],
      space,
    };

    while (stack.length && stack.at(-1)!.space.depth >= space.depth) {
      stack.pop();
    }

    if (stack.length) {
      stack.at(-1)!.children.push(node);
    } else {
      roots.push(node);
    }

    stack.push(node);
  });

  return roots;
}

export function buildWorkspaceTree<T extends WorkspaceTreeSpace>(spaces: T[]) {
  if (!spaces.length) {
    return [];
  }

  return hasExplicitParentLinks(spaces)
    ? buildTreeFromParentLinks(spaces)
    : buildTreeFromDepthOrder(spaces);
}

export function findWorkspaceNodePath<T extends WorkspaceTreeSpace>(
  nodes: WorkspaceTreeNode<T>[],
  targetWorkspaceCode?: string,
): WorkspaceTreeNode<T>[] {
  if (!targetWorkspaceCode) {
    return [];
  }

  for (const node of nodes) {
    if (node.space.workspaceCode === targetWorkspaceCode) {
      return [node];
    }

    const childPath = findWorkspaceNodePath(node.children, targetWorkspaceCode);
    if (childPath.length) {
      return [node, ...childPath];
    }
  }

  return [];
}

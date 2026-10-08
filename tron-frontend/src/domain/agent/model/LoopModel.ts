export type LoopKind = "React" | "Workflow" | "Dag";
export interface BranchCondition {
  sourceNode: string;
  pointer: string;
  equals: unknown;
  matchTarget: string;
  elseTarget: string;
}
export interface LoopNode {
  id: string;
  kind: "Model" | "Branch";
  instructions: string;
  join: "Selected" | "All";
  condition: BranchCondition | null;
}
export interface LoopEdge {
  from: string;
  to: string;
}
export interface LoopDesign {
  loopCode: string;
  name: string;
  kind: LoopKind;
  version: number;
  nodes: LoopNode[];
  edges: LoopEdge[];
  maxParallelNodes: number;
  updatedAt: number;
}
export const REACT_LOOP_CODE = "builtin.react-loop";
export const LOOP_DESIGNS_CHANGED = "tron:loop-designs-changed";
export function newLoopDesign(): LoopDesign {
  return {
    loopCode: "",
    name: "",
    kind: "Workflow",
    version: 0,
    nodes: [
      {
        id: "stage-1",
        kind: "Model",
        instructions: "",
        join: "Selected",
        condition: null,
      },
    ],
    edges: [],
    maxParallelNodes: 1,
    updatedAt: 0,
  };
}

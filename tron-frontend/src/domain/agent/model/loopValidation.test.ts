import { describe, it, expect } from "vitest";
import { newLoopDesign, type LoopDesign } from "./LoopModel";
import { validateLoop, visibleEdges, graphPositions } from "./loopValidation";
function design(): LoopDesign {
  return {
    ...newLoopDesign(),
    loopCode: "review-flow",
    name: "Review",
    kind: "Dag",
    nodes: [
      {
        id: "a",
        kind: "Model",
        instructions: "",
        join: "Selected",
        condition: null,
      },
      {
        id: "b",
        kind: "Model",
        instructions: "",
        join: "Selected",
        condition: null,
      },
    ],
    edges: [{ from: "a", to: "b" }],
  };
}
describe("Loop validation", () => {
  it("rejects cycles, unknown dependencies and duplicate ids", () => {
    const d = design();
    expect(validateLoop(d)).toBeNull();
    expect(
      validateLoop({ ...d, edges: [...d.edges, { from: "b", to: "a" }] }),
    ).toBe("cycle");
    expect(validateLoop({ ...d, edges: [{ from: "missing", to: "a" }] })).toBe(
      "edges",
    );
    expect(validateLoop({ ...d, nodes: [d.nodes[0], d.nodes[0]] })).toBe(
      "nodeIds",
    );
  });
  it("requires a branch source and two distinct exits", () => {
    const d = design();
    d.nodes.push(
      {
        id: "route",
        kind: "Branch",
        instructions: "",
        join: "Selected",
        condition: {
          sourceNode: "a",
          pointer: "/approved",
          equals: true,
          matchTarget: "b",
          elseTarget: "other",
        },
      },
      {
        id: "other",
        kind: "Model",
        instructions: "",
        join: "Selected",
        condition: null,
      },
    );
    d.edges = [
      { from: "a", to: "route" },
      { from: "route", to: "b" },
      { from: "route", to: "other" },
    ];
    expect(validateLoop(d)).toBeNull();
    d.nodes[2].condition!.elseTarget = "b";
    expect(validateLoop(d)).toBe("branch");
  });
  it("projects sequential order and independent root nodes", () => {
    const d = design();
    d.kind = "Workflow";
    d.edges = [];
    expect(visibleEdges(d)).toEqual([{ from: "a", to: "b" }]);
    expect(graphPositions(d).get("b")!.x).toBeGreaterThan(
      graphPositions(d).get("a")!.x,
    );
    d.kind = "Dag";
    expect(graphPositions(d).get("a")!.x).toBe(graphPositions(d).get("b")!.x);
    expect(graphPositions(d).get("a")!.y).not.toBe(
      graphPositions(d).get("b")!.y,
    );
  });
  it("checks UTF-8 instruction size and integer parallelism", () => {
    const d = design();
    d.nodes[0].instructions = "中".repeat(22000);
    expect(validateLoop(d)).toBe("size");
    d.nodes[0].instructions = "";
    d.maxParallelNodes = 1.5;
    expect(validateLoop(d)).toBe("parallel");
  });
});

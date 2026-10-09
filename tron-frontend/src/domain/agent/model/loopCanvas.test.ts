import { describe, expect, it } from "vitest";
import { newLoopDesign, type LoopDesign } from "./LoopModel";
import {
  connectLoopNodes,
  disconnectLoopEdge,
  withCanvasPositions,
  clearNodeReferences,
} from "./loopCanvas";
import { graphPositions, validateLoop } from "./loopValidation";

function design(): LoopDesign {
  const initial = newLoopDesign();
  return {
    ...initial,
    loopCode: "canvas",
    name: "Canvas",
    kind: "Dag",
    nodes: ["source", "route", "yes", "no"].map((id) => ({
      ...initial.nodes[0],
      id,
      kind: id === "route" ? "Branch" : "Model",
      condition:
        id === "route"
          ? {
              sourceNode: "",
              pointer: "/approved",
              equals: true,
              matchTarget: "",
              elseTarget: "",
            }
          : null,
    })),
  };
}
describe("DAG canvas editing", () => {
  it("connects branch input and distinct named outlets into an executable graph", () => {
    let d = connectLoopNodes(design(), 0, 1, "output");
    d = connectLoopNodes(d, 1, 2, "matched");
    d = connectLoopNodes(d, 1, 3, "otherwise");
    expect(d.nodes[1].condition).toMatchObject({
      sourceNode: "source",
      matchTarget: "yes",
      elseTarget: "no",
    });
    expect(validateLoop(d)).toBeNull();
    expect(() => connectLoopNodes(d, 1, 2, "otherwise")).toThrow("branch");
    expect(design().edges).toEqual([]);
  });
  it("replaces a branch outlet and clears references when disconnecting", () => {
    let d = connectLoopNodes(design(), 0, 1, "output");
    d = connectLoopNodes(d, 1, 2, "matched");
    d = connectLoopNodes(d, 1, 3, "matched");
    expect(d.edges).not.toContainEqual({ from: "route", to: "yes" });
    expect(d.nodes[1].condition!.matchTarget).toBe("no");
    d = disconnectLoopEdge(
      d,
      d.edges.findIndex((edge) => edge.from === "route"),
    );
    expect(d.nodes[1].condition!.matchTarget).toBe("");
    d = disconnectLoopEdge(d, 0);
    expect(d.nodes[1].condition!.sourceNode).toBe("");
  });
  it("rejects self connections and cycles without changing the graph", () => {
    const d = connectLoopNodes(design(), 0, 2, "output");
    expect(() => connectLoopNodes(d, 2, 0, "output")).toThrow("cycle");
    expect(() => connectLoopNodes(d, 0, 0, "output")).toThrow("cycle");
    expect(d.edges).toEqual([{ from: "source", to: "yes" }]);
    expect(connectLoopNodes(d, 0, 2, "output").edges).toHaveLength(1);
  });
  it("keeps coordinates stable while connecting and restores saved positions", () => {
    const d = withCanvasPositions(design());
    d.nodes[0].position = { x: -120, y: 420 };
    const before = graphPositions(d);
    const connected = connectLoopNodes(d, 0, 2, "output");
    expect(graphPositions(connected)).toEqual(before);
    expect(
      graphPositions(JSON.parse(JSON.stringify(connected))).get("source"),
    ).toEqual({ x: -120, y: 420 });
    connected.nodes[0].position = { x: Infinity, y: 0 };
    expect(validateLoop(connected)).toBe("position");
  });
  it("clears branch references when removing a node", () => {
    const d = connectLoopNodes(design(), 0, 1, "output");
    expect(
      clearNodeReferences(d.nodes.slice(1), "source")[0].condition!.sourceNode,
    ).toBe("");
  });
});

import { describe, expect, it } from "vitest";
import { graphEdgeSelectionStyle, graphNodeSelectionStyle, resolveGraphSelection } from "./graphSelection";

const nodes = ["selected", "incoming", "outgoing", "farther", "unrelated", "isolated"].map(nodeCode => ({ nodeCode }));
const edges = [
  { edgeCode: "in", sourceNodeCode: "incoming", targetNodeCode: "selected" },
  { edgeCode: "out", sourceNodeCode: "selected", targetNodeCode: "outgoing" },
  { edgeCode: "second-hop", sourceNodeCode: "outgoing", targetNodeCode: "farther" },
  { edgeCode: "elsewhere", sourceNodeCode: "farther", targetNodeCode: "unrelated" },
];

describe("graph selection focus", () => {
  it("keeps both incoming and outgoing neighbors clear and dims unrelated nodes, labels and edges", () => {
    const focus = resolveGraphSelection(nodes, edges, "selected", ["unrelated"], ["elsewhere"]);
    expect(focus.nodes).toEqual(new Set(["selected", "incoming", "outgoing"]));
    expect(focus.edges).toEqual(new Set(["in", "out"]));
    for (const id of ["selected", "incoming", "outgoing"]) {
      expect(graphNodeSelectionStyle(focus, id)).toEqual({ opacity: 1, labelOpacity: 1 });
    }
    for (const id of ["farther", "unrelated", "isolated"]) {
      expect(graphNodeSelectionStyle(focus, id).opacity).toBeLessThanOrEqual(0.2);
      expect(graphNodeSelectionStyle(focus, id).labelOpacity).toBeLessThanOrEqual(0.25);
    }
    expect(graphEdgeSelectionStyle(focus, "out").opacity).toBeGreaterThan(0.8);
    expect(graphEdgeSelectionStyle(focus, "second-hop").opacity).toBeLessThan(0.1);
  });

  it("replaces the focused neighborhood when another node is selected", () => {
    const focus = resolveGraphSelection(nodes, edges, "farther", [], []);
    expect(focus.nodes).toEqual(new Set(["farther", "outgoing", "unrelated"]));
    expect(focus.edges).toEqual(new Set(["second-hop", "elsewhere"]));
    expect(graphNodeSelectionStyle(focus, "selected").opacity).toBeLessThan(0.2);
  });

  it("can focus an isolated node without keeping stale highlighted nodes", () => {
    const focus = resolveGraphSelection(nodes, edges, "isolated", ["selected"], ["in"]);
    expect(focus.nodes).toEqual(new Set(["isolated"]));
    expect(focus.edges.size).toBe(0);
  });

  it.each(["", "removed-node"])("restores normal visibility after selection clears or disappears: %s", selected => {
    const focus = resolveGraphSelection(nodes, edges, selected, ["incoming"], ["in"]);
    expect(focus.active).toBe(false);
    expect(focus.nodes).toEqual(new Set(["incoming"]));
    for (const node of nodes) {
      expect(graphNodeSelectionStyle(focus, node.nodeCode).opacity).toBeGreaterThanOrEqual(0.9);
      expect(graphNodeSelectionStyle(focus, node.nodeCode).labelOpacity).toBe(1);
    }
    expect(graphEdgeSelectionStyle(focus, "elsewhere").opacity).toBeGreaterThan(0.3);
  });
});

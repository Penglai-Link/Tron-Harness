import { describe, expect, it } from "vitest";
import { mergeRagGraphWindow } from "./ragGraphWindow";
import type { RagGraphWindow } from "./RagGraphModel";
export function graphWindow(snapshotId = "snapshot"): RagGraphWindow {
  return { snapshotId, seeds: [{ id: "a", score: 1, channels: ["fulltext"] }],
    nodes: [{ id: "a", corpusId: "kb", kind: "Entity", name: "Alpha", entityType: "Service", aliases: [], descriptions: [{ text: "Alpha calls Beta.", referenceId: "r1" }], referenceIds: ["r1"], hops: [{ seedId: "a", nodeId: "a", direction: "Outgoing", hops: 0 }] }],
    edges: [{ id: "rel", from: "a", to: "b", relationship: "CALLS", structural: false, referenceIds: ["r1"] }],
    references: [{ id: "r1", corpusId: "kb", sourceCode: "source", sourceVersion: "v1", generationId: "g1", chunkCode: "c1", fileName: "manual.md", documentId: "doc", resourceCode: null, workspaceCode: "PLATFORM", quote: "Alpha calls Beta.", location: { page: 4 } }],
    stages: [], nextCursor: "cursor1", expiresAt: 100, unfinished: [], truncatedBy: ["EdgeBudget"], durationMs: 10 };
}
describe("incremental graph windows", () => {
  it("keeps all source supports when two windows contain the same logical relationship", () => {
    const current = graphWindow(); const next = graphWindow();
    next.nodes[0].referenceIds = ["r2"]; next.nodes[0].descriptions = [{ text: "Another original quote.", referenceId: "r2" }];
    next.edges[0].referenceIds = ["r2"]; next.references = [{ ...next.references[0], id: "r2", fileName: "second.md" }];
    next.nextCursor = null; next.truncatedBy = [];
    const merged = mergeRagGraphWindow(current, next);
    expect(merged.nodes).toHaveLength(1); expect(merged.nodes[0].descriptions).toHaveLength(2);
    expect(merged.edges).toHaveLength(1); expect(merged.edges[0].referenceIds).toEqual(["r1", "r2"]);
    expect(merged.references.map((reference) => reference.fileName)).toEqual(["manual.md", "second.md"]);
    expect(merged.nextCursor).toBeNull(); expect(current.nodes[0].referenceIds).toEqual(["r1"]);
  });
  it("replaces the old graph when the publication snapshot changes", () => {
    const current = graphWindow("old"); const next = graphWindow("new"); next.nodes = []; next.edges = []; next.references = [];
    expect(mergeRagGraphWindow(current, next)).toBe(next);
    expect(mergeRagGraphWindow(current, next).references).toEqual([]);
  });
  it("keeps the shortest distance for each seed and direction", () => {
    const current = graphWindow(); current.nodes[0].hops[0].hops = 3;
    const next = graphWindow(); next.nodes[0].hops[0].hops = 1;
    expect(mergeRagGraphWindow(current, next).nodes[0].hops).toEqual([{ seedId: "a", nodeId: "a", direction: "Outgoing", hops: 1 }]);
  });
});

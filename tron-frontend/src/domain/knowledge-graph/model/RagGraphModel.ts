export type RagGraphDirection = "Incoming" | "Outgoing";
export interface RagGraphOptions {
  seedCount: number; incomingDepth: number; outgoingDepth: number; nodeLimit: number; edgeLimit: number;
  relationshipTypes: string[]; includeStructure: boolean;
}
export const defaultRagGraphOptions: RagGraphOptions = {
  seedCount: 10, incomingDepth: 3, outgoingDepth: 3, nodeLimit: 100, edgeLimit: 200,
  relationshipTypes: [], includeStructure: false,
};
export interface RagGraphText { text: string; referenceId: string }
export interface RagGraphNode {
  id: string; corpusId: string; kind: "Entity" | "SourceVersion" | "Section" | "Chunk" | "SourceBlock" | "Table" | "Image" | "Asset" | "SourceCaption" | "OcrText" | "GeneratedDescription";
  name: string; entityType: string | null; descriptions: RagGraphText[]; aliases: RagGraphText[]; referenceIds: string[];
  hops: { seedId: string; nodeId: string; direction: RagGraphDirection; hops: number }[];
}
export interface RagGraphEdge { id: string; from: string; to: string; relationship: string; structural: boolean; referenceIds: string[] }
export interface RagGraphReference {
  id: string; corpusId: string; sourceCode: string; sourceVersion: string; generationId: string; chunkCode: string;
  fileName: string | null; documentId: string | null; resourceCode: string | null; workspaceCode: string; quote: string; location: Record<string, unknown>;
}
export interface RagGraphWindow {
  snapshotId: string;
  seeds: { id: string; score: number; channels: string[] }[]; nodes: RagGraphNode[]; edges: RagGraphEdge[]; references: RagGraphReference[];
  stages: { corpusId: string; channel: string; status: "Completed" | "Truncated" | "Failed" | "Invalidated" | "Skipped"; hitCount: number; durationMs: number; errorKey: string | null }[];
  nextCursor: string | null; expiresAt: number | null;
  unfinished: { nodeId: string; direction: RagGraphDirection | null; depth: number | null; phase: "Traversal" | "Provenance" }[];
  truncatedBy: string[]; durationMs: number;
}
export interface RagGraphSearchRequest extends RagGraphOptions { query: string; corpusIds: string[]; filters: Record<string, unknown> }
export interface RagGraphBrowseRequest extends RagGraphOptions { corpusIds: string[]; filters: Record<string, unknown> }
export type RagGraphExpandRequest = ({ nodeIds: string[]; corpusIds: string[]; filters: Record<string, unknown> } & RagGraphOptions) | { cursor: string };

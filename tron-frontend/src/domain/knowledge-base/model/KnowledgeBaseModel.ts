export interface KnowledgeBaseRecord {
  chunkProfile: string;
  /** Counts are optional because the RAG management API exposes configuration, not index metrics. */
  documentCount: number | null;
  embeddingFootprint: string;
  freshness: string;
  id: string;
  name: string;
  ownerWorkspace: string;
  retrievalProfile: string;
  sourceCount: number | null;
  status: "Active" | "Indexing" | "Draft" | "Disabled" | "Reserved" | "Warning";
  syncLagMinutes: number | null;
}

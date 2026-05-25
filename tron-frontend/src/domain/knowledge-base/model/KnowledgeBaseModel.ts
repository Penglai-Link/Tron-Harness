export interface KnowledgeBaseRecord {
  chunkProfile: string;
  documentCount: number;
  embeddingFootprint: string;
  freshness: string;
  id: string;
  name: string;
  ownerWorkspace: string;
  retrievalProfile: string;
  sourceCount: number;
  status: "Active" | "Indexing" | "Draft";
  syncLagMinutes: number;
}

import type { KnowledgeBaseRecord } from "@/domain/knowledge-base/model/KnowledgeBaseModel";

const knowledgeBaseRecords: KnowledgeBaseRecord[] = [
  {
    id: "kb-001",
    name: "Tron Product Manuals",
    ownerWorkspace: "Industrial Internet / EliteForge",
    documentCount: 1248,
    sourceCount: 18,
    chunkProfile: "semantic-768",
    retrievalProfile: "hybrid-rag",
    embeddingFootprint: "42 GB",
    freshness: "5 min ago",
    syncLagMinutes: 5,
    status: "Active",
  },
  {
    id: "kb-002",
    name: "Operations SOP Library",
    ownerWorkspace: "Industrial Internet / EliteForge / Online Docs",
    documentCount: 684,
    sourceCount: 9,
    chunkProfile: "policy-longform",
    retrievalProfile: "keyword+rerank",
    embeddingFootprint: "18 GB",
    freshness: "19 min ago",
    syncLagMinutes: 19,
    status: "Active",
  },
  {
    id: "kb-003",
    name: "Field Incident Archives",
    ownerWorkspace: "Industrial Internet / Platform",
    documentCount: 9320,
    sourceCount: 31,
    chunkProfile: "incident-512",
    retrievalProfile: "vector-first",
    embeddingFootprint: "126 GB",
    freshness: "Indexing now",
    syncLagMinutes: 0,
    status: "Indexing",
  },
  {
    id: "kb-004",
    name: "Regulation Draft Workspace",
    ownerWorkspace: "Compliance",
    documentCount: 147,
    sourceCount: 4,
    chunkProfile: "legal-1024",
    retrievalProfile: "citation-rag",
    embeddingFootprint: "6 GB",
    freshness: "Not indexed yet",
    syncLagMinutes: 999,
    status: "Draft",
  },
];

export function listKnowledgeBases() {
  return knowledgeBaseRecords;
}

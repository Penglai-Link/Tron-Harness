export interface ChunkConfig {
  mode: "AUTO" | "HEADING" | "FIXED";
  maxChars: number;
  overlapChars: number;
  headingDepth: number;
}

export const defaultChunkConfig: ChunkConfig = {
  mode: "AUTO", maxChars: 1200, overlapChars: 120, headingDepth: 2,
};

export type ParsingTier = "flash" | "basic" | "standard" | "advanced";
export interface ParsingCapabilities { defaultTier: ParsingTier; availableTiers: ParsingTier[]; parserVersion: string; officeTier: ParsingTier }

export interface ImageAnalysisSummary {
  imageCount: number;
  originalsOnly: number;
  withSourceAnnotations: number;
  withGeneratedDescriptions: number;
  withUnattributedBodyText: number;
  generationMode: "disabled_by_tier" | "parser_controlled";
  supplementStatus?: "disabled" | "pending" | "ready";
}
export interface ImageAnnotation {
  kind: "source_caption" | "source_footnote" | "generated_description" | "parser_body_text";
  source: string;
  text: string;
  page?: number;
  block: string;
  modelCode?: string;
  artifactResourceCode?: string;
}
export interface FidelityIssue {
  sourceRegion?: [number, number, number, number] | null;
  candidateRegion?: [number, number, number, number] | null;
  candidateExcerpt?: string | null;
  kind: string;
  page?: number | null;
  blocks: string[];
  sourceExcerpt: string;
  imageResourceCodes: string[];
}

export interface KnowledgeDocument {
  taskId?: string | null;
  id: string;
  name: string;
  fileName: string;
  resourceCode: string;
  resourceRevision: number;
  chunkConfig: ChunkConfig | null;
  parsingTier?: ParsingTier | null;
  parsingArtifact?: { effectiveTier: ParsingTier; parserVersion: string; resourceCode: string; imageResourceCodes: string[]; fidelityChecked?: boolean; fidelityIssues?: FidelityIssue[] } | null;
  visualSource?: { originalSha256: string; derivedSha256: string; derivedResourceCode: string; auditResourceCode: string; pageCount: number; pageImageSha256: string[]; originalFindings: FidelityIssue[]; renderer: string } | null;
  status: string;
  stage?: string | null;
  waitReason?: IngestionWaitReason | null;
  error: string | null;
  warnings?: string[];
  imageAnalysis?: ImageAnalysisSummary | null;
  chunkCount: number;
  indexRevision: number;
}
export interface IngestionWaitReason {
  code: string;
  params?: Record<string, string | number | null> | null;
  details?: IngestionWaitReason[];
}

export interface KnowledgeLibrary {
  id: string;
  name: string;
  description: string;
  workspaceCode: string;
  documentCount: number;
  status?: string;
}

export interface KnowledgeLibraryDetail extends KnowledgeLibrary {
  documents: KnowledgeDocument[];
  agentCodes: string[];
  defaultChunkConfig: ChunkConfig;
}

export interface DocumentLocation {
  bbox?: [number, number, number, number] | null;
  page?: number;
  logicalPage?: number;
  renderedPage?: number;
  chartSources?: string[];
  cellRanges?: string[];
  heading?: string;
  sheet?: string;
  rowStart?: number;
  rowEnd?: number;
  paragraph?: number;
  paragraphEnd?: number;
  lineStart?: number;
  lineEnd?: number;
  blocks?: string[];
  imageResourceCodes?: string[];
  imageAnnotations?: ImageAnnotation[];
  charStart: number;
  charEnd: number;
}

export interface ChunkPreview {
  fidelityIssues?: FidelityIssue[];
  chunkCount: number;
  imageAnalysis?: ImageAnalysisSummary | null;
  warnings?: string[];
  chunks: { text: string; title: string; headingPath: string[]; ordinal: number; location: DocumentLocation }[];
}

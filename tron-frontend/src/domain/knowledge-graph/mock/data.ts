import type {
  GraphEdgeView,
  GraphNodeView,
  KnowledgeGraphView,
} from "@/domain/knowledge-graph/model/KnowledgeGraphModel";

const timestamp = "2026-06-23T08:00:00Z";

function node(node: Omit<GraphNodeView, "createdAt" | "updatedAt">): GraphNodeView {
  return {
    createdAt: timestamp,
    updatedAt: timestamp,
    ...node,
  };
}

function edge(edge: Omit<GraphEdgeView, "createdAt" | "updatedAt">): GraphEdgeView {
  return {
    createdAt: timestamp,
    updatedAt: timestamp,
    ...edge,
  };
}

export function getKnowledgeGraphMock(): KnowledgeGraphView {
  const nodes: GraphNodeView[] = [
    node({
      nodeCode: "RAG_REVIEWER",
      labels: ["RagSkill"],
      x: 430,
      y: 290,
      score: 0.98,
      fullText: "Skill: rag-reviewer\nDescription: Reviews RAG retrieval changes before release.",
      embeddingText: "Reviews RAG retrieval changes before release.",
      properties: {
        corpusId: "skill-graph",
        embeddingJson: "indexed",
        skillDescription: "Reviews RAG retrieval changes before release.",
        skillName: "rag-reviewer",
        sourceCode: "skill:TRON_CORE:rag-reviewer",
        sourceVersion: "v1",
        status: "ACTIVE",
      },
    }),
    node({
      nodeCode: "GIT",
      labels: ["RagTool"],
      x: 245,
      y: 190,
      score: 0.95,
      fullText: "Git operations",
      properties: {
        builtin: true,
        confidence: 0.95,
        provider: "git",
        priority: 10,
      },
    }),
    node({
      nodeCode: "RAG_REVIEW",
      labels: ["RagTag"],
      x: 250,
      y: 400,
      score: 0.91,
      fullText: "RAG review workflow",
      properties: {
        category: "review",
      },
    }),
    node({
      nodeCode: "RAG_PROMPT_TEMPLATE",
      labels: ["RagPromptTemplate"],
      x: 640,
      y: 165,
      score: 0.89,
      fullText: "Prompt template for RAG graph search reviews.",
      properties: {
        promptCode: "rag-review-template",
        provider: "prompt",
        status: "ACTIVE",
      },
    }),
    node({
      nodeCode: "RAG_SKILL_TEXT_UNIT_001",
      labels: ["RagSkillTextUnit"],
      x: 645,
      y: 390,
      score: 0.84,
      fullText: "Reviews RAG retrieval changes before release.",
      properties: {
        sourceCode: "skill:TRON_CORE:rag-reviewer",
        sourceVersion: "v1",
        textUnitStatus: "ACTIVE",
      },
    }),
    node({
      nodeCode: "RAG_SKILL_FULLTEXT_001",
      labels: ["RagSkillFulltext"],
      x: 790,
      y: 305,
      score: 0.78,
      properties: {
        indexStatus: "INDEXED",
        sourceVersion: "v1",
      },
    }),
    node({
      nodeCode: "RAG_SKILL_VECTOR_001",
      labels: ["RagSkillVector"],
      x: 780,
      y: 475,
      score: 0.76,
      embeddingText: "Reviews RAG retrieval changes before release.",
      properties: {
        embeddingJson: "indexed",
        embeddingModelCode: "FAKE_EMBEDDING",
      },
    }),
    node({
      nodeCode: "ROOT_NODE",
      labels: ["TronGraphNode"],
      x: 118,
      y: 302,
      score: 0.72,
      fullText: "root task",
      properties: {
        priority: 3,
        workspaceCode: "WS-ROOT",
      },
    }),
    node({
      nodeCode: "CHILD_NODE",
      labels: ["TronGraphNode"],
      x: 118,
      y: 500,
      score: 0.7,
      fullText: "child task",
      properties: {
        priority: 4,
        workspaceCode: "TRON_CORE",
      },
    }),
    node({
      nodeCode: "RAG_MAPPER",
      labels: ["RagSkill"],
      x: 472,
      y: 92,
      score: 0.68,
      properties: {
        corpusId: "skill-graph",
        skillName: "rag-mapper",
        sourceVersion: "v1",
      },
    }),
    node({
      nodeCode: "RAG_PROMPTER",
      labels: ["RagSkill"],
      x: 548,
      y: 526,
      score: 0.66,
      properties: {
        corpusId: "skill-graph",
        skillName: "rag-prompter",
        sourceVersion: "v1",
      },
    }),
    node({
      nodeCode: "CODE_SEARCH",
      labels: ["RagTool"],
      x: 362,
      y: 62,
      score: 0.64,
      properties: {
        provider: "search",
        priority: 8,
      },
    }),
    node({
      nodeCode: "SOURCE_CONTROL",
      labels: ["RagTag"],
      x: 90,
      y: 154,
      score: 0.61,
      properties: {
        category: "tooling",
      },
    }),
    node({
      nodeCode: "TASK",
      labels: ["OntologyClass"],
      x: 900,
      y: 125,
      score: 0.6,
      properties: {
        ontologyDomain: "Development",
        resourceType: "class",
      },
    }),
    node({
      nodeCode: "TASK_PROPERTY",
      labels: ["OntologyProperty"],
      x: 920,
      y: 250,
      score: 0.58,
      properties: {
        ontologyDomain: "Development",
        resourceType: "property",
      },
    }),
    node({
      nodeCode: "TASK_SHAPE",
      labels: ["OntologyShape"],
      x: 916,
      y: 382,
      score: 0.57,
      properties: {
        ontologyDomain: "Development",
        resourceType: "shape",
      },
    }),
    node({
      nodeCode: "MEMORY_CONTEXT",
      labels: ["RagTag"],
      x: 360,
      y: 550,
      score: 0.55,
      properties: {
        category: "memory",
      },
    }),
    node({
      nodeCode: "SKILL_SOURCE_001",
      labels: ["RagSkillSource"],
      x: 565,
      y: 650,
      score: 0.53,
      properties: {
        sourceCode: "skill:TRON_CORE:rag-reviewer",
        sourceStatus: "ACTIVE",
      },
    }),
  ];

  const edges: GraphEdgeView[] = [
    edge({
      edgeCode: "RAG_REVIEWER_REQUIRES_GIT",
      sourceNodeCode: "RAG_REVIEWER",
      targetNodeCode: "GIT",
      relationship: "REQUIRES_TOOL",
      properties: {
        required: true,
        usage: "source-control",
        weight: 0.9,
      },
    }),
    edge({
      edgeCode: "RAG_REVIEWER_HAS_REVIEW_TAG",
      sourceNodeCode: "RAG_REVIEWER",
      targetNodeCode: "RAG_REVIEW",
      relationship: "HAS_TAG",
      properties: {},
    }),
    edge({
      edgeCode: "RAG_REVIEWER_USES_PROMPT_TEMPLATE",
      sourceNodeCode: "RAG_REVIEWER",
      targetNodeCode: "RAG_PROMPT_TEMPLATE",
      relationship: "USES_PROMPT_TEMPLATE",
      properties: {
        mode: "review",
      },
    }),
    edge({
      edgeCode: "RAG_REVIEWER_TO_TEXT_UNIT",
      sourceNodeCode: "RAG_REVIEWER",
      targetNodeCode: "RAG_SKILL_TEXT_UNIT_001",
      relationship: "LINKS",
      properties: {
        sourceVersion: "v1",
      },
    }),
    edge({
      edgeCode: "TEXT_UNIT_TO_FULLTEXT",
      sourceNodeCode: "RAG_SKILL_TEXT_UNIT_001",
      targetNodeCode: "RAG_SKILL_FULLTEXT_001",
      relationship: "LINKS",
      properties: {},
    }),
    edge({
      edgeCode: "TEXT_UNIT_TO_VECTOR",
      sourceNodeCode: "RAG_SKILL_TEXT_UNIT_001",
      targetNodeCode: "RAG_SKILL_VECTOR_001",
      relationship: "LINKS",
      properties: {},
    }),
    edge({
      edgeCode: "ROOT_TO_CHILD",
      sourceNodeCode: "ROOT_NODE",
      targetNodeCode: "CHILD_NODE",
      relationship: "RELATES_TO",
      properties: {
        weight: 9.5,
      },
    }),
    edge({
      edgeCode: "RAG_MAPPER_REQUIRES_SEARCH",
      sourceNodeCode: "RAG_MAPPER",
      targetNodeCode: "CODE_SEARCH",
      relationship: "REQUIRES_TOOL",
      properties: {
        required: true,
      },
    }),
    edge({
      edgeCode: "GIT_HAS_SOURCE_CONTROL_TAG",
      sourceNodeCode: "GIT",
      targetNodeCode: "SOURCE_CONTROL",
      relationship: "HAS_TAG",
      properties: {},
    }),
    edge({
      edgeCode: "RAG_PROMPTER_USES_TEMPLATE",
      sourceNodeCode: "RAG_PROMPTER",
      targetNodeCode: "RAG_PROMPT_TEMPLATE",
      relationship: "USES_PROMPT_TEMPLATE",
      properties: {},
    }),
    edge({
      edgeCode: "TASK_PROPERTY_RELATES_TASK",
      sourceNodeCode: "TASK_PROPERTY",
      targetNodeCode: "TASK",
      relationship: "RELATES_TO",
      properties: {},
    }),
    edge({
      edgeCode: "TASK_SHAPE_RELATES_TASK",
      sourceNodeCode: "TASK_SHAPE",
      targetNodeCode: "TASK",
      relationship: "RELATES_TO",
      properties: {},
    }),
    edge({
      edgeCode: "SKILL_SOURCE_TO_REVIEWER",
      sourceNodeCode: "SKILL_SOURCE_001",
      targetNodeCode: "RAG_REVIEWER",
      relationship: "LINKS",
      properties: {},
    }),
    edge({
      edgeCode: "RAG_REVIEWER_HAS_MEMORY_CONTEXT",
      sourceNodeCode: "RAG_REVIEWER",
      targetNodeCode: "MEMORY_CONTEXT",
      relationship: "HAS_TAG",
      properties: {},
    }),
  ];

  return {
    edges,
    facets: [
      { label: "RagSkill", value: "RagSkill", count: 3 },
      { label: "RagTool", value: "RagTool", count: 2 },
      { label: "RagTag", value: "RagTag", count: 3 },
      { label: "RagSkillTextUnit", value: "RagSkillTextUnit", count: 1 },
      { label: "RagSkillVector", value: "RagSkillVector", count: 1 },
      { label: "Ontology", value: "OntologyClass", count: 3 },
    ],
    metrics: {
      edgeCount: 3942,
      namespaceCount: 6,
      nodeCount: 1284,
    },
    namespaces: [
      "rag_skill",
      "rag_skill_text_units",
      "rag_skill_fulltext",
      "rag_skill_vectors",
      "knowledge-graph",
      "audit-graph",
    ],
    nodes,
    paths: [
      {
        id: "rag-reviewer-runtime",
        label: "RAG_REVIEWER runtime context",
        nodeCodes: ["RAG_REVIEWER", "GIT", "RAG_REVIEW", "RAG_PROMPT_TEMPLATE"],
        edgeCodes: [
          "RAG_REVIEWER_REQUIRES_GIT",
          "RAG_REVIEWER_HAS_REVIEW_TAG",
          "RAG_REVIEWER_USES_PROMPT_TEMPLATE",
        ],
      },
    ],
    scope: {
      appCode: "__NONE__",
      currentWorkspaceCode: "TRON_CORE",
      namedGraph: "WS-ROOT",
      namespace: "rag_skill",
      rootWorkspaceCode: "WS-ROOT",
      schema: "rag_skill",
      userCode: "__NONE__",
      visibleWorkspaceCodes: ["WS-ROOT", "TRON_CORE"],
      workspaceCode: "TRON_CORE",
    },
  };
}

export const defaultKnowledgeGraphMock = getKnowledgeGraphMock();

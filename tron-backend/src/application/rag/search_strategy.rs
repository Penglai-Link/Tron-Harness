use crate::domain::rag::RagError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RetrieverKind {
    Fulltext,
    Vector,
    StructuredFilter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RetrieverStrategy {
    pub kind: RetrieverKind,
    pub storage_retriever: &'static str,
}

impl RetrieverStrategy {
    pub fn needs_query_embedding(&self) -> bool {
        self.kind == RetrieverKind::Vector
    }

    pub fn can_run_without_query(&self) -> bool {
        self.kind == RetrieverKind::StructuredFilter
    }

    pub fn is_structured_filter(&self) -> bool {
        self.kind == RetrieverKind::StructuredFilter
    }

    pub fn should_skip_without_embedding(&self) -> bool {
        self.kind == RetrieverKind::Vector
    }

    pub fn unavailable_code(&self) -> &'static str {
        match self.kind {
            RetrieverKind::Fulltext => "RAG_FULLTEXT_ROUTE_UNAVAILABLE",
            RetrieverKind::Vector => "RAG_VECTOR_ROUTE_UNAVAILABLE",
            RetrieverKind::StructuredFilter => "RAG_STRUCTURED_FILTER_ROUTE_UNAVAILABLE",
        }
    }

    pub fn unavailable_message(&self) -> &'static str {
        match self.kind {
            RetrieverKind::Fulltext => "Fulltext route unavailable",
            RetrieverKind::Vector => "Vector route skipped; fulltext route remains available",
            RetrieverKind::StructuredFilter => "Structured filter route unavailable",
        }
    }
}

pub(super) fn retriever_strategy(strategy_id: &str) -> Result<RetrieverStrategy, RagError> {
    match strategy_id {
        "FULLTEXT" | "FULLTEXT_SQLITE_LIKE_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Fulltext,
            storage_retriever: "FULLTEXT",
        }),
        "FULLTEXT_SQLITE_FTS5_SIMPLE_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Fulltext,
            storage_retriever: "FULLTEXT_SQLITE_FTS5_SIMPLE_V1",
        }),
        "VECTOR" | "VECTOR_SQLITE_COSINE_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Vector,
            storage_retriever: "VECTOR",
        }),
        "VECTOR_SQLITE_VEC0_COSINE_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Vector,
            storage_retriever: "VECTOR_SQLITE_VEC0_COSINE_V1",
        }),
        "FULLTEXT_GRAPH_SINGLE_NODE_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Fulltext,
            storage_retriever: "FULLTEXT_GRAPH_SINGLE_NODE_V1",
        }),
        "VECTOR_GRAPH_SINGLE_NODE_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Vector,
            storage_retriever: "VECTOR_GRAPH_SINGLE_NODE_V1",
        }),
        "STRUCTURED_FILTER_SQLITE_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::StructuredFilter,
            storage_retriever: "STRUCTURED_FILTER_SQLITE_V1",
        }),
        "TREE_FULLTEXT_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Fulltext,
            storage_retriever: "TREE_FULLTEXT_V1",
        }),
        "TREE_VECTOR_V1" => Ok(RetrieverStrategy {
            kind: RetrieverKind::Vector,
            storage_retriever: "TREE_VECTOR_V1",
        }),
        _ => Err(RagError::SearchRouteUnavailable),
    }
}

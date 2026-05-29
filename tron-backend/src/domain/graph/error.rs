use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphScopeError {
    InvalidNamespace,
    InvalidWorkspaceCode,
    ArchivedWorkspace,
    EmptyWorkspaceLineage,
    WorkspaceResolutionFailed,
    ReservedSentinelValue,
    ReservedPropertyName,
}

impl fmt::Display for GraphScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNamespace => formatter.write_str("invalid graph namespace"),
            Self::InvalidWorkspaceCode => formatter.write_str("invalid graph workspace code"),
            Self::ArchivedWorkspace => formatter.write_str("graph workspace is archived"),
            Self::EmptyWorkspaceLineage => formatter.write_str("graph workspace lineage is empty"),
            Self::WorkspaceResolutionFailed => {
                formatter.write_str("graph workspace resolution failed")
            }
            Self::ReservedSentinelValue => formatter.write_str("reserved graph sentinel value"),
            Self::ReservedPropertyName => {
                formatter.write_str("reserved graph storage property name")
            }
        }
    }
}

impl Error for GraphScopeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    InvalidGraphCode,
    LabelRequired,
    RelationshipRequired,
    ReservedPropertyName,
    DuplicateNode { node_code: String },
    DuplicateEdge { edge_code: String },
    MissingNode { node_code: String },
    MissingEdge { edge_code: String },
    MissingEndpoint { node_code: String },
    EmptySearchQuery,
    InvalidTraversalQuery,
    RawQueryNotAllowed,
    CallerSuppliedVectorNotAllowed,
    UnsupportedStoreBehavior,
    StorageFailure,
    Scope(GraphScopeError),
}

impl fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGraphCode => formatter.write_str("invalid graph code"),
            Self::LabelRequired => formatter.write_str("graph label is required"),
            Self::RelationshipRequired => formatter.write_str("graph relationship is required"),
            Self::ReservedPropertyName => {
                formatter.write_str("reserved graph storage property name")
            }
            Self::DuplicateNode { node_code } => {
                write!(formatter, "graph node already exists: {node_code}")
            }
            Self::DuplicateEdge { edge_code } => {
                write!(formatter, "graph edge already exists: {edge_code}")
            }
            Self::MissingNode { node_code } => {
                write!(formatter, "graph node was not found: {node_code}")
            }
            Self::MissingEdge { edge_code } => {
                write!(formatter, "graph edge was not found: {edge_code}")
            }
            Self::MissingEndpoint { node_code } => {
                write!(formatter, "graph edge endpoint was not found: {node_code}")
            }
            Self::EmptySearchQuery => formatter.write_str("graph search query is empty"),
            Self::InvalidTraversalQuery => formatter.write_str("graph traversal query is invalid"),
            Self::RawQueryNotAllowed => {
                formatter.write_str("graph text query bypass is not allowed")
            }
            Self::CallerSuppliedVectorNotAllowed => {
                formatter.write_str("caller supplied graph vectors are not allowed")
            }
            Self::UnsupportedStoreBehavior => {
                formatter.write_str("graph store behavior is not supported")
            }
            Self::StorageFailure => formatter.write_str("graph storage failure"),
            Self::Scope(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for GraphError {}

impl From<GraphScopeError> for GraphError {
    fn from(error: GraphScopeError) -> Self {
        Self::Scope(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OntologyError {
    InvalidOntologyCode,
    InvalidOntologyDomain,
    InvalidOntologyResource,
    InvalidOntologyQuery,
    EmptyOntologySearchQuery,
    DuplicateOntologyResource {
        resource_type: &'static str,
        resource_code: String,
    },
    MissingOntologyResource {
        resource_type: &'static str,
        resource_code: String,
    },
    InconsistentOntologyResource,
    RawRdfNotAllowed,
    SparqlNotAllowed,
    TurtleTextNotAllowed,
    ShaclTextNotAllowed,
    UnsupportedStoreBehavior,
    StorageFailure,
    Scope(GraphScopeError),
}

impl fmt::Display for OntologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOntologyCode => formatter.write_str("invalid ontology code"),
            Self::InvalidOntologyDomain => formatter.write_str("invalid ontology domain"),
            Self::InvalidOntologyResource => formatter.write_str("invalid ontology resource"),
            Self::InvalidOntologyQuery => formatter.write_str("invalid ontology query"),
            Self::EmptyOntologySearchQuery => formatter.write_str("ontology search query is empty"),
            Self::DuplicateOntologyResource {
                resource_type,
                resource_code,
            } => write!(
                formatter,
                "ontology {resource_type} already exists: {resource_code}"
            ),
            Self::MissingOntologyResource {
                resource_type,
                resource_code,
            } => write!(
                formatter,
                "ontology {resource_type} was not found: {resource_code}"
            ),
            Self::InconsistentOntologyResource => {
                formatter.write_str("ontology resource is inconsistent")
            }
            Self::RawRdfNotAllowed
            | Self::SparqlNotAllowed
            | Self::TurtleTextNotAllowed
            | Self::ShaclTextNotAllowed => {
                formatter.write_str("ontology text payload bypass is not allowed")
            }
            Self::UnsupportedStoreBehavior => {
                formatter.write_str("ontology store behavior is not supported")
            }
            Self::StorageFailure => formatter.write_str("ontology storage failure"),
            Self::Scope(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for OntologyError {}

impl From<GraphScopeError> for OntologyError {
    fn from(error: GraphScopeError) -> Self {
        Self::Scope(error)
    }
}

use std::collections::{BTreeMap, BTreeSet};

use super::{ensure_not_reserved_storage_property, error::GraphError};

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub node_code: String,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, GraphPropertyValue>,
    pub full_text: Option<String>,
    pub embedding_text: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub edge_code: String,
    pub source_node_code: String,
    pub target_node_code: String,
    pub relationship: String,
    pub properties: BTreeMap<String, GraphPropertyValue>,
    pub full_text: Option<String>,
    pub embedding_text: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNodeWrite {
    pub node_code: String,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, GraphPropertyValue>,
    pub full_text: Option<String>,
    pub embedding_text: Option<String>,
    pub embedding_model_code: Option<String>,
}

impl GraphNodeWrite {
    pub fn validate(self) -> Result<Self, GraphError> {
        Ok(Self {
            node_code: normalize_graph_code(&self.node_code)?,
            labels: normalize_labels(self.labels)?,
            properties: normalize_properties(self.properties)?,
            full_text: normalize_optional_text(self.full_text),
            embedding_text: normalize_optional_text(self.embedding_text),
            embedding_model_code: normalize_optional_graph_code(self.embedding_model_code)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdgeWrite {
    pub edge_code: String,
    pub source_node_code: String,
    pub target_node_code: String,
    pub relationship: String,
    pub properties: BTreeMap<String, GraphPropertyValue>,
    pub full_text: Option<String>,
    pub embedding_text: Option<String>,
    pub embedding_model_code: Option<String>,
}

impl GraphEdgeWrite {
    pub fn validate(self) -> Result<Self, GraphError> {
        Ok(Self {
            edge_code: normalize_graph_code(&self.edge_code)?,
            source_node_code: normalize_graph_code(&self.source_node_code)?,
            target_node_code: normalize_graph_code(&self.target_node_code)?,
            relationship: normalize_relationship(&self.relationship)?,
            properties: normalize_properties(self.properties)?,
            full_text: normalize_optional_text(self.full_text),
            embedding_text: normalize_optional_text(self.embedding_text),
            embedding_model_code: normalize_optional_graph_code(self.embedding_model_code)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphPropertyValue {
    Text(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphPropertyFilter {
    pub property_name: String,
    pub operator: GraphFilterOperator,
    pub value: GraphPropertyValue,
}

impl GraphPropertyFilter {
    fn validate(self) -> Result<Self, GraphError> {
        let property_name = normalize_property_name(&self.property_name)?;
        Ok(Self {
            property_name,
            operator: self.operator,
            value: self.value,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphFilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
    Contains,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphQueryLimit {
    value: usize,
}

impl GraphQueryLimit {
    pub fn new(value: usize) -> Result<Self, GraphError> {
        if value == 0 {
            return Err(GraphError::InvalidGraphCode);
        }

        Ok(Self { value })
    }

    pub fn value(self) -> usize {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphSearchQuery {
    pub filters: Vec<GraphPropertyFilter>,
    pub full_text: Option<String>,
    pub vector_text: Option<String>,
    pub embedding_model_code: Option<String>,
    pub traversal: Option<GraphSearchTraversal>,
    pub labels: Vec<String>,
    pub limit: GraphQueryLimit,
    pub min_score: Option<f64>,
}

impl GraphSearchQuery {
    pub fn empty() -> Self {
        Self {
            filters: Vec::new(),
            full_text: None,
            vector_text: None,
            embedding_model_code: None,
            traversal: None,
            labels: Vec::new(),
            limit: GraphQueryLimit { value: 25 },
            min_score: None,
        }
    }

    pub fn validate(self) -> Result<Self, GraphError> {
        let filters = self
            .filters
            .into_iter()
            .map(GraphPropertyFilter::validate)
            .collect::<Result<Vec<_>, _>>()?;
        let full_text = normalize_optional_text(self.full_text);
        let vector_text = normalize_optional_text(self.vector_text);
        let embedding_model_code = normalize_optional_graph_code(self.embedding_model_code)?;
        let traversal = self
            .traversal
            .map(GraphSearchTraversal::validate)
            .transpose()?;
        let labels = normalize_optional_labels(self.labels)?;

        if filters.is_empty()
            && full_text.is_none()
            && vector_text.is_none()
            && traversal.is_none()
            && labels.is_empty()
        {
            return Err(GraphError::EmptySearchQuery);
        }

        Ok(Self {
            filters,
            full_text,
            vector_text,
            embedding_model_code,
            traversal,
            labels,
            limit: self.limit,
            min_score: self.min_score,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphSearchHit {
    pub node: GraphNode,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphSearchTraversal {
    pub start_node_codes: Vec<String>,
    pub clauses: Vec<GraphTraversalClause>,
}

impl GraphSearchTraversal {
    fn validate(self) -> Result<Self, GraphError> {
        let (start_node_codes, clauses) =
            normalize_traversal_inputs(self.start_node_codes, self.clauses)?;

        Ok(Self {
            start_node_codes,
            clauses,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphTraverseQuery {
    pub start_node_codes: Vec<String>,
    pub clauses: Vec<GraphTraversalClause>,
    pub filters: Vec<GraphPropertyFilter>,
    pub limit: GraphQueryLimit,
}

impl GraphTraverseQuery {
    pub fn validate(self) -> Result<Self, GraphError> {
        let (start_node_codes, clauses) =
            normalize_traversal_inputs(self.start_node_codes, self.clauses)?;
        let filters = self
            .filters
            .into_iter()
            .map(GraphPropertyFilter::validate)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            start_node_codes,
            clauses,
            filters,
            limit: self.limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphTraversalClause {
    pub direction: GraphTraversalDirection,
    pub relationships: Vec<String>,
    pub min_depth: usize,
    pub max_depth: usize,
}

impl GraphTraversalClause {
    fn validate(self) -> Result<Self, GraphError> {
        if self.min_depth == 0 || self.max_depth < self.min_depth {
            return Err(GraphError::InvalidTraversalQuery);
        }

        Ok(Self {
            direction: self.direction,
            relationships: normalize_relationships(self.relationships)?,
            min_depth: self.min_depth,
            max_depth: self.max_depth,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphTraversalDirection {
    Incoming,
    Outgoing,
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphPath {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub fn normalize_graph_code(value: &str) -> Result<String, GraphError> {
    let value = value.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
    {
        return Err(GraphError::InvalidGraphCode);
    }

    Ok(value.to_owned())
}

fn normalize_labels(labels: Vec<String>) -> Result<Vec<String>, GraphError> {
    let labels = normalize_optional_labels(labels)?;
    if labels.is_empty() {
        return Err(GraphError::LabelRequired);
    }

    Ok(labels)
}

fn normalize_optional_labels(labels: Vec<String>) -> Result<Vec<String>, GraphError> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for label in labels {
        let label = normalize_label(&label)?;
        if seen.insert(label.clone()) {
            normalized.push(label);
        }
    }

    Ok(normalized)
}

fn normalize_label(value: &str) -> Result<String, GraphError> {
    let value = value.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
    {
        return Err(GraphError::LabelRequired);
    }

    Ok(value.to_owned())
}

fn normalize_relationship(value: &str) -> Result<String, GraphError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GraphError::RelationshipRequired);
    }
    normalize_graph_code(value).map_err(|_| GraphError::RelationshipRequired)
}

fn normalize_relationships(values: Vec<String>) -> Result<Vec<String>, GraphError> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let relationship = normalize_relationship(&value)?;
        if seen.insert(relationship.clone()) {
            normalized.push(relationship);
        }
    }

    Ok(normalized)
}

fn normalize_code_list(values: Vec<String>) -> Result<Vec<String>, GraphError> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let code = normalize_graph_code(&value)?;
        if seen.insert(code.clone()) {
            normalized.push(code);
        }
    }

    Ok(normalized)
}

fn normalize_traversal_inputs(
    start_node_codes: Vec<String>,
    clauses: Vec<GraphTraversalClause>,
) -> Result<(Vec<String>, Vec<GraphTraversalClause>), GraphError> {
    let start_node_codes =
        normalize_code_list(start_node_codes).map_err(|_| GraphError::InvalidTraversalQuery)?;
    if start_node_codes.is_empty() || clauses.is_empty() {
        return Err(GraphError::InvalidTraversalQuery);
    }
    let clauses = clauses
        .into_iter()
        .map(GraphTraversalClause::validate)
        .collect::<Result<Vec<_>, _>>()?;

    Ok((start_node_codes, clauses))
}

fn normalize_properties(
    properties: BTreeMap<String, GraphPropertyValue>,
) -> Result<BTreeMap<String, GraphPropertyValue>, GraphError> {
    let mut normalized = BTreeMap::new();
    for (name, value) in properties {
        normalized.insert(normalize_property_name(&name)?, value);
    }

    Ok(normalized)
}

fn normalize_property_name(value: &str) -> Result<String, GraphError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GraphError::InvalidGraphCode);
    }
    ensure_not_reserved_storage_property(value).map_err(|_| GraphError::ReservedPropertyName)?;

    Ok(value.to_owned())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn normalize_optional_graph_code(value: Option<String>) -> Result<Option<String>, GraphError> {
    match value {
        Some(value) => Ok(Some(normalize_graph_code(&value)?)),
        None => Ok(None),
    }
}

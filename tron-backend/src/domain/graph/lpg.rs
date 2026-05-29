use std::collections::{BTreeMap, BTreeSet};

use super::{ensure_not_reserved_storage_property, error::GraphError};

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub node_code: String,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, GraphPropertyValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub edge_code: String,
    pub source_node_code: String,
    pub target_node_code: String,
    pub relationship: String,
    pub properties: BTreeMap<String, GraphPropertyValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNodeWrite {
    pub node_code: String,
    pub labels: Vec<String>,
    pub properties: BTreeMap<String, GraphPropertyValue>,
}

impl GraphNodeWrite {
    pub fn validate(self) -> Result<Self, GraphError> {
        Ok(Self {
            node_code: normalize_graph_code(&self.node_code)?,
            labels: normalize_labels(self.labels)?,
            properties: normalize_properties(self.properties)?,
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
}

impl GraphEdgeWrite {
    pub fn validate(self) -> Result<Self, GraphError> {
        Ok(Self {
            edge_code: normalize_graph_code(&self.edge_code)?,
            source_node_code: normalize_graph_code(&self.source_node_code)?,
            target_node_code: normalize_graph_code(&self.target_node_code)?,
            relationship: normalize_relationship(&self.relationship)?,
            properties: normalize_properties(self.properties)?,
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
        let labels = normalize_optional_labels(self.labels)?;

        if filters.is_empty() && full_text.is_none() && vector_text.is_none() && labels.is_empty() {
            return Err(GraphError::EmptySearchQuery);
        }

        Ok(Self {
            filters,
            full_text,
            vector_text,
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
pub struct GraphTraverseQuery {
    pub start_node_codes: Vec<String>,
    pub clauses: Vec<GraphTraversalClause>,
    pub filters: Vec<GraphPropertyFilter>,
    pub limit: GraphQueryLimit,
}

impl GraphTraverseQuery {
    pub fn validate(self) -> Result<Self, GraphError> {
        let start_node_codes = normalize_code_list(self.start_node_codes)
            .map_err(|_| GraphError::InvalidTraversalQuery)?;
        if start_node_codes.is_empty() || self.clauses.is_empty() {
            return Err(GraphError::InvalidTraversalQuery);
        }

        let clauses = self
            .clauses
            .into_iter()
            .map(GraphTraversalClause::validate)
            .collect::<Result<Vec<_>, _>>()?;
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::domain::graph::{
        GraphEdgeWrite, GraphError, GraphFilterOperator, GraphNodeWrite, GraphPropertyFilter,
        GraphPropertyValue, GraphQueryLimit, GraphSearchQuery, GraphTraversalClause,
        GraphTraversalDirection, GraphTraverseQuery,
    };

    #[test]
    fn node_and_edge_write_commands_validate_codes_labels_relationships_and_properties() {
        let node = GraphNodeWrite {
            node_code: " NODE-A ".to_owned(),
            labels: vec![
                " Memory ".to_owned(),
                "Agent".to_owned(),
                "Memory".to_owned(),
            ],
            properties: BTreeMap::from([
                (
                    "displayName".to_owned(),
                    GraphPropertyValue::Text("memory node".to_owned()),
                ),
                ("priority".to_owned(), GraphPropertyValue::Integer(10)),
                ("active".to_owned(), GraphPropertyValue::Boolean(true)),
            ]),
        }
        .validate()
        .expect("valid node write should normalize");

        assert_eq!(node.node_code, "NODE-A");
        assert_eq!(node.labels, vec!["Memory", "Agent"]);
        assert_eq!(
            node.properties.get("displayName"),
            Some(&GraphPropertyValue::Text("memory node".to_owned()))
        );

        let edge = GraphEdgeWrite {
            edge_code: " EDGE-A ".to_owned(),
            source_node_code: " NODE-A ".to_owned(),
            target_node_code: " NODE-B ".to_owned(),
            relationship: " REFERENCES ".to_owned(),
            properties: BTreeMap::from([(
                "confidence".to_owned(),
                GraphPropertyValue::Decimal(0.93),
            )]),
        }
        .validate()
        .expect("valid edge write should normalize");

        assert_eq!(edge.edge_code, "EDGE-A");
        assert_eq!(edge.source_node_code, "NODE-A");
        assert_eq!(edge.target_node_code, "NODE-B");
        assert_eq!(edge.relationship, "REFERENCES");

        assert_eq!(
            GraphNodeWrite {
                node_code: "NODE-A".to_owned(),
                labels: vec![" ".to_owned()],
                properties: BTreeMap::new(),
            }
            .validate(),
            Err(GraphError::LabelRequired)
        );
        assert_eq!(
            GraphEdgeWrite {
                edge_code: "EDGE-A".to_owned(),
                source_node_code: "NODE-A".to_owned(),
                target_node_code: "NODE-B".to_owned(),
                relationship: " ".to_owned(),
                properties: BTreeMap::new(),
            }
            .validate(),
            Err(GraphError::RelationshipRequired)
        );
        assert_eq!(
            GraphNodeWrite {
                node_code: "bad code".to_owned(),
                labels: vec!["Memory".to_owned()],
                properties: BTreeMap::new(),
            }
            .validate(),
            Err(GraphError::InvalidGraphCode)
        );
        assert_eq!(
            GraphNodeWrite {
                node_code: "NODE-A".to_owned(),
                labels: vec!["Memory".to_owned()],
                properties: BTreeMap::from([(
                    "workspaceCode".to_owned(),
                    GraphPropertyValue::Text("WS-ROOT".to_owned()),
                )]),
            }
            .validate(),
            Err(GraphError::ReservedPropertyName)
        );
    }

    #[test]
    fn search_and_traversal_contracts_are_structured_and_typed() {
        let query = GraphSearchQuery {
            filters: vec![GraphPropertyFilter {
                property_name: "kind".to_owned(),
                operator: GraphFilterOperator::Equals,
                value: GraphPropertyValue::Text("decision".to_owned()),
            }],
            full_text: Some("  release checklist ".to_owned()),
            vector_text: Some("semantic description of release risk".to_owned()),
            labels: vec!["Memory".to_owned(), "Decision".to_owned()],
            limit: GraphQueryLimit::new(25).expect("positive limit should be valid"),
            min_score: Some(0.72),
        }
        .validate()
        .expect("hybrid search query should validate");

        assert_eq!(query.full_text.as_deref(), Some("release checklist"));
        assert_eq!(
            query.vector_text.as_deref(),
            Some("semantic description of release risk")
        );
        assert_eq!(query.labels, vec!["Memory", "Decision"]);
        assert_eq!(query.limit.value(), 25);
        assert_eq!(query.min_score, Some(0.72));

        assert_eq!(
            GraphSearchQuery::empty().validate(),
            Err(GraphError::EmptySearchQuery)
        );

        let traversal = GraphTraverseQuery {
            start_node_codes: vec![" NODE-A ".to_owned()],
            clauses: vec![GraphTraversalClause {
                direction: GraphTraversalDirection::Outgoing,
                relationships: vec!["DEPENDS_ON".to_owned(), "REFERENCES".to_owned()],
                min_depth: 1,
                max_depth: 3,
            }],
            filters: vec![GraphPropertyFilter {
                property_name: "active".to_owned(),
                operator: GraphFilterOperator::Equals,
                value: GraphPropertyValue::Boolean(true),
            }],
            limit: GraphQueryLimit::new(50).expect("positive limit should be valid"),
        }
        .validate()
        .expect("traversal should validate");

        assert_eq!(traversal.start_node_codes, vec!["NODE-A"]);
        assert_eq!(traversal.clauses[0].min_depth, 1);
        assert_eq!(traversal.clauses[0].max_depth, 3);
        assert_eq!(
            traversal.clauses[0].relationships,
            vec!["DEPENDS_ON", "REFERENCES"]
        );

        assert_eq!(
            GraphTraverseQuery {
                start_node_codes: vec![],
                clauses: vec![GraphTraversalClause {
                    direction: GraphTraversalDirection::Any,
                    relationships: vec![],
                    min_depth: 0,
                    max_depth: 0,
                }],
                filters: vec![],
                limit: GraphQueryLimit::new(10).expect("positive limit should be valid"),
            }
            .validate(),
            Err(GraphError::InvalidTraversalQuery)
        );
    }
}

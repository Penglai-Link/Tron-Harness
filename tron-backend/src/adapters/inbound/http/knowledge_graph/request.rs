use serde::Deserialize;

use crate::application::graph::{
    GraphExplorerExpandCommand, GraphExplorerInheritanceMode, GraphExplorerQueryCommand,
    GraphExplorerScopeMode, default_scope_modes,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KnowledgeGraphNamespaceQuery {
    pub workspace_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KnowledgeGraphSnapshotQuery {
    pub namespace: String,
    pub workspace_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphQueryRequest {
    pub workspace_code: String,
    pub namespace: String,
    pub scope_modes: Option<Vec<String>>,
    pub inheritance_mode: Option<String>,
    pub query: Option<String>,
    pub labels: Option<Vec<String>>,
    pub relationship: Option<String>,
    pub relationships: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphExpandRequest {
    pub workspace_code: String,
    pub namespace: String,
    pub scope_modes: Option<Vec<String>>,
    pub inheritance_mode: Option<String>,
    pub node_code: String,
    pub depth: Option<usize>,
    pub relationships: Option<Vec<String>>,
    pub limit: Option<usize>,
}

impl KnowledgeGraphQueryRequest {
    pub fn into_command(
        self,
        user_code: String,
    ) -> Result<GraphExplorerQueryCommand, KnowledgeGraphRequestError> {
        Ok(GraphExplorerQueryCommand {
            workspace_code: required(self.workspace_code)?,
            namespace: required(self.namespace)?,
            scope_modes: parse_scope_modes(self.scope_modes)?,
            inheritance_mode: parse_inheritance_mode(self.inheritance_mode)?,
            query: self
                .query
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty()),
            labels: self.labels.unwrap_or_default(),
            relationships: relationship_filters(self.relationships, self.relationship),
            user_code,
        })
    }
}

impl KnowledgeGraphExpandRequest {
    pub fn into_command(
        self,
        user_code: String,
    ) -> Result<GraphExplorerExpandCommand, KnowledgeGraphRequestError> {
        let limit = normalize_limit(self.limit.unwrap_or(100))?;
        let depth = self.depth.unwrap_or(1);
        if depth == 0 {
            return Err(KnowledgeGraphRequestError);
        }
        Ok(GraphExplorerExpandCommand {
            workspace_code: required(self.workspace_code)?,
            namespace: required(self.namespace)?,
            scope_modes: parse_scope_modes(self.scope_modes)?,
            inheritance_mode: parse_inheritance_mode(self.inheritance_mode)?,
            node_code: required(self.node_code)?,
            depth,
            relationships: self.relationships.unwrap_or_default(),
            limit,
            user_code,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KnowledgeGraphRequestError;

fn required(value: String) -> Result<String, KnowledgeGraphRequestError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(KnowledgeGraphRequestError);
    }
    Ok(value.to_owned())
}

fn normalize_limit(limit: usize) -> Result<usize, KnowledgeGraphRequestError> {
    if limit == 0 {
        return Err(KnowledgeGraphRequestError);
    }
    Ok(limit.min(500))
}

fn parse_scope_modes(
    values: Option<Vec<String>>,
) -> Result<Vec<GraphExplorerScopeMode>, KnowledgeGraphRequestError> {
    let Some(values) = values else {
        return Ok(default_scope_modes());
    };
    if values.is_empty() {
        return Err(KnowledgeGraphRequestError);
    }
    let mut modes = Vec::new();
    for value in values {
        let mode = match value.as_str() {
            "SPACE" => GraphExplorerScopeMode::Space,
            "USER" => GraphExplorerScopeMode::User,
            _ => return Err(KnowledgeGraphRequestError),
        };
        if !modes.contains(&mode) {
            modes.push(mode);
        }
    }
    Ok(modes)
}

fn parse_inheritance_mode(
    value: Option<String>,
) -> Result<GraphExplorerInheritanceMode, KnowledgeGraphRequestError> {
    match value.as_deref().unwrap_or("WORKSPACE_LINEAGE") {
        "WORKSPACE_LINEAGE" => Ok(GraphExplorerInheritanceMode::WorkspaceLineage),
        _ => Err(KnowledgeGraphRequestError),
    }
}

fn relationship_filters(
    relationships: Option<Vec<String>>,
    relationship: Option<String>,
) -> Vec<String> {
    let mut filters = Vec::new();
    let has_relationships_field = relationships.is_some();

    if let Some(relationships) = relationships {
        for relationship in relationships {
            push_relationship_filter(&mut filters, relationship);
        }
    }

    if !has_relationships_field && filters.is_empty() {
        if let Some(relationship) = relationship {
            push_relationship_filter(&mut filters, relationship);
        }
    }

    filters
}

fn push_relationship_filter(filters: &mut Vec<String>, value: String) {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("all") {
        return;
    }
    if !filters.iter().any(|filter| filter == value) {
        filters.push(value.to_owned());
    }
}

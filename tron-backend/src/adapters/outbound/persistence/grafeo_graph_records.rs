use std::collections::{BTreeMap, HashMap};

use grafeo::{Session, Value};

use crate::domain::graph::{
    GraphEdge, GraphEdgeWrite, GraphError, GraphNode, GraphNodeWrite, GraphPropertyValue,
    ResolvedGraphScope,
};

const NODE_QUERY: &str = "MATCH (n:TronGraphNode) RETURN \
    n.nodeCode, n.recordLabels, n.businessProps, n.fullText, n.embeddingText, \
    n.createdAt, n.updatedAt, n.namespace, n.workspaceCode, n.appCode, n.userCode";
const EDGE_QUERY: &str = "MATCH (source:TronGraphNode)-[e:TRON_EDGE]->(target:TronGraphNode) RETURN \
    e.edgeCode, e.sourceNodeCode, e.targetNodeCode, e.relationship, e.businessProps, \
    e.fullText, e.embeddingText, e.createdAt, e.updatedAt, e.namespace, e.workspaceCode, \
    e.appCode, e.userCode, source.workspaceCode, target.workspaceCode";

#[derive(Clone)]
pub(super) struct StoredGraphNodeRecord {
    pub node: GraphNode,
    pub namespace: String,
    pub workspace_code: String,
    pub app_code: String,
    pub user_code: String,
}

#[derive(Clone)]
pub(super) struct StoredGraphEdgeRecord {
    pub edge: GraphEdge,
    pub namespace: String,
    pub workspace_code: String,
    pub app_code: String,
    pub user_code: String,
    pub source_workspace_code: String,
    pub target_workspace_code: String,
}

pub(super) trait ScopedRecord {
    fn namespace(&self) -> &str;
    fn workspace_code(&self) -> &str;
    fn app_code(&self) -> &str;
    fn user_code(&self) -> &str;
}

impl ScopedRecord for StoredGraphNodeRecord {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    fn workspace_code(&self) -> &str {
        &self.workspace_code
    }

    fn app_code(&self) -> &str {
        &self.app_code
    }

    fn user_code(&self) -> &str {
        &self.user_code
    }
}

impl ScopedRecord for StoredGraphEdgeRecord {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    fn workspace_code(&self) -> &str {
        &self.workspace_code
    }

    fn app_code(&self) -> &str {
        &self.app_code
    }

    fn user_code(&self) -> &str {
        &self.user_code
    }
}

pub(super) fn load_nodes(session: &Session) -> Result<Vec<StoredGraphNodeRecord>, GraphError> {
    let result = session.execute(NODE_QUERY).map_err(storage_error)?;
    result
        .rows()
        .iter()
        .map(|row| parse_node_row(row))
        .collect::<Result<Vec<_>, _>>()
}

pub(super) fn load_edges(session: &Session) -> Result<Vec<StoredGraphEdgeRecord>, GraphError> {
    let result = session.execute(EDGE_QUERY).map_err(storage_error)?;
    result
        .rows()
        .iter()
        .map(|row| parse_edge_row(row))
        .collect::<Result<Vec<_>, _>>()
}

pub(super) fn find_exact_node(
    session: &Session,
    scope: &ResolvedGraphScope,
    node_code: &str,
) -> Result<Option<StoredGraphNodeRecord>, GraphError> {
    Ok(load_nodes(session)?
        .into_iter()
        .find(|record| record.node.node_code == node_code && exact_scope_match(record, scope)))
}

pub(super) fn find_exact_edge(
    session: &Session,
    scope: &ResolvedGraphScope,
    edge_code: &str,
) -> Result<Option<StoredGraphEdgeRecord>, GraphError> {
    Ok(load_edges(session)?
        .into_iter()
        .find(|record| record.edge.edge_code == edge_code && exact_scope_match(record, scope)))
}

pub(super) fn base_scope_params(scope: &ResolvedGraphScope) -> HashMap<String, Value> {
    HashMap::from([
        (
            "namespace".into(),
            Value::from(scope.storage_dimensions.namespace.clone()),
        ),
        (
            "workspaceCode".into(),
            Value::from(scope.storage_dimensions.workspace_code.clone()),
        ),
        (
            "appCode".into(),
            Value::from(scope.storage_dimensions.app_code.clone()),
        ),
        (
            "userCode".into(),
            Value::from(scope.storage_dimensions.user_code.clone()),
        ),
    ])
}

pub(super) fn build_node_params(
    scope: &ResolvedGraphScope,
    command: &GraphNodeWrite,
    created_at: &str,
    updated_at: &str,
) -> HashMap<String, Value> {
    let mut params = base_scope_params(scope);
    params.insert("nodeCode".into(), Value::from(command.node_code.clone()));
    params.insert(
        "recordLabels".into(),
        Value::List(
            command
                .labels
                .iter()
                .map(|label| Value::from(label.clone()))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    params.insert(
        "businessProps".into(),
        graph_properties_to_value(&command.properties),
    );
    params.insert(
        "fullText".into(),
        command
            .full_text
            .clone()
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    params.insert(
        "embeddingText".into(),
        command
            .embedding_text
            .clone()
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    params.insert("createdAt".into(), Value::from(created_at.to_owned()));
    params.insert("updatedAt".into(), Value::from(updated_at.to_owned()));
    params
}

pub(super) fn build_edge_params(
    scope: &ResolvedGraphScope,
    command: &GraphEdgeWrite,
    source_workspace_code: &str,
    target_workspace_code: &str,
    created_at: &str,
    updated_at: &str,
) -> HashMap<String, Value> {
    let mut params = base_scope_params(scope);
    params.insert("edgeCode".into(), Value::from(command.edge_code.clone()));
    params.insert(
        "sourceNodeCode".into(),
        Value::from(command.source_node_code.clone()),
    );
    params.insert(
        "targetNodeCode".into(),
        Value::from(command.target_node_code.clone()),
    );
    params.insert(
        "sourceWorkspaceCode".into(),
        Value::from(source_workspace_code.to_owned()),
    );
    params.insert(
        "targetWorkspaceCode".into(),
        Value::from(target_workspace_code.to_owned()),
    );
    params.insert(
        "relationship".into(),
        Value::from(command.relationship.clone()),
    );
    params.insert(
        "businessProps".into(),
        graph_properties_to_value(&command.properties),
    );
    params.insert(
        "fullText".into(),
        command
            .full_text
            .clone()
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    params.insert(
        "embeddingText".into(),
        command
            .embedding_text
            .clone()
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    params.insert("createdAt".into(), Value::from(created_at.to_owned()));
    params.insert("updatedAt".into(), Value::from(updated_at.to_owned()));
    params
}

pub(super) fn exact_scope_match(record: &impl ScopedRecord, scope: &ResolvedGraphScope) -> bool {
    record.namespace() == scope.storage_dimensions.namespace
        && record.workspace_code() == scope.storage_dimensions.workspace_code
        && record.app_code() == scope.storage_dimensions.app_code
        && record.user_code() == scope.storage_dimensions.user_code
}

pub(super) fn visible_scope_match(record: &impl ScopedRecord, scope: &ResolvedGraphScope) -> bool {
    record.namespace() == scope.storage_dimensions.namespace
        && record.app_code() == scope.storage_dimensions.app_code
        && record.user_code() == scope.storage_dimensions.user_code
        && scope
            .visible_workspace_codes
            .contains(&record.workspace_code().to_owned())
}

pub(super) fn storage_error(_error: grafeo::Error) -> GraphError {
    GraphError::StorageFailure
}

pub(super) fn graph_properties_to_value(
    properties: &BTreeMap<String, GraphPropertyValue>,
) -> Value {
    let mut mapped = BTreeMap::new();
    for (key, value) in properties {
        mapped.insert(
            key.clone().into(),
            match value {
                GraphPropertyValue::Text(value) => Value::from(value.clone()),
                GraphPropertyValue::Integer(value) => Value::from(*value),
                GraphPropertyValue::Decimal(value) => Value::from(*value),
                GraphPropertyValue::Boolean(value) => Value::from(*value),
            },
        );
    }

    Value::Map(mapped.into())
}

pub(super) fn graph_properties_from_value(
    value: &Value,
) -> Result<BTreeMap<String, GraphPropertyValue>, GraphError> {
    match value {
        Value::Map(values) => values
            .iter()
            .map(|(key, value)| {
                Ok((
                    key.to_string(),
                    match value {
                        Value::String(text) => GraphPropertyValue::Text(text.to_string()),
                        Value::Int64(number) => GraphPropertyValue::Integer(*number),
                        Value::Float64(number) => GraphPropertyValue::Decimal(*number),
                        Value::Bool(boolean) => GraphPropertyValue::Boolean(*boolean),
                        _ => return Err(GraphError::StorageFailure),
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, _>>(),
        Value::Null => Ok(BTreeMap::new()),
        _ => Err(GraphError::StorageFailure),
    }
}

fn parse_node_row(row: &[Value]) -> Result<StoredGraphNodeRecord, GraphError> {
    Ok(StoredGraphNodeRecord {
        node: GraphNode {
            node_code: required_string(&row[0])?,
            labels: string_list(&row[1])?,
            properties: graph_properties_from_value(&row[2])?,
            full_text: optional_string(&row[3]),
            embedding_text: optional_string(&row[4]),
            created_at: required_string(&row[5])?,
            updated_at: required_string(&row[6])?,
        },
        namespace: required_string(&row[7])?,
        workspace_code: required_string(&row[8])?,
        app_code: required_string(&row[9])?,
        user_code: required_string(&row[10])?,
    })
}

fn parse_edge_row(row: &[Value]) -> Result<StoredGraphEdgeRecord, GraphError> {
    Ok(StoredGraphEdgeRecord {
        edge: GraphEdge {
            edge_code: required_string(&row[0])?,
            source_node_code: required_string(&row[1])?,
            target_node_code: required_string(&row[2])?,
            relationship: required_string(&row[3])?,
            properties: graph_properties_from_value(&row[4])?,
            full_text: optional_string(&row[5]),
            embedding_text: optional_string(&row[6]),
            created_at: required_string(&row[7])?,
            updated_at: required_string(&row[8])?,
        },
        namespace: required_string(&row[9])?,
        workspace_code: required_string(&row[10])?,
        app_code: required_string(&row[11])?,
        user_code: required_string(&row[12])?,
        source_workspace_code: required_string(&row[13])?,
        target_workspace_code: required_string(&row[14])?,
    })
}

fn required_string(value: &Value) -> Result<String, GraphError> {
    value
        .as_str()
        .map(|value| value.to_owned())
        .ok_or(GraphError::StorageFailure)
}

fn optional_string(value: &Value) -> Option<String> {
    value.as_str().map(|text| text.to_owned())
}

fn string_list(value: &Value) -> Result<Vec<String>, GraphError> {
    match value {
        Value::List(values) => values
            .iter()
            .map(required_string)
            .collect::<Result<Vec<_>, _>>(),
        Value::Null => Ok(Vec::new()),
        _ => Err(GraphError::StorageFailure),
    }
}

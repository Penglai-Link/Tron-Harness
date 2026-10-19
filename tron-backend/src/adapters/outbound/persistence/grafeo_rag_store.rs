//! Real adjacency storage; no all-graph load or SQLite relationship mirror.
use super::grafeo_core::GrafeoStoreCore;
use crate::{
    application::rag::graph::{
        GraphAdjacencyPage, GraphAdjacencyQuery, INTERNAL_RAG_GRAPH_NAMESPACE, RagGraphStore,
    },
    domain::{
        graph::{GraphStorageDimensions, ResolvedGraphScope},
        rag::graph::*,
    },
};
use grafeo::{NodeId, Session, Value};
use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct GrafeoRagStore {
    core: GrafeoStoreCore,
    writes: Arc<Mutex<()>>,
}
impl GrafeoRagStore {
    pub fn new(core: GrafeoStoreCore) -> Self {
        Self {
            writes: core.rag_write_gate(),
            core,
        }
    }
    fn session(&self, partition: &GraphPartition) -> Result<Session, String> {
        let key = partition.key();
        let namespace = INTERNAL_RAG_GRAPH_NAMESPACE.to_owned();
        let scope = ResolvedGraphScope {
            schema: namespace.clone(),
            named_graph: key.clone(),
            root_workspace_code: key.clone(),
            current_workspace_code: key.clone(),
            visible_workspace_codes: vec![key.clone()],
            storage_dimensions: GraphStorageDimensions {
                namespace,
                workspace_code: key,
                app_code: "__NONE__".into(),
                user_code: "__NONE__".into(),
            },
        };
        self.core.session_for_scope(&scope).map_err(err)
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("RAG graph storage: {e}")
}
fn params(items: &[(&str, &str)]) -> HashMap<String, Value> {
    items
        .iter()
        .map(|(k, v)| ((*k).into(), Value::from((*v).to_owned())))
        .collect()
}
fn strings(items: impl IntoIterator<Item = String>) -> Value {
    Value::List(
        items
            .into_iter()
            .map(Value::from)
            .collect::<Vec<_>>()
            .into(),
    )
}
fn read_json<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_str(value.as_str().ok_or_else(|| err("missing fact payload"))?).map_err(err)
}
/// Resolve identities once per generation. Never form a Cartesian product of
/// all graph nodes to locate each relationship's two endpoints.
fn stage_nodes(
    session: &Session,
    nodes: &[RagGraphNode],
) -> Result<HashMap<String, NodeId>, String> {
    let mut known = HashMap::new();
    if !nodes.is_empty() {
        let found = session
            .execute_with_params(
                "MATCH (n:TronRagNode) WHERE n.nodeId IN $ids RETURN n.nodeId, n.payload, id(n)",
                HashMap::from([(
                    "ids".into(),
                    strings(nodes.iter().map(|node| node.id.clone())),
                )]),
            )
            .map_err(err)?;
        for row in found.rows() {
            let id = row[0]
                .as_str()
                .ok_or_else(|| err("missing node identity"))?
                .to_owned();
            let payload = row[1]
                .as_str()
                .ok_or_else(|| err("missing node fact"))?
                .to_owned();
            let native = row[2]
                .as_int64()
                .and_then(|id| u64::try_from(id).ok())
                .ok_or_else(|| err("invalid native node identity"))?;
            if known.insert(id, (payload, NodeId::new(native))).is_some() {
                return Err(err("duplicate graph node identity"));
            }
        }
    }
    let mut identities = HashMap::new();
    for node in nodes {
        let payload = serde_json::to_string(node).map_err(err)?;
        let native = if let Some((existing, native)) = known.get(&node.id) {
            if existing != &payload {
                return Err(err("immutable graph fact changed for an existing identity"));
            }
            *native
        } else {
            let native = session
                .create_node_with_props(
                    &["TronRagNode"],
                    [
                        ("nodeId", Value::from(node.id.clone())),
                        ("nodeGeneration", Value::from(node.generation_id.clone())),
                        ("payload", Value::from(payload.clone())),
                    ],
                )
                .map_err(err)?;
            known.insert(node.id.clone(), (payload, native));
            native
        };
        identities.insert(node.id.clone(), native);
    }
    Ok(identities)
}
fn stage_edges(
    session: &Session,
    edges: &[RagGraphEdge],
    identities: &HashMap<String, NodeId>,
) -> Result<(), String> {
    if edges.is_empty() {
        return Ok(());
    }
    let found=session.execute_with_params(
        "MATCH (a:TronRagNode)-[e:TRON_RAG_EDGE]->(b:TronRagNode) WHERE e.edgeId IN $ids RETURN e.edgeId,e.payload",
        HashMap::from([("ids".into(),strings(edges.iter().map(|edge|edge.id.clone())))]),
    ).map_err(err)?;
    let mut known = HashMap::new();
    for row in found.rows() {
        let id = row[0]
            .as_str()
            .ok_or_else(|| err("missing relationship identity"))?
            .to_owned();
        let payload = row[1]
            .as_str()
            .ok_or_else(|| err("missing relationship fact"))?
            .to_owned();
        if known.insert(id, payload).is_some() {
            return Err(err("duplicate relationship support identity"));
        }
    }
    for edge in edges {
        let payload = serde_json::to_string(edge).map_err(err)?;
        if let Some(existing) = known.get(&edge.id) {
            if existing != &payload {
                return Err(err("immutable relationship support changed"));
            }
            continue;
        }
        let from = *identities
            .get(&edge.from)
            .ok_or_else(|| err("relationship source node is absent"))?;
        let to = *identities
            .get(&edge.to)
            .ok_or_else(|| err("relationship target node is absent"))?;
        // Native creation participates in this Session's active named-graph
        // transaction and WAL. Commit/rollback and durable flush remain shared.
        session
            .create_edge_with_props(
                from,
                to,
                "TRON_RAG_EDGE",
                [
                    ("edgeId", Value::from(edge.id.clone())),
                    (
                        "sourceGeneration",
                        Value::from(edge.support.generation_id.clone()),
                    ),
                    ("relationship", Value::from(edge.relationship.clone())),
                    ("structural", Value::from(edge.structural)),
                    ("payload", Value::from(payload.clone())),
                ],
            )
            .map_err(err)?;
        known.insert(edge.id.clone(), payload);
    }
    Ok(())
}
impl RagGraphStore for GrafeoRagStore {
    fn stage_generation(&self, generation: &RagGraphGeneration) -> Result<(), String> {
        let _guard = self.writes.lock().map_err(err)?;
        let mut session = self.session(&generation.partition)?;
        session.begin_transaction().map_err(err)?;
        let staged = (|| {
            let identities = stage_nodes(&session, &generation.nodes)?;
            stage_edges(&session, &generation.edges, &identities)?;
            session.commit().map_err(err)?;
            // SQLite may activate only after both transaction commit and disk flush succeed.
            self.core.flush_durable().map_err(err)
        })();
        if staged.is_err() {
            let _ = session.rollback();
        }
        staged
    }
    fn nodes(
        &self,
        partition: &GraphPartition,
        ids: &[String],
        generations: &BTreeSet<String>,
    ) -> Result<Vec<RagGraphNode>, String> {
        if ids.is_empty() || generations.is_empty() {
            return Ok(vec![]);
        }
        let session = self.session(partition)?;
        let p = HashMap::from([
            ("ids".into(), strings(ids.iter().cloned())),
            ("generations".into(), strings(generations.iter().cloned())),
        ]);
        let mut nodes = std::collections::BTreeMap::<String, RagGraphNode>::new();
        let found = session.execute_with_params(
            "MATCH (n:TronRagNode) WHERE n.nodeId IN $ids AND n.nodeGeneration IN $generations RETURN n.payload ORDER BY n.nodeId", p.clone()).map_err(err)?;
        for row in found.rows() {
            let node: RagGraphNode = read_json(&row[0])?;
            nodes.insert(node.id.clone(), node);
        }
        // Shared identities are visible only when an authorized source supports
        // an incident relationship. Batch both directions instead of N+1 reads.
        // Keep node predicates inside the pattern: Grafeo otherwise expands
        // every node before applying a combined node/edge WHERE predicate.
        for pattern in [
            "(n:TronRagNode WHERE n.nodeId IN $ids AND n.nodeGeneration = '')-[e:TRON_RAG_EDGE]->(other:TronRagNode)",
            "(n:TronRagNode WHERE n.nodeId IN $ids AND n.nodeGeneration = '')<-[e:TRON_RAG_EDGE]-(other:TronRagNode)",
        ] {
            let found = session.execute_with_params(&format!(
                "MATCH {pattern} WHERE e.sourceGeneration IN $generations RETURN n.payload ORDER BY n.nodeId"), p.clone()).map_err(err)?;
            for row in found.rows() {
                let node: RagGraphNode = read_json(&row[0])?;
                nodes.insert(node.id.clone(), node);
            }
        }
        Ok(nodes.into_values().collect())
    }
    fn adjacent(&self, query: &GraphAdjacencyQuery) -> Result<GraphAdjacencyPage, String> {
        if query.generations.is_empty() {
            return Ok(Default::default());
        }
        if !(1..=1000).contains(&query.limit) {
            return Err(err("invalid adjacency budget"));
        }
        let session = self.session(&query.partition)?;
        let pattern = match query.direction {
            GraphDirection::Incoming => {
                "(n:TronRagNode {nodeId: $node})<-[e:TRON_RAG_EDGE]-(other:TronRagNode)"
            }
            GraphDirection::Outgoing => {
                "(n:TronRagNode {nodeId: $node})-[e:TRON_RAG_EDGE]->(other:TronRagNode)"
            }
        };
        let mut p = params(&[
            ("node", &query.node_id),
            ("after", query.after_edge_id.as_deref().unwrap_or("")),
        ]);
        p.insert(
            "generations".into(),
            strings(query.generations.iter().cloned()),
        );
        p.insert(
            "types".into(),
            strings(query.relationship_types.iter().cloned()),
        );
        let structural = if query.include_structural {
            ""
        } else {
            " AND e.structural = false"
        };
        let types = if query.relationship_types.is_empty() {
            ""
        } else {
            " AND e.relationship IN $types"
        };
        let found = session.execute_with_params(&format!(
            "MATCH {pattern} WHERE e.sourceGeneration IN $generations AND e.edgeId > $after{structural}{types} \
             RETURN e.payload ORDER BY e.edgeId LIMIT {}", query.limit + 1), p).map_err(err)?;
        let mut edges = found
            .rows()
            .iter()
            .map(|row| read_json(&row[0]))
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = edges.len() > query.limit;
        edges.truncate(query.limit);
        Ok(GraphAdjacencyPage { edges, has_more })
    }
    fn structural_adjacent(
        &self,
        query: &crate::application::rag::graph::GraphStructuralQuery,
    ) -> Result<GraphAdjacencyPage, String> {
        query.validate()?;
        if query.node_ids.is_empty() || query.generations.is_empty() {
            return Ok(Default::default());
        }
        let session = self.session(&query.partition)?;
        let pattern = match query.direction {
            GraphDirection::Incoming => {
                "(n:TronRagNode WHERE n.nodeId IN $ids)<-[e:TRON_RAG_EDGE]-(other:TronRagNode)"
            }
            GraphDirection::Outgoing => {
                "(n:TronRagNode WHERE n.nodeId IN $ids)-[e:TRON_RAG_EDGE]->(other:TronRagNode)"
            }
        };
        let mut p = params(&[("after", query.after_edge_id.as_deref().unwrap_or(""))]);
        p.insert("ids".into(), strings(query.node_ids.iter().cloned()));
        p.insert(
            "generations".into(),
            strings(query.generations.iter().cloned()),
        );
        p.insert("types".into(), strings(query.relationships.iter().cloned()));
        let found = session
            .execute_with_params(
                &format!(
                    "MATCH {pattern} WHERE e.sourceGeneration IN $generations \
             AND e.structural = true AND e.relationship IN $types AND e.edgeId > $after \
             RETURN e.payload ORDER BY e.edgeId LIMIT {}",
                    query.limit + 1
                ),
                p,
            )
            .map_err(err)?;
        let mut edges = found
            .rows()
            .iter()
            .map(|row| read_json(&row[0]))
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = edges.len() > query.limit;
        edges.truncate(query.limit);
        Ok(GraphAdjacencyPage { edges, has_more })
    }
    fn entity_evidence(
        &self,
        partition: &GraphPartition,
        ids: &[String],
        generations: &BTreeSet<String>,
        after: Option<&str>,
        limit: usize,
    ) -> Result<GraphAdjacencyPage, String> {
        if !(1..=1000).contains(&limit) {
            return Err(err("invalid provenance budget"));
        }
        if ids.is_empty() || generations.is_empty() {
            return Ok(Default::default());
        }
        let session = self.session(partition)?;
        let mut p = params(&[("after", after.unwrap_or(""))]);
        p.insert("ids".into(), strings(ids.iter().cloned()));
        p.insert("generations".into(), strings(generations.iter().cloned()));
        let found = session
            .execute_with_params(
                &format!(
                    "MATCH (b:TronRagNode WHERE b.nodeId IN $ids)<-[e:TRON_RAG_EDGE]-(a:TronRagNode) \
             WHERE e.sourceGeneration IN $generations AND e.edgeId > $after \
             AND e.relationship IN ['MENTIONS','DESCRIBES'] AND e.structural = true \
             RETURN e.payload ORDER BY e.edgeId LIMIT {}",
                    limit + 1
                ),
                p,
            )
            .map_err(err)?;
        let mut edges = found
            .rows()
            .iter()
            .map(|row| read_json(&row[0]))
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = edges.len() > limit;
        edges.truncate(limit);
        Ok(GraphAdjacencyPage { edges, has_more })
    }
    fn delete_generation(&self, partition: &GraphPartition, id: &str) -> Result<(), String> {
        let _guard = self.writes.lock().map_err(err)?;
        let mut session = self.session(partition)?;
        session.begin_transaction().map_err(err)?;
        let deleted = (|| {
            let candidates = session.execute_with_params(
                "MATCH (a:TronRagNode)-[e:TRON_RAG_EDGE]->(b:TronRagNode) WHERE e.sourceGeneration = $id RETURN a.nodeId, b.nodeId",
                params(&[("id", id)])).map_err(err)?.rows().iter()
                .flat_map(|row| row.iter().filter_map(Value::as_str).map(str::to_owned)).collect::<BTreeSet<_>>();
            session.execute_with_params("MATCH (a:TronRagNode)-[e:TRON_RAG_EDGE]->(b:TronRagNode) WHERE e.sourceGeneration = $id DELETE e",
                params(&[("id", id)])).map_err(err)?;
            session
                .execute_with_params(
                    "MATCH (n:TronRagNode) WHERE n.nodeGeneration = $id DETACH DELETE n",
                    params(&[("id", id)]),
                )
                .map_err(err)?;
            if !candidates.is_empty() {
                let remaining=session.execute_with_params(
                    "MATCH (n:TronRagNode)-[e:TRON_RAG_EDGE]-(other:TronRagNode) WHERE n.nodeId IN $ids AND n.nodeGeneration = '' RETURN n.nodeId",
                    HashMap::from([("ids".into(),strings(candidates.iter().cloned()))]),
                ).map_err(err)?.rows().iter().filter_map(|row|row[0].as_str().map(str::to_owned)).collect::<BTreeSet<_>>();
                let orphaned = candidates
                    .difference(&remaining)
                    .cloned()
                    .collect::<Vec<_>>();
                if !orphaned.is_empty() {
                    session.execute_with_params(
                    "MATCH (n:TronRagNode) WHERE n.nodeId IN $ids AND n.nodeGeneration = '' DELETE n",
                    HashMap::from([("ids".into(),strings(orphaned))]),
                ).map_err(err)?;
                }
            }
            session.commit().map_err(err)?;
            self.core.flush_durable().map_err(err)
        })();
        if deleted.is_err() {
            let _ = session.rollback();
        }
        deleted
    }
}

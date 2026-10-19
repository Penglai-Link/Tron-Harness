//! Cleanup across actual Grafeo and SQLite, including a crash between both stores.
use serde_json::json;
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tron_backend::{
    adapters::outbound::persistence::sqlite::SqliteRagGraphIndexRepository,
    adapters::outbound::persistence::{GrafeoAdapterConfig, GrafeoRagStore, GrafeoStoreCore},
    application::rag::graph::*,
    domain::rag::graph::*,
};
fn partition() -> GraphPartition {
    GraphPartition {
        corpus_id: "skill".into(),
        scope_type: "USER".into(),
        scope_code: "owner".into(),
    }
}
fn stage(index: &SqliteRagGraphIndexRepository, store: &dyn RagGraphStore, id: &str) {
    let manifest = GraphGenerationManifest {
        id: id.into(),
        partition: partition(),
        logical_source_code: id.into(),
        source_code: id.into(),
        source_version: "v1".into(),
        task_id: format!("task-{id}"),
        configuration_hash: "config".into(),
        model_code: "model".into(),
        model_fingerprint: "space".into(),
        provider_model: None,
        dimension: 2,
        expected_projections: 2,
    };
    let nodes = [
        ("A", RagGraphNodeKind::Entity),
        ("B", RagGraphNodeKind::Entity),
        (&format!("chunk-{id}"), RagGraphNodeKind::Chunk),
    ]
    .into_iter()
    .map(|(name, kind)| RagGraphNode {
        id: name.into(),
        kind,
        generation_id: if kind == RagGraphNodeKind::Entity {
            String::new()
        } else {
            id.into()
        },
        name: name.into(),
        entity_type: if kind == RagGraphNodeKind::Entity {
            Some("Service".into())
        } else {
            None
        },
        properties: json!({}),
    })
    .collect();
    let support = RagGraphSupport {
        generation_id: id.into(),
        source_code: id.into(),
        source_version: "v1".into(),
        task_id: manifest.task_id.clone(),
        chunk_code: format!("chunk-{id}"),
        quote: "A calls B".into(),
        location: json!({"page":1}),
    };
    let facts = RagGraphGeneration {
        partition: partition(),
        id: id.into(),
        source_code: id.into(),
        source_version: "v1".into(),
        task_id: manifest.task_id.clone(),
        nodes,
        edges: vec![
            RagGraphEdge {
                id: format!("edge-{id}"),
                logical_relation_id: "calls".into(),
                from: "A".into(),
                to: "B".into(),
                relationship: "CALLS".into(),
                structural: false,
                support: support.clone(),
            },
            RagGraphEdge {
                id: format!("mention-{id}"),
                logical_relation_id: format!("mention-{id}"),
                from: format!("chunk-{id}"),
                to: "A".into(),
                relationship: "MENTIONS".into(),
                structural: true,
                support,
            },
        ],
    };
    index.begin_generation(&manifest).unwrap();
    store.stage_generation(&facts).unwrap();
    index
        .stage_projections(
            &manifest,
            &["A", "B"].map(|name| PreparedGraphProjection {
                entity_id: name.into(),
                chunk_codes: vec![format!("chunk-{id}")],
                text: format!("{name} service"),
                embedding: vec![1., 0.],
            }),
        )
        .unwrap();
}
fn activate(path: &str, id: &str) {
    let c = rusqlite::Connection::open(path).unwrap();
    let tx = c.unchecked_transaction().unwrap();
    tx.execute("INSERT INTO rag_graph_active_sources(partition_key,logical_source_code,generation_id,publication_epoch,updated_at) SELECT partition_key,logical_source_code,generation_id,1,0 FROM rag_graph_generations WHERE generation_id=?1",[id]).unwrap();
    tx.execute(
        "UPDATE rag_graph_generations SET state='Active' WHERE generation_id=?1",
        [id],
    )
    .unwrap();
    tx.execute(
        "DELETE FROM rag_graph_cleanup_jobs WHERE generation_id=?1",
        [id],
    )
    .unwrap();
    tx.commit().unwrap();
}
fn retire(path: &str, id: &str) {
    let c = rusqlite::Connection::open(path).unwrap();
    let tx = c.unchecked_transaction().unwrap();
    tx.execute(
        "DELETE FROM rag_graph_active_sources WHERE generation_id=?1",
        [id],
    )
    .unwrap();
    tx.execute(
        "UPDATE rag_graph_generations SET state='Retired' WHERE generation_id=?1",
        [id],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO rag_graph_cleanup_jobs(generation_id,not_before,updated_at) VALUES (?1,0,0)",
        [id],
    )
    .unwrap();
    tx.commit().unwrap();
}
fn store(path: &str) -> Arc<GrafeoRagStore> {
    Arc::new(GrafeoRagStore::new(
        GrafeoStoreCore::connect(GrafeoAdapterConfig {
            storage_path: path.into(),
        })
        .unwrap(),
    ))
}
fn open(path: &str) -> Arc<SqliteRagGraphIndexRepository> {
    Arc::new(SqliteRagGraphIndexRepository::open(path).unwrap())
}
fn q(ids: &[&str]) -> GraphAdjacencyQuery {
    GraphAdjacencyQuery {
        partition: partition(),
        node_id: "A".into(),
        direction: GraphDirection::Outgoing,
        generations: ids.iter().map(|id| (*id).into()).collect(),
        relationship_types: BTreeSet::new(),
        include_structural: false,
        after_edge_id: None,
        limit: 100,
    }
}

#[test]
fn deleting_retired_generation_keeps_other_source_support_and_removes_both_sql_indexes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite").display().to_string();
    let graph = dir.path().join("facts.grafeo").display().to_string();
    let index = open(&path);
    let store = store(&graph);
    let infra = GraphInfrastructure::new(store.clone(), index.clone());
    for id in ["one", "two"] {
        stage(&index, store.as_ref(), id);
        activate(&path, id);
    }
    retire(&path, "one");
    let report = infra
        .cleanup_batch(chrono::Utc::now().timestamp_millis(), 4)
        .unwrap();
    assert_eq!(report.cleaned, 1);
    assert_eq!(report.failed, 0);
    assert!(index.export_generation("one").unwrap().is_none());
    assert_eq!(
        index.active_generations(&[partition()]).unwrap()[0].id,
        "two"
    );
    assert!(store.adjacent(&q(&["one"])).unwrap().edges.is_empty());
    assert_eq!(store.adjacent(&q(&["two"])).unwrap().edges.len(), 1);
    assert_eq!(
        store
            .nodes(
                &partition(),
                &["A".into(), "B".into()],
                &BTreeSet::from(["two".into()])
            )
            .unwrap()
            .len(),
        2
    );
    let counts = index.export_generation("two").unwrap().unwrap().1;
    assert_eq!(counts.len(), 2);
    retire(&path, "two");
    assert_eq!(
        infra
            .cleanup_batch(chrono::Utc::now().timestamp_millis(), 4)
            .unwrap()
            .cleaned,
        1
    );
    assert!(
        store
            .adjacent(&q(&["one", "two"]))
            .unwrap()
            .edges
            .is_empty()
    );
    assert!(
        store
            .nodes(
                &partition(),
                &[
                    "A".into(),
                    "B".into(),
                    "chunk-one".into(),
                    "chunk-two".into()
                ],
                &BTreeSet::from(["one".into(), "two".into()])
            )
            .unwrap()
            .is_empty()
    );
}

#[test]
fn reopen_after_graph_deletion_resumes_sql_cleanup_with_the_same_durable_job() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite").display().to_string();
    let graph = dir.path().join("facts.grafeo").display().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    {
        let index = open(&path);
        let store = store(&graph);
        stage(&index, store.as_ref(), "one");
        activate(&path, "one");
        retire(&path, "one");
        let claim = index.claim_cleanup(now, 1000).unwrap().unwrap();
        store
            .delete_generation(&claim.manifest.partition, &claim.manifest.id)
            .unwrap();
        assert!(
            index.export_generation("one").unwrap().is_some(),
            "simulate crash before SQLite finish"
        );
    }
    let index = open(&path);
    let store = store(&graph);
    let infra = GraphInfrastructure::new(store.clone(), index.clone());
    assert_eq!(infra.cleanup_batch(now + 999, 1).unwrap().cleaned, 0);
    assert_eq!(infra.cleanup_batch(now + 1001, 1).unwrap().cleaned, 1);
    assert!(index.export_generation("one").unwrap().is_none());
    assert!(store.adjacent(&q(&["one"])).unwrap().edges.is_empty());
}

struct FlakyStore {
    inner: Arc<GrafeoRagStore>,
    fail: AtomicBool,
}
impl RagGraphStore for FlakyStore {
    fn stage_generation(&self, g: &RagGraphGeneration) -> Result<(), String> {
        self.inner.stage_generation(g)
    }
    fn nodes(
        &self,
        p: &GraphPartition,
        ids: &[String],
        g: &BTreeSet<String>,
    ) -> Result<Vec<RagGraphNode>, String> {
        self.inner.nodes(p, ids, g)
    }
    fn adjacent(&self, q: &GraphAdjacencyQuery) -> Result<GraphAdjacencyPage, String> {
        self.inner.adjacent(q)
    }
    fn structural_adjacent(&self, q: &GraphStructuralQuery) -> Result<GraphAdjacencyPage, String> {
        self.inner.structural_adjacent(q)
    }
    fn entity_evidence(
        &self,
        p: &GraphPartition,
        ids: &[String],
        g: &BTreeSet<String>,
        after: Option<&str>,
        limit: usize,
    ) -> Result<GraphAdjacencyPage, String> {
        self.inner.entity_evidence(p, ids, g, after, limit)
    }
    fn delete_generation(&self, p: &GraphPartition, id: &str) -> Result<(), String> {
        if self.fail.load(Ordering::SeqCst) {
            Err("synthetic graph failure".into())
        } else {
            self.inner.delete_generation(p, id)
        }
    }
}
#[test]
fn failure_never_deletes_sql_projections_and_successful_retry_finishes_the_job() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite").display().to_string();
    let graph = dir.path().join("facts.grafeo").display().to_string();
    let index = open(&path);
    let native = store(&graph);
    stage(&index, native.as_ref(), "one");
    activate(&path, "one");
    retire(&path, "one");
    let flaky = Arc::new(FlakyStore {
        inner: native,
        fail: AtomicBool::new(true),
    });
    let infra = GraphInfrastructure::new(flaky.clone(), index.clone());
    let now = chrono::Utc::now().timestamp_millis();
    let report = infra.cleanup_batch(now, 4).unwrap();
    assert_eq!(report.failed, 1);
    assert_eq!(report.cleaned, 0);
    assert!(index.export_generation("one").unwrap().is_some());
    flaky.fail.store(false, Ordering::SeqCst);
    assert_eq!(infra.cleanup_batch(now + 1000, 4).unwrap().cleaned, 0);
    assert_eq!(infra.cleanup_batch(now + 2500, 4).unwrap().cleaned, 1);
    assert!(index.export_generation("one").unwrap().is_none());
}

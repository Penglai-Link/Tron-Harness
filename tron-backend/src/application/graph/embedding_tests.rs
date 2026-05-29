use crate::{
    application::graph::{
        GraphService,
        service_test_embedding_support::{
            EmbeddingCall, FakeEmbeddingRuntime, chunked_embedding_result, embedding_call_result,
        },
        service_test_support::{
            FakeScopeResolver, FakeStore, plain_prepared_search, resolved_scope, scope_command,
            search_query, semantic_node_write, vector_search_query,
        },
    },
    domain::graph::{GraphError, GraphNodeWrite},
};

#[tokio::test]
async fn vector_indexed_writes_derive_embeddings_before_store_execution() {
    let scope = resolved_scope();
    let resolver = FakeScopeResolver::new(scope.clone());
    let store = FakeStore::with_nodes(&[]);
    let runtime = FakeEmbeddingRuntime::default();
    runtime.push_single_result(Ok(embedding_call_result(
        "TEXT_EMBED_3_SMALL",
        &[0.1, 0.2, 0.3],
    )));
    runtime.push_single_result(Ok(embedding_call_result(
        "TEXT_EMBED_3_SMALL",
        &[0.9, 0.3, 0.1],
    )));
    let service = GraphService::with_embedding_runtime(
        store.clone(),
        resolver,
        std::sync::Arc::new(runtime.clone()),
    );

    service
        .create_node(scope_command(), semantic_node_write("NODE-A"))
        .await
        .expect("semantic node write should pass");
    service
        .search(scope_command(), vector_search_query())
        .await
        .expect("vector search should pass");

    assert_eq!(
        runtime.calls(),
        vec![
            EmbeddingCall::Single {
                workspace_code: scope.current_workspace_code.clone(),
                model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
                input: "semantic memory body".to_owned(),
            },
            EmbeddingCall::Single {
                workspace_code: scope.current_workspace_code.clone(),
                model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
                input: "semantic memory lookup".to_owned(),
            },
        ]
    );

    let mut expected_search = search_query();
    expected_search.vector_text = Some("semantic memory lookup".to_owned());
    expected_search.embedding_model_code = Some("TEXT_EMBED_3_SMALL".to_owned());

    assert_eq!(store.calls().len(), 2);
    match &store.calls()[0] {
        crate::application::graph::service_test_support::StoreCall::CreateNode {
            command, ..
        } => {
            assert_eq!(command.command.node_code, "NODE-A");
            assert_eq!(
                command.semantic_model_code.as_deref(),
                Some("TEXT_EMBED_3_SMALL")
            );
            assert_eq!(
                command
                    .semantic_projection
                    .as_ref()
                    .expect("projection should exist")
                    .as_slice(),
                &[0.1, 0.2, 0.3]
            );
            assert!(command.semantic_slices.is_empty());
        }
        other => panic!("unexpected first store call: {other:?}"),
    }
    match &store.calls()[1] {
        crate::application::graph::service_test_support::StoreCall::Search { query, .. } => {
            assert_eq!(
                query,
                &crate::application::graph::PreparedGraphSearchQuery {
                    semantic_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
                    semantic_projection: Some(
                        crate::application::graph::GraphSemanticProjection::from_values(vec![
                            0.9, 0.3, 0.1
                        ],),
                    ),
                    ..plain_prepared_search(expected_search)
                }
            );
        }
        other => panic!("unexpected second store call: {other:?}"),
    }
}

#[tokio::test]
async fn long_semantic_writes_use_chunked_embedding_derivation() {
    let resolver = FakeScopeResolver::new(resolved_scope());
    let store = FakeStore::with_nodes(&[]);
    let runtime = FakeEmbeddingRuntime::default();
    runtime.push_chunked_result(Ok(chunked_embedding_result(
        "TEXT_EMBED_3_SMALL",
        &[(0, "chunk-a", &[0.1, 0.4]), (1, "chunk-b", &[0.8, 0.2])],
    )));
    let service = GraphService::with_embedding_runtime(
        store.clone(),
        resolver,
        std::sync::Arc::new(runtime.clone()),
    );

    let long_text = "x".repeat(513);
    let command = GraphNodeWrite {
        embedding_text: Some(long_text.clone()),
        embedding_model_code: Some("TEXT_EMBED_3_SMALL".to_owned()),
        ..semantic_node_write("LONG-NODE")
    };

    service
        .create_node(scope_command(), command)
        .await
        .expect("chunked semantic node write should pass");

    assert_eq!(runtime.calls().len(), 1);
    match &runtime.calls()[0] {
        EmbeddingCall::Chunked {
            workspace_code,
            model_code,
            input,
            max_chars_per_chunk,
        } => {
            assert_eq!(workspace_code, "WS-CHILD");
            assert_eq!(model_code.as_deref(), Some("TEXT_EMBED_3_SMALL"));
            assert_eq!(input, &long_text);
            assert_eq!(*max_chars_per_chunk, 512);
        }
        other => panic!("unexpected embedding call: {other:?}"),
    }
    match &store.calls()[0] {
        crate::application::graph::service_test_support::StoreCall::CreateNode {
            command, ..
        } => {
            assert_eq!(
                command.semantic_model_code.as_deref(),
                Some("TEXT_EMBED_3_SMALL")
            );
            assert!(command.semantic_projection.is_none());
            assert_eq!(command.semantic_slices.len(), 2);
            assert_eq!(command.semantic_slices[0].content, "chunk-a");
            assert_eq!(command.semantic_slices[1].content, "chunk-b");
        }
        other => panic!("unexpected store call: {other:?}"),
    }
}

#[tokio::test]
async fn vector_requests_fail_without_runtime_or_provider_and_never_reach_store() {
    let resolver = FakeScopeResolver::new(resolved_scope());
    let store = FakeStore::with_nodes(&[]);
    let service = GraphService::new(store.clone(), resolver.clone());

    assert_eq!(
        service
            .create_node(scope_command(), semantic_node_write("NODE-A"))
            .await,
        Err(GraphError::EmbeddingUnavailable)
    );
    assert_eq!(
        service.search(scope_command(), vector_search_query()).await,
        Err(GraphError::EmbeddingUnavailable)
    );
    assert_eq!(store.calls(), Vec::new());

    let failing_store = FakeStore::with_nodes(&[]);
    let failing_runtime = FakeEmbeddingRuntime::default();
    failing_runtime.push_single_result(Err(GraphError::EmbeddingProviderFailure));
    let failing_service = GraphService::with_embedding_runtime(
        failing_store.clone(),
        resolver,
        std::sync::Arc::new(failing_runtime),
    );

    assert_eq!(
        failing_service
            .create_node(scope_command(), semantic_node_write("NODE-B"))
            .await,
        Err(GraphError::EmbeddingProviderFailure)
    );
    assert_eq!(failing_store.calls(), Vec::new());
}

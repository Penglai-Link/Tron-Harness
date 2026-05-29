use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{
    application::graph::{
        GraphChunkedEmbeddingCallResult, GraphEmbeddingCallResult, GraphEmbeddingRuntime,
        GraphSemanticProjection, GraphSemanticSlice,
    },
    domain::graph::GraphError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum EmbeddingCall {
    Single {
        workspace_code: String,
        model_code: Option<String>,
        input: String,
    },
    Chunked {
        workspace_code: String,
        model_code: Option<String>,
        input: String,
        max_chars_per_chunk: usize,
    },
}

#[derive(Default)]
struct FakeEmbeddingRuntimeState {
    calls: Vec<EmbeddingCall>,
    single_results: VecDeque<Result<GraphEmbeddingCallResult, GraphError>>,
    chunked_results: VecDeque<Result<GraphChunkedEmbeddingCallResult, GraphError>>,
}

#[derive(Clone, Default)]
pub(super) struct FakeEmbeddingRuntime {
    state: Arc<Mutex<FakeEmbeddingRuntimeState>>,
}

impl FakeEmbeddingRuntime {
    pub(super) fn push_single_result(&self, result: Result<GraphEmbeddingCallResult, GraphError>) {
        self.state
            .lock()
            .expect("embedding runtime state mutex poisoned")
            .single_results
            .push_back(result);
    }

    pub(super) fn push_chunked_result(
        &self,
        result: Result<GraphChunkedEmbeddingCallResult, GraphError>,
    ) {
        self.state
            .lock()
            .expect("embedding runtime state mutex poisoned")
            .chunked_results
            .push_back(result);
    }

    pub(super) fn calls(&self) -> Vec<EmbeddingCall> {
        self.state
            .lock()
            .expect("embedding runtime state mutex poisoned")
            .calls
            .clone()
    }
}

impl GraphEmbeddingRuntime for FakeEmbeddingRuntime {
    fn embedding_call<'a>(
        &'a self,
        workspace_code: &'a str,
        model_code: Option<&'a str>,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GraphEmbeddingCallResult, GraphError>> + Send + 'a>>
    {
        let workspace_code = workspace_code.to_owned();
        let model_code = model_code.map(str::to_owned);
        let input = input.to_owned();
        Box::pin(async move {
            let mut state = self
                .state
                .lock()
                .expect("embedding runtime state mutex poisoned");
            state.calls.push(EmbeddingCall::Single {
                workspace_code,
                model_code,
                input,
            });
            state
                .single_results
                .pop_front()
                .unwrap_or_else(|| Err(GraphError::EmbeddingProviderFailure))
        })
    }

    fn embedding_call_with_chunking<'a>(
        &'a self,
        workspace_code: &'a str,
        model_code: Option<&'a str>,
        input: &'a str,
        max_chars_per_chunk: usize,
    ) -> Pin<
        Box<dyn Future<Output = Result<GraphChunkedEmbeddingCallResult, GraphError>> + Send + 'a>,
    > {
        let workspace_code = workspace_code.to_owned();
        let model_code = model_code.map(str::to_owned);
        let input = input.to_owned();
        Box::pin(async move {
            let mut state = self
                .state
                .lock()
                .expect("embedding runtime state mutex poisoned");
            state.calls.push(EmbeddingCall::Chunked {
                workspace_code,
                model_code,
                input,
                max_chars_per_chunk,
            });
            state
                .chunked_results
                .pop_front()
                .unwrap_or_else(|| Err(GraphError::EmbeddingProviderFailure))
        })
    }
}

pub(super) fn embedding_call_result(
    model_code: &str,
    projection: &[f32],
) -> GraphEmbeddingCallResult {
    GraphEmbeddingCallResult {
        model_code: model_code.to_owned(),
        projection: GraphSemanticProjection::from_values(projection.to_vec()),
    }
}

pub(super) fn chunked_embedding_result(
    model_code: &str,
    slices: &[(usize, &str, &[f32])],
) -> GraphChunkedEmbeddingCallResult {
    GraphChunkedEmbeddingCallResult {
        model_code: model_code.to_owned(),
        slices: slices
            .iter()
            .map(|(chunk_index, content, projection)| GraphSemanticSlice {
                chunk_index: *chunk_index,
                content: (*content).to_owned(),
                projection: GraphSemanticProjection::from_values(projection.to_vec()),
            })
            .collect(),
    }
}

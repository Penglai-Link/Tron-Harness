use super::*;
use crate::adapters::outbound::persistence::sqlite::stored_contexts;
use crate::application::{
    document::*,
    rag::{RagEvidence, RagSearchRow},
};

fn hit(chunk: &LocatedChunk) -> RagEvidence {
    RagEvidence {
        context_text: None,
        recall_ranks: Default::default(),
        bundle: None,
        selected_blocks: Default::default(),
        row: RagSearchRow {
            corpus_id: "kb".into(),
            source_code: "source".into(),
            source_version: "v1".into(),
            target_code: format!("chunk-{}", chunk.ordinal),
            scope_type: "USER".into(),
            scope_code: "owner".into(),
            title: None,
            text: chunk.text.clone(),
            search_text: chunk.index_text.clone(),
            heading_path: chunk.heading_path.clone(),
            ordinal: chunk.ordinal as i64,
            token_count: 1,
            embedding: None,
            storage_score: None,
            source_metadata: json!({}),
            field_values: Default::default(),
            text_metadata: json!({"contextId":chunk.context_id,"sourceSpans":chunk.source_spans,"sourceLocations":chunk.source_locations,"location":chunk.location}),
        },
        channels: Default::default(),
        score: 1.,
    }
}
fn plan() -> (ChunkedDocument, String) {
    let body = "完整的治疗建议。".repeat(50);
    let document = ParsedDocument {
        format: "pdf".into(),
        warnings: vec![],
        image_analysis: None,
        sections: vec![
            LocatedSection {
                text: body.clone(),
                heading_level: None,
                location: DocumentLocation {
                    source_block_id: Some("body-original".into()),
                    page: Some(1),
                    char_end: body.chars().count(),
                    ..Default::default()
                },
            },
            LocatedSection {
                text: "无关的期刊公告".into(),
                heading_level: None,
                location: DocumentLocation {
                    source_block_id: Some("notice-original".into()),
                    page: Some(2),
                    char_start: body.chars().count() + 1,
                    char_end: body.chars().count() + 9,
                    ..Default::default()
                },
            },
        ],
    };
    let plan = split_document_with_strategies(
        &document,
        &ChunkConfig {
            max_chars: 200,
            overlap_chars: 20,
            ..Default::default()
        },
        &eliteforge_agent_capability_chunking::built_in_chunk_strategies(),
        "doc",
        "manual.pdf",
    )
    .unwrap();
    assert_eq!(
        plan.contexts.len(),
        1,
        "the test must reproduce a mixed packing group"
    );
    (plan, body)
}
fn contexts(plan: &ChunkedDocument) -> BTreeMap<ContextKey, StoredChunkContext> {
    stored_contexts(&plan.contexts, &plan.chunks)
        .into_iter()
        .map(|c| (("kb".into(), "source".into(), c.id.clone()), c))
        .collect()
}

#[test]
fn context_expansion_restores_original_paragraph_without_unrelated_notice_or_page() {
    let (plan, body) = plan();
    let mut result = RagEvidenceResult {
        hits: vec![hit(&plan.chunks[0])],
        ..Default::default()
    };
    let leaf = result.hits[0].row.text.clone();
    expand_result(&mut result, &contexts(&plan));
    let row = &result.hits[0].row;
    assert_eq!(result.hits[0].text(), body);
    assert_eq!(
        row.text, leaf,
        "publication checks retain the exact matched text"
    );
    assert!(!result.hits[0].text().contains("期刊公告"));
    assert_eq!(row.text_metadata["matchedText"], leaf);
    assert_eq!(row.text_metadata["location"]["pages"], json!([1]));
    assert!(
        row.text_metadata["contextLocations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|l| l["sourceBlockId"] == "body-original")
    );
    assert!(
        serde_json::from_value::<Vec<ChunkSourceSpan>>(row.text_metadata["contextSpans"].clone())
            .unwrap()
            .iter()
            .all(|s| s.block_id == "block-0")
    );
    let original_metadata = row.text_metadata.clone();
    expand_result(&mut result, &contexts(&plan));
    assert_eq!(
        result.hits[0].row.text_metadata, original_metadata,
        "repeated assembly preserves the original matched leaf and citation"
    );
}

#[test]
fn context_deduplication_keeps_distinct_original_blocks_in_one_packing_group() {
    let (plan, body) = plan();
    let mut result = RagEvidenceResult {
        hits: vec![hit(&plan.chunks[0]), hit(plan.chunks.last().unwrap())],
        ..Default::default()
    };
    expand_result(&mut result, &contexts(&plan));
    assert_eq!(result.hits.len(), 2);
    assert_eq!(result.hits[0].text(), body);
    assert_eq!(result.hits[1].text(), "无关的期刊公告");
}

#[test]
fn context_deduplication_merges_only_continuations_of_the_same_original_block() {
    let (plan, body) = plan();
    let mut result = RagEvidenceResult {
        hits: vec![hit(&plan.chunks[0]), hit(&plan.chunks[1])],
        ..Default::default()
    };
    expand_result(&mut result, &contexts(&plan));
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].text(), body);
}

#[test]
fn context_invalid_layout_or_unknown_spans_preserves_matched_leaf() {
    let (plan, _) = plan();
    let mut contexts = contexts(&plan);
    contexts.values_mut().next().unwrap().text.push('x');
    let mut result = RagEvidenceResult {
        hits: vec![hit(&plan.chunks[0])],
        ..Default::default()
    };
    let leaf = result.hits[0].row.text.clone();
    expand_result(&mut result, &contexts);
    assert_eq!(result.hits[0].row.text, leaf);
    assert!(
        result.hits[0]
            .row
            .text_metadata
            .get("matchedText")
            .is_none()
    );
    let good = StoredChunkContext {
        id: "context".into(),
        text: "abc".into(),
        spans: vec![ChunkSourceSpan {
            block_id: "block-0".into(),
            start: 0,
            end: 3,
        }],
        source_locations: vec![],
    };
    assert!(
        source_context::select(
            &good,
            &[ChunkSourceSpan {
                block_id: "block-1".into(),
                start: 0,
                end: 1
            }]
        )
        .is_none()
    );
    assert!(
        source_context::select(
            &good,
            &[ChunkSourceSpan {
                block_id: "block-0".into(),
                start: 0,
                end: 4
            }]
        )
        .is_none()
    );
}

#[test]
fn expansion_budget_preserves_all_original_candidates_for_rerank() {
    let (plan, _) = plan();
    let mut available = contexts(&plan);
    let mut candidate = hit(&plan.chunks[0]);
    let count = RETRIEVAL_CONTEXT_CHARS + 1;
    let context = available.values_mut().next().unwrap();
    context.text = "x".repeat(count);
    context.spans = vec![ChunkSourceSpan {
        block_id: "block-0".into(),
        start: 0,
        end: count,
    }];
    candidate.row.text_metadata["sourceSpans"] = json!([{"blockId":"block-0","start":0,"end":1}]);
    let leaf = candidate.row.text.clone();
    let mut result = RagEvidenceResult {
        hits: vec![candidate],
        ..Default::default()
    };
    expand_result(&mut result, &available);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].text(), leaf);
    assert_eq!(result.warnings, vec!["SourceContextBudget"]);
}

#[test]
fn context_table_ranges_and_numeric_block_order_preserve_exact_unicode_text() {
    let span = |id: &str, start, end| ChunkSourceSpan {
        block_id: id.into(),
        start,
        end,
    };
    let context = StoredChunkContext {
        id: "context".into(),
        text: "表头\n数据行\n无关".into(),
        spans: vec![
            span("block-2", 0, 2),
            span("block-2", 0, 2),
            span("block-2", 5, 8),
            span("block-10", 0, 2),
        ],
        source_locations: vec![],
    };
    let selected = source_context::select(&context, &[span("block-2", 5, 8)]).unwrap();
    assert_eq!(selected.text, "表头\n数据行");
    assert_eq!(selected.spans.len(), 2);
    assert_eq!(selected.spans[1], span("block-2", 5, 8));
}
